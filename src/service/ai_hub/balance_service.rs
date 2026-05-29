// 用途：导入应用结果类型
// 说明：用于统一返回结果
use crate::error::ApplicationResult;
// 用途：导入用户表
// 说明：用于查询用户信息
use crate::domain::dto::ai_hub::UserSubscriptionStatus;
use crate::domain::table::basic::SysUser;
use crate::domain::table::key_value_config::KeyValueConfig;
// 用途：导入余额VO
// 说明：用于返回响应数据
use crate::domain::vo::ai_hub::BalanceVO;
// 用途：导入交易服务
// 说明：用于处理余额变更操作
use super::transaction_service::TransactionService;
// 用途：导入交易DTO
// 说明：用于创建交易记录
use crate::domain::dto::ai_hub::{DeductDTO, RechargeDTO, SetBalanceDTO};
// 用途：导入数据库连接池
// 说明：用于获取数据库连接
use crate::pool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const BALANCE_CACHE_TTL: Duration = Duration::from_secs(15);

#[derive(Clone)]
struct CachedBalanceSnapshot {
    balance: BalanceVO,
    expires_at: Instant,
}

/// 余额管理服务
///
/// 负责用户余额的查询和管理操作
#[derive(Clone)]
pub struct BalanceService {
    transaction_service: TransactionService,
    balance_cache: Arc<RwLock<HashMap<String, CachedBalanceSnapshot>>>,
}

impl Default for BalanceService {
    fn default() -> Self {
        Self::new()
    }
}

impl BalanceService {
    pub fn new() -> Self {
        Self {
            transaction_service: TransactionService {},
            balance_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn clear_balance_cache(&self) {
        self.balance_cache.write().await.clear();
    }

    pub async fn invalidate_user_balance_cache(&self, user_id: &str) {
        self.balance_cache.write().await.remove(user_id);
    }

    /// 查询用户余额
    pub async fn get_balance(&self, user_id: &str) -> ApplicationResult<BalanceVO> {
        if let Some(cached) = self.balance_cache.read().await.get(user_id).cloned()
            && cached.expires_at > Instant::now()
        {
            return Ok(cached.balance);
        }

        let users = SysUser::select_by_map(pool!(), rbs::value! { "id": user_id }).await?;

        if users.is_empty() {
            return Err(crate::error::ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(user_id.to_string()),
            });
        }

        let user = &users[0];

        let mut conn = pool!().acquire().await?;
        let subscription_key = format!("user.subscription.{}", user_id);
        let subscription_raw = KeyValueConfig::get_value(&mut conn, &subscription_key, "").await?;
        let subscription = serde_json::from_str::<UserSubscriptionStatus>(&subscription_raw).ok();

        let (
            quota_total,
            quota_used,
            quota_remaining,
            subscription_status,
            subscription_expires_at,
            subscription_next_reset_at,
        ) = if let Some(status) = subscription {
            let total = status.quota_total.max(0.0);
            let used = status.quota_used.max(0.0);
            let remaining = (total - used).max(0.0);
            (
                total,
                used,
                remaining,
                status.status,
                Some(status.expires_at),
                status.next_quota_reset_at,
            )
        } else {
            (0.0, 0.0, 0.0, "none".to_string(), None, None)
        };

        let balance = BalanceVO {
            user_id: user.id.clone().unwrap_or_default(),
            username: user.account.clone().unwrap_or_default(),
            balance: user.balance.unwrap_or(0.0),
            subscription_quota_total: quota_total,
            subscription_quota_used: quota_used,
            subscription_quota_remaining: quota_remaining,
            subscription_status,
            subscription_expires_at,
            subscription_next_reset_at,
        };

        self.balance_cache.write().await.insert(
            user_id.to_string(),
            CachedBalanceSnapshot {
                balance: balance.clone(),
                expires_at: Instant::now() + BALANCE_CACHE_TTL,
            },
        );

        Ok(balance)
    }

    /// 充值
    pub async fn recharge(
        &self,
        dto: RechargeDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<String> {
        let user_id = dto.user_id.clone();
        let transaction_id = self.transaction_service.recharge(dto, operator_id).await?;
        self.invalidate_user_balance_cache(&user_id).await;
        Ok(transaction_id)
    }

    /// 扣减
    pub async fn deduct(&self, dto: DeductDTO) -> ApplicationResult<String> {
        let user_id = dto.user_id.clone();
        let transaction_id = self.transaction_service.deduct(dto).await?;
        self.invalidate_user_balance_cache(&user_id).await;
        Ok(transaction_id)
    }

    /// 设置余额
    pub async fn set_balance(
        &self,
        dto: SetBalanceDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<String> {
        let user_id = dto.user_id.clone();
        let transaction_id = self
            .transaction_service
            .set_balance(dto, operator_id)
            .await?;
        self.invalidate_user_balance_cache(&user_id).await;
        Ok(transaction_id)
    }

    /// 检查余额是否足够
    pub async fn check_balance(&self, user_id: &str, amount: f64) -> ApplicationResult<bool> {
        let balance = self.get_balance(user_id).await?;
        Ok(balance.balance >= amount)
    }
}
