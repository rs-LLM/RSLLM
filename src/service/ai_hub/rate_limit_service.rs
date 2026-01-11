//! 速率限制服务模块
//! 提供基于令牌桶算法的RPM/TPM速率限制功能
use crate::domain::table::ai_hub::user_level_config::UserLevelConfig;
use crate::domain::table::basic::SysUser;
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;

/// 速率限制检查结果
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateLimitCheckResult {
    pub allowed: bool,
    pub rpm_remaining: i32,
    pub tpm_remaining: i32,
    pub warning: Option<String>,
}

/// 令牌桶
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    last_refill: DateTime,
    consumed_in_current_minute: f64,
    current_minute: i64,
}

impl TokenBucket {
    fn new(capacity: f64) -> Self {
        let now = DateTime::now();
        let current_minute = now.unix_timestamp() / 60;

        TokenBucket {
            tokens: capacity,
            capacity,
            last_refill: now,
            consumed_in_current_minute: 0.0,
            current_minute,
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        let now = DateTime::now();
        let timestamp = now.unix_timestamp();
        let elapsed = (timestamp - self.last_refill.unix_timestamp()) as f64;

        let refill_rate = self.capacity / 60.0;
        let refill = elapsed * refill_rate;

        self.tokens = (self.tokens + refill).min(self.capacity);
        self.last_refill = now;

        log::debug!(
            "[Token Bucket] try_consume: tokens={}, capacity={}, tokens_before={}, refill={}, tokens_after={}",
            tokens,
            self.capacity,
            self.tokens - refill,
            refill,
            self.tokens
        );

        let current_minute = timestamp / 60;
        if current_minute != self.current_minute {
            log::debug!(
                "[Token Bucket] Minute changed from {} to {}, resetting consumed_in_current_minute",
                self.current_minute,
                current_minute
            );
            self.current_minute = current_minute;
            self.consumed_in_current_minute = 0.0;
        }

        if self.tokens >= tokens {
            self.tokens -= tokens;
            self.consumed_in_current_minute += tokens;
            log::debug!(
                "[Token Bucket] Consumed {} tokens, total consumed in current minute: {}",
                tokens,
                self.consumed_in_current_minute
            );
            true
        } else {
            log::debug!(
                "[Token Bucket] Not enough tokens: need {}, have {}",
                tokens,
                self.tokens
            );
            false
        }
    }

    fn remaining(&self) -> f64 {
        let now = DateTime::now();
        let elapsed = (now.unix_timestamp() - self.last_refill.unix_timestamp()) as f64;

        let refill_rate = self.capacity / 60.0;
        let refill = elapsed * refill_rate;

        let remaining = (self.tokens + refill).min(self.capacity);

        log::debug!(
            "[Token Bucket] remaining calculation: capacity={}, tokens={}, elapsed={}s, refill={}, remaining={}",
            self.capacity,
            self.tokens,
            elapsed,
            refill,
            remaining
        );

        remaining
    }

    fn get_consumed(&self) -> f64 {
        let now = DateTime::now();
        let new_minute = now.unix_timestamp() / 60;

        log::debug!(
            "[Token Bucket] get_consumed: current_minute={}, new_minute={}, consumed_in_current_minute={}",
            self.current_minute,
            new_minute,
            self.consumed_in_current_minute
        );

        if new_minute != self.current_minute {
            0.0
        } else {
            self.consumed_in_current_minute
        }
    }
}

/// 速率限制服务
///
/// 负责基于令牌桶算法的RPM/TPM速率限制
#[derive(Clone)]
pub struct RateLimitService {
    rpm_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    tpm_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
}

impl Default for RateLimitService {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitService {
    pub fn new() -> Self {
        RateLimitService {
            rpm_buckets: Arc::new(RwLock::new(HashMap::new())),
            tpm_buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取用户等级配置
    pub async fn get_user_level_config(&self, user_id: &str) -> ApplicationResult<UserLevelConfig> {
        let users = SysUser::select_by_map(pool!(), rbs::value! { "id": user_id }).await?;

        if users.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(user_id.to_string()),
            });
        }

        let user = &users[0];
        let user_level = user.user_level.clone().unwrap_or_else(|| "L1".to_string());

        let configs =
            UserLevelConfig::select_by_map(pool!(), rbs::value! { "level": user_level.clone() })
                .await?;

        if configs.is_empty() {
            return Err(ApplicationError::NotFound {
                message: format!("User level config not found for level: {}", user_level),
                resource: Some("user_level_config".to_string()),
                id: Some(user_level),
            });
        }

