//! 配额管理服务模块
//! 提供用户配额的分配、扣减、充值和查询功能
use crate::domain::table::ai_hub::user_quota::AiHubUserQuota;
use crate::domain::dto::user_quota::{CreateQuotaDTO, UpdateQuotaDTO, RechargeQuotaDTO, DeductQuotaDTO, QuotaQueryDTO, AllocateQuotaDTO};
use crate::domain::vo::user_quota::{AiHubUserQuotaVO, QuotaOverviewVO, QuotaWarningVO};
use crate::error::Result;
use crate::pool;
use rbatis::rbdc::DateTime;
use std::str::FromStr;

/// 配额管理服务
///
/// 负责用户配额的分配、扣减、充值和查询
#[derive(Clone)]
pub struct QuotaService {}

impl QuotaService {
    /// 创建配额
    pub async fn create_quota(&self, dto: CreateQuotaDTO) -> Result<String> {
        let remaining_quota = dto.total_quota;
        
        let cycle_start = match &dto.cycle_start {
            Some(t) => Some(DateTime::from_str(t).map_err(|e| Error::from(format!("Invalid cycle_start: {}", e)))?),
            None => None,
        };
        
        let cycle_end = match &dto.cycle_end {
            Some(t) => Some(DateTime::from_str(t).map_err(|e| Error::from(format!("Invalid cycle_end: {}", e)))?),
            None => None,
        };

        let quota = AiHubUserQuota {
            id: Some(uuid::Uuid::new_v4().to_string()),
            user_id: dto.user_id,
            quota_type: dto.quota_type,
            total_quota: dto.total_quota,
            used_quota: 0.0,
            remaining_quota,
            cycle_start,
            cycle_end,
            status: "active".to_string(),
            warning_threshold: dto.warning_threshold,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        let id = quota.id.clone().ok_or_else(|| Error::from("Failed to generate quota ID"))?;
        AiHubUserQuota::insert(pool!(), &quota).await?;
        Ok(id)
    }

    /// 更新配额
    pub async fn update_quota(&self, id: &str, dto: UpdateQuotaDTO) -> Result<()> {
        // 使用select_by_map替代select_by_id
        let mut quota = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! { "id": id }
        ).await?
        .first()
        .cloned()
        .ok_or_else(|| Error::from("Quota not found"))?;

        if let Some(total_quota) = dto.total_quota {
            quota.total_quota = total_quota;
            // 重新计算剩余额度
            quota.remaining_quota = total_quota - quota.used_quota;
        }
        if let Some(used_quota) = dto.used_quota {
            quota.used_quota = used_quota;
            quota.remaining_quota = quota.total_quota - used_quota;
        }
        if let Some(remaining_quota) = dto.remaining_quota {
            quota.remaining_quota = remaining_quota;
            quota.used_quota = quota.total_quota - remaining_quota;
        }
        if let Some(cycle_start) = &dto.cycle_start {
            quota.cycle_start = Some(DateTime::from_str(cycle_start).map_err(|e| Error::from(format!("Invalid cycle_start: {}", e)))?);
        }
        if let Some(cycle_end) = &dto.cycle_end {
            quota.cycle_end = Some(DateTime::from_str(cycle_end).map_err(|e| Error::from(format!("Invalid cycle_end: {}", e)))?);
        }
        if let Some(status) = dto.status {
            quota.status = status;
        }
        if let Some(warning_threshold) = dto.warning_threshold {
            quota.warning_threshold = Some(warning_threshold);
        }

        quota.updated_at = Some(DateTime::now());
        // 使用update_by_map替代update_by_id
        AiHubUserQuota::update_by_map(
            pool!(),
            &quota,
            rbs::value! { "id": id }
        ).await?;
        Ok(())
    }

    /// 配额充值
    pub async fn recharge(&self, id: &str, dto: RechargeQuotaDTO) -> Result<()> {
        // 使用select_by_map替代select_by_id
        let mut quota = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! { "id": id }
        ).await?
        .first()
        .cloned()
        .ok_or_else(|| Error::from("Quota not found"))?;

        // 原子操作：增加总额度和剩余额度
        quota.total_quota += dto.amount;
        quota.remaining_quota += dto.amount;
        quota.updated_at = Some(DateTime::now());

        // 使用update_by_map替代update_by_id
        AiHubUserQuota::update_by_map(
            pool!(),
            &quota,
            rbs::value! { "id": id }
        ).await?;
        
