//! 配额管理服务模块
//! 提供基于用户等级的RPM/TPM限流管理功能
use crate::domain::dto::ai_hub::user_quota::{CreateQuotaDTO, QuotaQueryDTO, UpdateQuotaDTO};
use crate::domain::table::ai_hub::user_level_config::UserLevelConfig;
use crate::domain::table::ai_hub::user_quota::AiHubUserQuota;
use crate::domain::table::basic::SysUser;
use crate::domain::vo::ai_hub::user_quota::{AiHubUserQuotaVO, QuotaOverviewVO, QuotaWarningVO};
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use crate::service::ai_hub::UserLevelService;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

/// 配额列表响应结构体
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListQuotasResponse {
    pub items: Vec<AiHubUserQuotaVO>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

/// 配额检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaCheckResult {
    pub allowed: bool,
    pub rpm_remaining: i32,
    pub tpm_remaining: i32,
    pub warning: Option<String>,
}

/// 配额管理服务
///
/// 负责基于用户等级的RPM/TPM限流管理
#[derive(Clone)]
pub struct QuotaService {}

impl QuotaService {
    /// 创建配额
    pub async fn create_quota(&self, dto: CreateQuotaDTO) -> ApplicationResult<String> {
        let cycle_start = match &dto.cycle_start {
            Some(t) => {
                Some(
                    DateTime::from_str(t).map_err(|e| ApplicationError::ValidationError {
                        message: format!("Invalid cycle_start: {}", e),
                        field: Some("cycle_start".to_string()),
                        value: Some(t.clone()),
                    })?,
                )
            }
            None => None,
        };

        let cycle_end = match &dto.cycle_end {
            Some(t) => {
                Some(
                    DateTime::from_str(t).map_err(|e| ApplicationError::ValidationError {
                        message: format!("Invalid cycle_end: {}", e),
                        field: Some("cycle_end".to_string()),
                        value: Some(t.clone()),
                    })?,
                )
            }
            None => None,
        };

        let quota = AiHubUserQuota {
            id: Some(ulid::Ulid::new().to_string()),
            user_id: dto.user_id,
            quota_period: Some(dto.quota_type),
            period_start: cycle_start,
            period_end: cycle_end,
            warning_threshold: dto.warning_threshold,
            critical_threshold: None,
            status: Some("active".to_string()),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
            last_used_at: None,
            rpm_limit: None,
            rpm_used: Some(0),
            rpm_reset_time: Some(DateTime::now()),
            tpm_limit: None,
            tpm_used: Some(0),
            tpm_reset_time: Some(DateTime::now()),
        };

        let id = quota
            .id
            .clone()
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "Failed to generate quota ID".to_string(),
                code: Some("QUOTA_ID_GENERATION_FAILED".to_string()),
                context: Some("Failed to generate quota ID after successful creation".to_string()),
            })?;
        AiHubUserQuota::insert(pool!(), &quota).await?;
        Ok(id)
    }

    /// 更新配额
    pub async fn update_quota(&self, id: &str, dto: UpdateQuotaDTO) -> ApplicationResult<()> {
        let mut quota = AiHubUserQuota::select_by_map(pool!(), rbs::value! { "id": id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "Quota not found".to_string(),
                resource: Some("quota".to_string()),
                id: Some(id.to_string()),
            })?;

        if let Some(cycle_start) = &dto.cycle_start {
            quota.period_start = Some(DateTime::from_str(cycle_start).map_err(|e| {
                ApplicationError::ValidationError {
                    message: format!("Invalid cycle_start: {}", e),
                    field: Some("cycle_start".to_string()),
                    value: Some(cycle_start.clone()),
                }
            })?);
        }
        if let Some(cycle_end) = &dto.cycle_end {
            quota.period_end = Some(DateTime::from_str(cycle_end).map_err(|e| {
                ApplicationError::ValidationError {
                    message: format!("Invalid cycle_end: {}", e),
                    field: Some("cycle_end".to_string()),
                    value: Some(cycle_end.clone()),
                }
            })?);
        }
        if let Some(status) = dto.status {
            quota.status = Some(status);
        }
        if let Some(warning_threshold) = dto.warning_threshold {
            quota.warning_threshold = Some(warning_threshold);
        }

        quota.updated_at = Some(DateTime::now());
        AiHubUserQuota::update_by_map(pool!(), &quota, rbs::value! { "id": id }).await?;
        Ok(())
    }

    /// 查询配额详情
    pub async fn get_quota(&self, id: &str) -> ApplicationResult<AiHubUserQuotaVO> {
        let quota = AiHubUserQuota::select_by_map(pool!(), rbs::value! { "id": id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "Quota not found".to_string(),
                resource: Some("quota".to_string()),
                id: Some(id.to_string()),
            })?;
        Ok(self.to_vo(quota))
    }

    /// 查询用户配额列表
    pub async fn list_quotas(&self, query: QuotaQueryDTO) -> ApplicationResult<ListQuotasResponse> {
        let mut map = rbs::value! {};

        if let Some(user_id) = query.user_id {
            map["user_id"] = rbs::Value::String(user_id);
        }
        if let Some(quota_type) = query.quota_type {
            map["quota_period"] = rbs::Value::String(quota_type);
        }
        if let Some(status) = query.status {
            map["status"] = rbs::Value::String(status);
        }

        let mut quotas = AiHubUserQuota::select_by_map(pool!(), map).await?;

        if let Some(false) = query.include_expired {
            let now = DateTime::now();
            quotas.retain(|q| {
                q.period_end.is_none() || q.period_end.as_ref().map(|e| e.ge(&now)).unwrap_or(false)
            });
        }

        quotas.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = quotas.len() as i64;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        let offset = ((page - 1) * page_size) as usize;
        let end = offset + page_size as usize;
        quotas = quotas.into_iter().skip(offset).take(end - offset).collect();

        Ok(ListQuotasResponse {
            items: quotas.into_iter().map(|q| self.to_vo(q)).collect(),
            total,
            page,
            size: page_size,
        })
    }

    /// 查询配额概览
    pub async fn get_overview(&self, user_id: &str) -> ApplicationResult<QuotaOverviewVO> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        let quota_vos: Vec<AiHubUserQuotaVO> =
            quotas.iter().map(|q| self.to_vo(q.clone())).collect();

        Ok(QuotaOverviewVO {
            user_id: user_id.to_string(),
            active_quota_count: quota_vos.len() as i32,
            quotas: quota_vos,
        })
    }

    /// 检查配额并获取警告信息
    pub async fn check_quota_warning(
        &self,
        user_id: &str,
    ) -> ApplicationResult<Option<QuotaWarningVO>> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        for quota in quotas {
            let rpm_usage_rate =
                if let (Some(limit), Some(used)) = (quota.rpm_limit, quota.rpm_used) {
                    if limit > 0 {
                        (used as f64 / limit as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

            let tpm_usage_rate =
                if let (Some(limit), Some(used)) = (quota.tpm_limit, quota.tpm_used) {
                    if limit > 0 {
                        (used as f64 / limit as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

            let max_usage_rate = rpm_usage_rate.max(tpm_usage_rate);

            if let Some(threshold) = quota.warning_threshold
                && max_usage_rate >= threshold
            {
                let warning = QuotaWarningVO {
                    user_id: user_id.to_string(),
                    quota_id: quota.id.clone().unwrap_or_default(),
                    quota_type: quota.quota_period.clone().unwrap_or_default(),
                    current_usage_rate: max_usage_rate,
                    warning_threshold: threshold,
                    warning_message: format!(
                        "配额使用率已达到 {:.1}%，RPM使用率: {:.1}%，TPM使用率: {:.1}%，阈值: {:.1}%",
                        max_usage_rate, rpm_usage_rate, tpm_usage_rate, threshold
                    ),
                };
                return Ok(Some(warning));
            }
        }

        Ok(None)
    }

    /// 检查配额是否充足（基于RPM/TPM）
    pub async fn check_quota(
        &self,
        user_id: &str,
        request_tokens: i32,
    ) -> ApplicationResult<QuotaCheckResult> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        if quotas.is_empty() {
            return Ok(QuotaCheckResult {
                allowed: false,
                rpm_remaining: 0,
                tpm_remaining: 0,
                warning: Some("No active quota found".to_string()),
            });
        }

        let quota = &quotas[0];
        let now = DateTime::now();

        let mut rpm_remaining = 0i32;
        let mut tpm_remaining = 0i32;
        let mut warning = None;

        if let (Some(rpm_limit), Some(rpm_used), Some(rpm_reset_time)) = (
            quota.rpm_limit,
            quota.rpm_used,
            quota.rpm_reset_time.clone(),
        ) {
            let now_ts = now.unix_timestamp();
            let reset_ts = rpm_reset_time.unix_timestamp();

            if now_ts >= reset_ts {
                rpm_remaining = rpm_limit;
            } else {
                rpm_remaining = (rpm_limit - rpm_used).max(0);
            }

            if rpm_remaining <= 0 {
                return Ok(QuotaCheckResult {
                    allowed: false,
                    rpm_remaining: 0,
                    tpm_remaining: 0,
                    warning: Some("RPM limit exceeded".to_string()),
                });
            }
        }

        if let (Some(tpm_limit), Some(tpm_used), Some(tpm_reset_time)) = (
            quota.tpm_limit,
            quota.tpm_used,
            quota.tpm_reset_time.clone(),
        ) {
            let now_ts = now.unix_timestamp();
            let reset_ts = tpm_reset_time.unix_timestamp();

            if now_ts >= reset_ts {
                tpm_remaining = tpm_limit;
            } else {
                tpm_remaining = (tpm_limit - tpm_used).max(0);
            }

            if tpm_remaining < request_tokens {
                return Ok(QuotaCheckResult {
                    allowed: false,
                    rpm_remaining,
                    tpm_remaining,
                    warning: Some(format!(
                        "TPM limit exceeded: required {}, remaining {}",
                        request_tokens, tpm_remaining
                    )),
                });
            }
        }

        if let Some(threshold) = quota.warning_threshold {
            let rpm_usage_rate =
                if let (Some(limit), Some(used)) = (quota.rpm_limit, quota.rpm_used) {
                    if limit > 0 {
                        (used as f64 / limit as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

            let tpm_usage_rate =
                if let (Some(limit), Some(used)) = (quota.tpm_limit, quota.tpm_used) {
                    if limit > 0 {
                        (used as f64 / limit as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

            let max_usage_rate = rpm_usage_rate.max(tpm_usage_rate);

            if max_usage_rate >= threshold {
                warning = Some(format!(
                    "Quota usage rate: {:.1}%, RPM: {:.1}%, TPM: {:.1}%",
                    max_usage_rate, rpm_usage_rate, tpm_usage_rate
                ));
            }
        }

        Ok(QuotaCheckResult {
            allowed: true,
            rpm_remaining,
            tpm_remaining,
            warning,
        })
    }

    /// 检查RPM是否超限
    pub async fn check_rpm(&self, user_id: &str) -> ApplicationResult<bool> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        if quotas.is_empty() {
            return Ok(false);
        }

        let quota = &quotas[0];
        let now = DateTime::now();

        if let (Some(rpm_limit), Some(rpm_used), Some(rpm_reset_time)) = (
            quota.rpm_limit,
            quota.rpm_used,
            quota.rpm_reset_time.clone(),
        ) {
            let now_ts = now.unix_timestamp();
            let reset_ts = rpm_reset_time.unix_timestamp();

            if now_ts >= reset_ts {
                return Ok(true);
            }

            return Ok(rpm_used < rpm_limit);
        }

        Ok(true)
    }

    /// 检查TPM是否超限
    pub async fn check_tpm(&self, user_id: &str, request_tokens: i32) -> ApplicationResult<bool> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        if quotas.is_empty() {
            return Ok(false);
        }

        let quota = &quotas[0];
        let now = DateTime::now();

        if let (Some(tpm_limit), Some(tpm_used), Some(tpm_reset_time)) = (
            quota.tpm_limit,
            quota.tpm_used,
            quota.tpm_reset_time.clone(),
        ) {
            let now_ts = now.unix_timestamp();
            let reset_ts = tpm_reset_time.unix_timestamp();

            if now_ts >= reset_ts {
                return Ok(true);
            }

            return Ok((tpm_used + request_tokens) <= tpm_limit);
        }

        Ok(true)
    }

    /// 记录配额使用（RPM和TPM）
    pub async fn record_usage(&self, user_id: &str, request_tokens: i32) -> ApplicationResult<()> {
        let mut quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        if quotas.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "No active quota found".to_string(),
                resource: Some("quota".to_string()),
                id: Some(user_id.to_string()),
            });
        }

        let mut quota = quotas.remove(0);
        let now = DateTime::now();

        if quota.rpm_reset_time.is_some() {
            let now_ts = now.unix_timestamp();
            let reset_ts = quota
                .rpm_reset_time
                .as_ref()
                .map(|dt| dt.unix_timestamp())
                .unwrap_or(0);

            if now_ts >= reset_ts {
                quota.rpm_used = Some(0);
                quota.rpm_reset_time = Some(now.clone());
            }
        }

        if let Some(rpm_used) = quota.rpm_used {
            quota.rpm_used = Some(rpm_used + 1);
        }

        if quota.tpm_reset_time.is_some() {
            let now_ts = now.unix_timestamp();
            let reset_ts = quota
                .tpm_reset_time
                .as_ref()
                .map(|dt| dt.unix_timestamp())
                .unwrap_or(0);

            if now_ts >= reset_ts {
                quota.tpm_used = Some(0);
                quota.tpm_reset_time = Some(now.clone());
            }
        }

        if let Some(tpm_used) = quota.tpm_used {
            quota.tpm_used = Some(tpm_used + request_tokens);
        }

        quota.last_used_at = Some(now.clone());
        quota.updated_at = Some(now.clone());

        AiHubUserQuota::update_by_map(
            pool!(),
            &quota,
            rbs::value! { "id": quota.id.clone().unwrap_or_default() },
        )
        .await?;

        Ok(())
    }

    /// 重置周期性配额（RPM/TPM计数器）
    pub async fn reset_periodic_quota(&self, user_id: &str) -> ApplicationResult<()> {
        let mut quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        if quotas.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "No active quota found".to_string(),
                resource: Some("quota".to_string()),
                id: Some(user_id.to_string()),
            });
        }

        let mut quota = quotas.remove(0);
        let now = DateTime::now();

        quota.rpm_used = Some(0);
        quota.rpm_reset_time = Some(now.clone());
        quota.tpm_used = Some(0);
        quota.tpm_reset_time = Some(now.clone());
        quota.updated_at = Some(now);

        AiHubUserQuota::update_by_map(
            pool!(),
            &quota,
            rbs::value! { "id": quota.id.clone().unwrap_or_default() },
        )
        .await?;

        Ok(())
    }

    /// 获取配额状态
    pub async fn get_quota_status(&self, user_id: &str) -> ApplicationResult<AiHubUserQuotaVO> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        if quotas.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "No active quota found".to_string(),
                resource: Some("quota".to_string()),
                id: Some(user_id.to_string()),
            });
        }

        Ok(self.to_vo(quotas[0].clone()))
    }

    /// 更新配额配置
    pub async fn update_quota_config(
        &self,
        user_id: &str,
        rpm_limit: Option<i32>,
        tpm_limit: Option<i32>,
    ) -> ApplicationResult<()> {
        let mut quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            },
        )
        .await?;

        if quotas.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "No active quota found".to_string(),
                resource: Some("quota".to_string()),
                id: Some(user_id.to_string()),
            });
        }

        let mut quota = quotas.remove(0);

        if let Some(rpm_limit) = rpm_limit {
            quota.rpm_limit = Some(rpm_limit);
        }

        if let Some(tpm_limit) = tpm_limit {
            quota.tpm_limit = Some(tpm_limit);
        }

        quota.updated_at = Some(DateTime::now());

        AiHubUserQuota::update_by_map(
            pool!(),
            &quota,
            rbs::value! { "id": quota.id.clone().unwrap_or_default() },
        )
        .await?;

        Ok(())
    }

    /// 获取用户等级配置
    pub async fn get_user_level_config(&self, user_id: &str) -> ApplicationResult<UserLevelConfig> {
        UserLevelService::new().init_default_levels().await?;

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
                id: Some(user_level.clone()),
            });
        }

        Ok(configs[0].clone())
    }

    /// 基于用户等级创建配额
    pub async fn create_quota_by_level(&self, user_id: &str) -> ApplicationResult<String> {
        let level_config = self.get_user_level_config(user_id).await?;

        let rpm_limit = Some(level_config.rpm_limit);
        let tpm_limit = Some(level_config.tpm_limit);

        let quota = AiHubUserQuota {
            id: Some(ulid::Ulid::new().to_string()),
            user_id: user_id.to_string(),
            quota_period: Some("daily".to_string()),
            period_start: Some(DateTime::now()),
            period_end: None,
            warning_threshold: Some(80.0),
            critical_threshold: None,
            status: Some("active".to_string()),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
            last_used_at: None,
            rpm_limit,
            rpm_used: Some(0),
            rpm_reset_time: Some(DateTime::now()),
            tpm_limit,
            tpm_used: Some(0),
            tpm_reset_time: Some(DateTime::now()),
        };

        let id = quota
            .id
            .clone()
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "Failed to generate quota ID".to_string(),
                code: Some("QUOTA_ID_GENERATION_FAILED".to_string()),
                context: Some("Failed to generate quota ID after successful creation".to_string()),
            })?;
        AiHubUserQuota::insert(pool!(), &quota).await?;
        Ok(id)
    }

    /// 将AiHubUserQuota转换为AiHubUserQuotaVO
    fn to_vo(&self, quota: AiHubUserQuota) -> AiHubUserQuotaVO {
        let need_warning = if let (Some(warning_threshold), Some(rpm_limit), Some(rpm_used)) =
            (quota.warning_threshold, quota.rpm_limit, quota.rpm_used)
        {
            if rpm_limit > 0 {
                let rpm_usage_rate = (rpm_used as f64 / rpm_limit as f64) * 100.0;
                rpm_usage_rate >= warning_threshold
            } else {
                false
            }
        } else {
            false
        };

        AiHubUserQuotaVO {
            id: quota.id,
            user_id: quota.user_id,
            quota_type: quota.quota_period.unwrap_or_default(),
            cycle_start: quota.period_start.map(|d| d.to_string()),
            cycle_end: quota.period_end.map(|d| d.to_string()),
            status: quota.status.unwrap_or_default(),
            warning_threshold: quota.warning_threshold,
            need_warning,
            created_at: quota.created_at.map(|d| d.to_string()),
            rpm_limit: quota.rpm_limit,
            rpm_used: quota.rpm_used,
            tpm_limit: quota.tpm_limit,
            tpm_used: quota.tpm_used,
        }
    }
}