        Ok(configs[0].clone())
    }

    /// 检查用户配额
    pub async fn check_quota(&self, user_id: &str) -> ApplicationResult<RateLimitCheckResult> {
        let level_config = self.get_user_level_config(user_id).await?;

        let rpm_limit = level_config.rpm_limit as f64;
        let tpm_limit = level_config.tpm_limit as f64;

        let mut rpm_buckets = self.rpm_buckets.write().await;
        let mut tpm_buckets = self.tpm_buckets.write().await;

        let rpm_bucket = rpm_buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(rpm_limit));

        let tpm_bucket = tpm_buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(tpm_limit));

        let rpm_allowed = rpm_bucket.try_consume(1.0);
        let tpm_allowed = tpm_bucket.try_consume(1.0);

        let rpm_remaining = rpm_bucket.remaining() as i32;
        let tpm_remaining = tpm_bucket.remaining() as i32;

        let rpm_burst_threshold = level_config.rpm_limit / 2;
        let tpm_burst_threshold = level_config.tpm_limit / 2;

        let warning = if rpm_remaining < rpm_burst_threshold {
            Some("RPM usage is high".to_string())
        } else if tpm_remaining < tpm_burst_threshold {
            Some("TPM usage is high".to_string())
        } else {
            None
        };

        Ok(RateLimitCheckResult {
            allowed: rpm_allowed && tpm_allowed,
            rpm_remaining,
            tpm_remaining,
            warning,
        })
    }

    /// 检查用户配额（指定token数量）
    pub async fn check_quota_with_tokens(
        &self,
        user_id: &str,
        tokens: i32,
    ) -> ApplicationResult<RateLimitCheckResult> {
        let level_config = self.get_user_level_config(user_id).await?;

        let rpm_limit = level_config.rpm_limit as f64;
        let tpm_limit = level_config.tpm_limit as f64;

        let mut rpm_buckets = self.rpm_buckets.write().await;
        let mut tpm_buckets = self.tpm_buckets.write().await;

        let rpm_bucket = rpm_buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(rpm_limit));

        let tpm_bucket = tpm_buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(tpm_limit));

        let rpm_allowed = rpm_bucket.try_consume(1.0);
        let tpm_allowed = tpm_bucket.try_consume(tokens as f64);

        let rpm_remaining = rpm_bucket.remaining() as i32;
        let tpm_remaining = tpm_bucket.remaining() as i32;

        log::info!(
            "[Rate Limit] Check quota with tokens for user {}: tokens={}, RPM allowed={}, TPM allowed={}, RPM remaining={}, TPM remaining={}",
            user_id,
            tokens,
            rpm_allowed,
            tpm_allowed,
            rpm_remaining,
            tpm_remaining
        );

        let warning = if rpm_remaining < rpm_limit as i32 / 4 {
            Some("RPM usage is high".to_string())
        } else if tpm_remaining < tpm_limit as i32 / 4 {
            Some("TPM usage is high".to_string())
        } else {
            None
        };

        Ok(RateLimitCheckResult {
            allowed: rpm_allowed && tpm_allowed,
            rpm_remaining,
            tpm_remaining,
            warning,
        })
    }

    /// 检查用户配额（使用自定义限速标准）
    pub async fn check_quota_with_custom_limits(
        &self,
        user_id: &str,
        rpm_limit: i32,
        tpm_limit: i32,
    ) -> ApplicationResult<RateLimitCheckResult> {
        let rpm_limit_f64 = rpm_limit as f64;
        let tpm_limit_f64 = tpm_limit as f64;

        let mut rpm_buckets = self.rpm_buckets.write().await;
        let mut tpm_buckets = self.tpm_buckets.write().await;

        let rpm_bucket = rpm_buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(rpm_limit_f64));

        let tpm_bucket = tpm_buckets
            .entry(user_id.to_string())
            .or_insert_with(|| TokenBucket::new(tpm_limit_f64));

        let rpm_allowed = rpm_bucket.try_consume(1.0);
        let tpm_allowed = tpm_bucket.try_consume(1.0);

        let rpm_remaining = rpm_bucket.remaining() as i32;
        let tpm_remaining = tpm_bucket.remaining() as i32;

        let warning = if rpm_remaining < rpm_limit / 4 {
            Some("RPM usage is high".to_string())
        } else if tpm_remaining < tpm_limit / 4 {
            Some("TPM usage is high".to_string())
        } else {
            None
        };

        Ok(RateLimitCheckResult {
            allowed: rpm_allowed && tpm_allowed,
            rpm_remaining,
            tpm_remaining,
            warning,
        })
    }

    /// 重置用户速率限制
    pub async fn reset_rate_limit(&self, user_id: &str) -> ApplicationResult<()> {
        let mut rpm_buckets = self.rpm_buckets.write().await;
        let mut tpm_buckets = self.tpm_buckets.write().await;

        if let Some(bucket) = rpm_buckets.get_mut(user_id) {
            bucket.tokens = bucket.capacity;
            bucket.last_refill = DateTime::now();
        }

        if let Some(bucket) = tpm_buckets.get_mut(user_id) {
            bucket.tokens = bucket.capacity;
            bucket.last_refill = DateTime::now();
        }

        Ok(())
    }

    /// 获取用户当前速率限制状态
    pub async fn get_rate_limit_status(
        &self,
        user_id: &str,
    ) -> ApplicationResult<RateLimitCheckResult> {
        let level_config = self.get_user_level_config(user_id).await?;

        let rpm_buckets = self.rpm_buckets.read().await;
        let tpm_buckets = self.tpm_buckets.read().await;

        let rpm_bucket_exists = rpm_buckets.contains_key(user_id);
        let tpm_bucket_exists = tpm_buckets.contains_key(user_id);

        let rpm_consumed = rpm_buckets
            .get(user_id)
            .map(|bucket| bucket.get_consumed() as i32)
            .unwrap_or(0);

        let tpm_consumed = tpm_buckets
            .get(user_id)
            .map(|bucket| bucket.get_consumed() as i32)
            .unwrap_or(0);

        let rpm_remaining = level_config.rpm_limit - rpm_consumed;
        let tpm_remaining = level_config.tpm_limit - tpm_consumed;

        log::info!(
            "[Rate Limit] Get status for user {}: RPM bucket exists={}, TPM bucket exists={}, RPM consumed={}, TPM consumed={}, RPM remaining={}, TPM remaining={}",
            user_id,
            rpm_bucket_exists,
            tpm_bucket_exists,
            rpm_consumed,
            tpm_consumed,
            rpm_remaining,
            tpm_remaining
        );

        let rpm_burst_threshold = level_config.rpm_limit / 2;
        let tpm_burst_threshold = level_config.tpm_limit / 2;

        let warning = if rpm_remaining < rpm_burst_threshold {
            Some("RPM usage is high".to_string())
        } else if tpm_remaining < tpm_burst_threshold {
            Some("TPM usage is high".to_string())
        } else {
            None
        };

        Ok(RateLimitCheckResult {
            allowed: rpm_remaining > 0 && tpm_remaining > 0,
            rpm_remaining,
            tpm_remaining,
            warning,
        })
    }

    /// 清理过期的令牌桶
    pub async fn cleanup_expired_buckets(&self, ttl_seconds: i64) -> ApplicationResult<()> {
        let now = DateTime::now();
        let mut rpm_buckets = self.rpm_buckets.write().await;
        let mut tpm_buckets = self.tpm_buckets.write().await;

        rpm_buckets.retain(|_, bucket| {
            let elapsed = (now.unix_timestamp() - bucket.last_refill.unix_timestamp()).abs();
            elapsed < ttl_seconds
        });

        tpm_buckets.retain(|_, bucket| {
            let elapsed = (now.unix_timestamp() - bucket.last_refill.unix_timestamp()).abs();
            elapsed < ttl_seconds
        });

        Ok(())
    }

    /// 消耗指定数量的token（用于流式响应中追加消耗）
    pub async fn consume_tokens(&self, user_id: &str, tokens: i32) -> ApplicationResult<()> {
        let mut rpm_buckets = self.rpm_buckets.write().await;
        let mut tpm_buckets = self.tpm_buckets.write().await;

        let rpm_remaining = if let Some(bucket) = rpm_buckets.get_mut(user_id) {
            bucket.try_consume(1.0);
            bucket.remaining() as i32
        } else {
            0
        };

        let tpm_remaining = if let Some(bucket) = tpm_buckets.get_mut(user_id) {
            bucket.try_consume(tokens as f64);
            bucket.remaining() as i32
        } else {
            0
        };

        log::info!(
            "[Rate Limit] Consumed tokens for user {}: RPM consumed=1, TPM consumed={}, RPM remaining={}, TPM remaining={}",
            user_id,
            tokens,
            rpm_remaining,
            tpm_remaining
        );

        Ok(())
    }
}