        // TODO: 记录充值日志（需要创建日志表）
        Ok(())
    }

    /// 配额扣减（原子操作）
    pub async fn deduct(&self, id: &str, dto: DeductQuotaDTO) -> Result<()> {
        // 使用事务保证原子性
        let mut tx = pool!().acquire_begin().await?;
        
        // 重新查询并锁定记录
        let mut quota = match AiHubUserQuota::select_by_map(&mut tx, rbs::value! { "id": id }).await? {
            mut qs if !qs.is_empty() => qs.remove(0),
            _ => {
                tx.rollback().await?;
                return Err(Error::from("Quota not found"));
            }
        };

        // 检查配额状态
        if quota.status != "active" {
            tx.rollback().await?;
            return Err(Error::from("Quota is not active"));
        }

        // 检查余额是否充足
        if quota.remaining_quota < dto.amount {
            tx.rollback().await?;
            return Err(Error::from(format!(
                "Insufficient quota: required {}, remaining {}", 
                dto.amount, quota.remaining_quota
            )));
        }

        // 扣减配额
        quota.used_quota += dto.amount;
        quota.remaining_quota -= dto.amount;
        quota.updated_at = Some(DateTime::now());

        // 更新记录
        match AiHubUserQuota::update_by_map(&mut tx, &quota, rbs::value! { "id": id }).await {
            Ok(_) => {
                tx.commit().await?;
                Ok(())
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e.into())
            }
        }
    }

    /// 批量扣减配额
    pub async fn deduct_batch(&self, user_id: &str, dtos: Vec<DeductQuotaDTO>) -> Result<()> {
        let total_amount: f64 = dtos.iter().map(|d| d.amount).sum();
        
        // 使用事务保证原子性
        let mut tx = pool!().acquire_begin().await?;
        
        // 查询用户所有活跃配额
        // 使用select_by_map替代select_by_wrapper
        let mut quotas = AiHubUserQuota::select_by_map(
            &mut tx,
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            }
        ).await?;

        if quotas.is_empty() {
            tx.rollback().await?;
            return Err(Error::from("No active quota found"));
        }

        // 按创建时间排序，优先使用较早的配额
        quotas.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let mut remaining_to_deduct = total_amount;

        for mut quota in quotas {
            if remaining_to_deduct <= 0.0 {
                break;
            }

            if quota.remaining_quota <= 0.0 {
                continue;
            }

            // 使用手动比较替代min函数，因为f64没有实现Ord trait
            let deduct_amount = if remaining_to_deduct < quota.remaining_quota {
                remaining_to_deduct
            } else {
                quota.remaining_quota
            };
            
            quota.used_quota += deduct_amount;
            quota.remaining_quota -= deduct_amount;
            quota.updated_at = Some(DateTime::now());

            // 使用update_by_map替代update_by_id
            AiHubUserQuota::update_by_map(
                &mut tx,
                &quota,
                rbs::value! { "id": quota.id.clone().unwrap_or_default() }
            ).await?;
            
            remaining_to_deduct -= deduct_amount;
        }

        if remaining_to_deduct > 0.0 {
            tx.rollback().await?;
            return Err(Error::from(format!(
                "Insufficient total quota: required {}, remaining {}", 
                total_amount, total_amount - remaining_to_deduct
            )));
        }

        tx.commit().await?;
        Ok(())
    }

    /// 查询配额详情
    pub async fn get_quota(&self, id: &str) -> Result<AiHubUserQuotaVO> {
        // 使用select_by_map替代select_by_id
        let quota = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! { "id": id }
        ).await?
        .first()
        .cloned()
        .ok_or_else(|| Error::from("Quota not found"))?;
        Ok(self.to_vo(quota))
    }

    /// 查询用户配额列表
    pub async fn list_quotas(&self, query: QuotaQueryDTO) -> Result<Vec<AiHubUserQuotaVO>> {
        // 构建查询条件
        let mut map = rbs::value! {};

        if let Some(user_id) = query.user_id {
            map["user_id"] = rbs::Value::String(user_id);
        }
        if let Some(quota_type) = query.quota_type {
            map["quota_type"] = rbs::Value::String(quota_type);
        }
        if let Some(status) = query.status {
            map["status"] = rbs::Value::String(status);
        }

        // 使用select_by_map替代select_by_wrapper
        let mut quotas = AiHubUserQuota::select_by_map(pool!(), map).await?;

        // 手动过滤过期配额
        if let Some(false) = query.include_expired {
            let now = DateTime::now();
            quotas.retain(|q| q.cycle_end.is_none() || q.cycle_end.as_ref().map(|e| e.ge(&now)).unwrap_or(false));
        }

        // 手动排序
        quotas.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // 手动分页
        if let Some(page) = query.page {
            if let Some(page_size) = query.page_size {
                let offset = ((page - 1) * page_size) as usize;
                let end = offset + page_size as usize;
                quotas = quotas.into_iter().skip(offset).take(end - offset).collect();
            }
        }

        Ok(quotas.into_iter().map(|q| self.to_vo(q)).collect())
    }

    /// 查询配额概览
    pub async fn get_overview(&self, user_id: &str) -> Result<QuotaOverviewVO> {
        // 使用select_by_map替代select_by_wrapper
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            }
        ).await?;

        let total_quota: f64 = quotas.iter().map(|q| q.total_quota).sum();
        let used_quota: f64 = quotas.iter().map(|q| q.used_quota).sum();
        let remaining_quota: f64 = quotas.iter().map(|q| q.remaining_quota).sum();
        let overall_usage_rate = if total_quota > 0.0 {
            (used_quota / total_quota) * 100.0
        } else {
            0.0
        };

        let quota_vos: Vec<AiHubUserQuotaVO> = quotas.iter().map(|q| self.to_vo(q.clone())).collect();

        Ok(QuotaOverviewVO {
            user_id: user_id.to_string(),
            total_quota,
            used_quota,
            remaining_quota,
            overall_usage_rate,
            active_quota_count: quota_vos.len() as i32,
            quotas: quota_vos,
        })
    }

    /// 检查配额并获取警告信息
    pub async fn check_quota_warning(&self, user_id: &str) -> Result<Option<QuotaWarningVO>> {
        // 使用select_by_map替代select_by_wrapper
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            }
        ).await?;

        for quota in quotas {
            let usage_rate = if quota.total_quota > 0.0 {
                (quota.used_quota / quota.total_quota) * 100.0
            } else {
                0.0
            };

            if let Some(threshold) = quota.warning_threshold {
                if usage_rate >= threshold {
                    let warning = QuotaWarningVO {
                        user_id: user_id.to_string(),
                        quota_id: quota.id.clone().unwrap_or_default(),
                        quota_type: quota.quota_type.clone(),
                        current_usage_rate: usage_rate,
                        warning_threshold: threshold,
                        remaining_quota: quota.remaining_quota,
                        warning_message: format!(
                            "配额使用率已达到 {:.1}%，剩余额度: {:.2}，阈值: {:.1}%",
                            usage_rate, quota.remaining_quota, threshold
                        ),
                    };
                    return Ok(Some(warning));
                }
            }
        }

        Ok(None)
    }

    /// 分配配额
    pub async fn allocate_quota(&self, dto: AllocateQuotaDTO) -> Result<String> {
        // 检查是否已存在同类型配额
        let existing = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": &dto.user_id,
                "quota_type": &dto.quota_type,
                "status": "active"
            }
        ).await?;

        if !existing.is_empty() {
            if let Some(true) = dto.overwrite {
                // 保存第一个配额的ID
                let first_quota_id = existing[0].id.clone().ok_or_else(|| Error::from("Quota ID is missing"))?;
                
                // 覆盖现有配额
                for quota in existing {
                    let quota_id = quota.id.clone().ok_or_else(|| Error::from("Quota ID is missing"))?;
                    self.update_quota(
                        &quota_id,
                        UpdateQuotaDTO {
                            total_quota: Some(dto.allocate_amount),
                            used_quota: Some(0.0),
                            remaining_quota: Some(dto.allocate_amount),
                            cycle_start: None,
                            cycle_end: None,
                            status: Some("active".to_string()),
                            warning_threshold: None,
                        }
                    ).await?;
                }
                return Ok(first_quota_id);
            } else {
                return Err(Error::from("Active quota already exists for this user and type"));
            }
        }

        // 创建新配额
        self.create_quota(CreateQuotaDTO {
            user_id: dto.user_id,
            quota_type: dto.quota_type,
            total_quota: dto.allocate_amount,
            cycle_start: None,
            cycle_end: None,
            warning_threshold: None,
        }).await
    }

    /// 检查用户配额是否充足
    pub async fn check_quota_sufficient(&self, user_id: &str, amount: f64) -> Result<bool> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            }
        ).await?;

        let total_remaining: f64 = quotas.iter().map(|q| q.remaining_quota).sum();
        Ok(total_remaining >= amount)
    }

    /// 获取用户总配额信息
    pub async fn get_user_quota_info(&self, user_id: &str) -> Result<(f64, f64, f64)> {
        let quotas = AiHubUserQuota::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "active"
            }
        ).await?;

        let total_quota: f64 = quotas.iter().map(|q| q.total_quota).sum();
        let used_quota: f64 = quotas.iter().map(|q| q.used_quota).sum();
        let remaining_quota: f64 = quotas.iter().map(|q| q.remaining_quota).sum();

        Ok((total_quota, used_quota, remaining_quota))
    }

    /// 转换为VO
    fn to_vo(&self, quota: AiHubUserQuota) -> AiHubUserQuotaVO {
        let usage_rate = if quota.total_quota > 0.0 {
            (quota.used_quota / quota.total_quota) * 100.0
        } else {
            0.0
        };

        let need_warning = if let Some(threshold) = quota.warning_threshold {
            usage_rate >= threshold
        } else {
            false
        };

        AiHubUserQuotaVO {
            id: quota.id,
            user_id: quota.user_id,
            quota_type: quota.quota_type,
            total_quota: quota.total_quota,
            used_quota: quota.used_quota,
            remaining_quota: quota.remaining_quota,
            usage_rate,
            cycle_start: quota.cycle_start.map(|t| t.to_string()),
            cycle_end: quota.cycle_end.map(|t| t.to_string()),
            status: quota.status,
            warning_threshold: quota.warning_threshold,
            need_warning,
            created_at: quota.created_at.map(|t| t.to_string()),
        }
    }
}

use crate::error::Error;
