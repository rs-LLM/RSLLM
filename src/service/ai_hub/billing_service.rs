//! 计费服务模块
//! 提供费用计算、配额检查和用量记录功能
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::error::Result;
use crate::pool;
use crate::domain::vo::usage_log::AiHubUsageLogVO;
use rbatis::rbdc::DateTime;
use std::cmp::min;
use std::str::FromStr;

/// 计费服务
///
/// 负责费用计算、配额检查和用量记录
#[derive(Clone)]
pub struct BillingService {
    pub quota_service: crate::service::ai_hub::QuotaService,
    pub price_rule_service: crate::service::ai_hub::PriceRuleService,
}

impl BillingService {
    /// 计算费用并检查配额
    /// 
    /// 根据token数量、定价规则计算费用，并检查用户配额是否充足
    pub async fn calculate_and_check(
        &self,
        user_id: &str,
        model_id: &str,
        provider_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        base_input_price: f64,  // 分/1000 tokens
        base_output_price: f64, // 分/1000 tokens
        _request_type: &str,
    ) -> Result<CalculatedFee> {
        // 获取用户总配额信息
        let (_total_quota, used_quota, remaining_quota) = self.quota_service.get_user_quota_info(user_id).await?;
        
        if remaining_quota <= 0.0 {
            return Err(Error::from("配额不足"));
        }

        // 获取用户等级和总用量（用于价格规则匹配）
        let user_level = self.get_user_level(user_id).await?;
        let total_usage = used_quota;

        // 计算输入费用
        let input_calc = self.price_rule_service.calculate_price(
            crate::domain::dto::PriceCalculationDTO {
                base_price: base_input_price,
                input_tokens,
                output_tokens: 0,
                user_level: Some(user_level.clone()),
                total_usage: Some(total_usage),
                apply_rules: Some(true),
            }
        ).await?;

        // 计算输出费用
        let output_calc = self.price_rule_service.calculate_price(
            crate::domain::dto::PriceCalculationDTO {
                base_price: base_output_price,
                input_tokens: 0,
                output_tokens,
                user_level: Some(user_level.clone()),
                total_usage: Some(total_usage),
                apply_rules: Some(true),
            }
        ).await?;

        // 计算总费用（分）
        let input_cost = input_calc.total_amount;
        let output_cost = output_calc.total_amount;
        let total_cost = input_cost + output_cost;

        // 检查配额是否充足
        if remaining_quota < total_cost {
            return Err(Error::from(format!(
                "配额不足：需要 {:.2} 分，剩余 {:.2} 分", 
                total_cost, remaining_quota
            )));
        }

        Ok(CalculatedFee {
            user_id: user_id.to_string(),
            model_id: model_id.to_string(),
            provider_id: provider_id.to_string(),
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            input_cost,
            output_cost,
            total_cost,
            input_calculation: input_calc,
            output_calculation: output_calc,
            remaining_quota_before: remaining_quota,
            remaining_quota_after: remaining_quota - total_cost,
        })
    }

    /// 扣减配额并记录用量
    /// 
    /// 原子操作：扣减配额并创建用量记录
    pub async fn deduct_quota_and_log(
        &self,
        fee: &CalculatedFee,
        request_id: &str,
        duration_ms: i64,
        status: &str,
        extra: Option<serde_json::Value>,
    ) -> Result<String> {
        // 使用事务保证原子性
        let mut tx = pool!().acquire_begin().await?;

        // 1. 扣减配额
        let deduct_result = self.quota_service.deduct_batch(
            &fee.user_id,
            vec![crate::domain::dto::DeductQuotaDTO {
                amount: fee.total_cost,
                request_id: Some(request_id.to_string()),
                remark: Some(format!("AI请求: {}", request_id)),
            }],
        ).await;

        if let Err(e) = deduct_result {
            tx.rollback().await?;
            return Err(e);
        }

        // 2. 记录用量
        let usage_log = AiHubUsageLog {
            id: Some(uuid::Uuid::new_v4().to_string()),
            request_id: Some(request_id.to_string()),
            user_id: Some(fee.user_id.clone()),
            model_id: Some(fee.model_id.clone()),
            provider_id: Some(fee.provider_id.clone()),
            input_tokens: Some(fee.input_tokens),
            output_tokens: Some(fee.output_tokens),
            total_tokens: Some(fee.total_tokens),
            input_cost: Some(fee.input_cost),
            output_cost: Some(fee.output_cost),
            total_cost: Some(fee.total_cost),
            request_time: Some(DateTime::now()),
            response_time: Some(DateTime::now()),
            duration_ms: Some(duration_ms),
            request_type: request_id.to_string(),
            status: status.to_string(),
            extra,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        match AiHubUsageLog::insert(&mut tx, &usage_log).await {
            Ok(_) => {
                tx.commit().await?;
                Ok(usage_log.id.ok_or_else(|| Error::from("Failed to generate usage log ID"))?)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e.into())
            }
        }
    }

    /// 快速计费流程
    /// 
    /// 一站式完成计费、配额检查、扣减和记录
    pub async fn quick_bill(
        &self,
        user_id: &str,
        model_id: &str,
        provider_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        base_input_price: f64,
        base_output_price: f64,
        request_type: &str,
        request_id: &str,
        duration_ms: i64,
        status: &str,
        extra: Option<serde_json::Value>,
    ) -> Result<String> {
        // 1. 计算费用并检查配额
        let fee = self.calculate_and_check(
            user_id,
            model_id,
            provider_id,
            input_tokens,
            output_tokens,
            base_input_price,
            base_output_price,
            request_type,
        ).await?;

        // 2. 扣减配额并记录用量
        self.deduct_quota_and_log(&fee, request_id, duration_ms, status, extra).await
    }

    /// 查询用量记录
    pub async fn get_usage_log(&self, id: &str) -> Result<AiHubUsageLogVO> {
        let logs = AiHubUsageLog::select_by_map(pool!(), rbs::value!({"id": id})).await?;
        let log = logs.into_iter().next()
            .ok_or_else(|| Error::from("Usage log not found"))?;
        Ok(self.to_usage_log_vo(log))
    }

    /// 查询用户用量记录列表
    pub async fn list_usage_logs(
        &self,
        user_id: &str,
        start_time: Option<String>,
        end_time: Option<String>,
        model_id: Option<String>,
        request_type: Option<String>,
        status: Option<String>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<Vec<AiHubUsageLogVO>> {
        // 查询指定用户的所有用量记录
        let user_logs = AiHubUsageLog::select_by_map(
            pool!(),
            rbs::value!({"user_id": user_id})
        ).await?;
        
        // 手动筛选记录
        let mut filtered_logs: Vec<AiHubUsageLog> = Vec::new();
        
        for log in user_logs {
            // 筛选开始时间
            if let Some(start) = &start_time {
                let start_dt = DateTime::from_str(start).map_err(|e| Error::from(format!("Invalid start_time: {}", e)))?;
                if log.created_at.clone().unwrap_or(DateTime::now()) < start_dt {
                    continue;
                }
            }
            
            // 筛选结束时间
            if let Some(end) = &end_time {
                let end_dt = DateTime::from_str(end).map_err(|e| Error::from(format!("Invalid end_time: {}", e)))?;
                if log.created_at.clone().unwrap_or(DateTime::now()) > end_dt {
                    continue;
                }
            }
            
            // 筛选模型ID
            if let Some(model) = &model_id {
                if let Some(log_model) = &log.model_id {
                    if log_model != model {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            
            // 筛选请求类型
            if let Some(req_type) = &request_type {
                if log.request_type != *req_type {
                    continue;
                }
            }
            
            // 筛选状态
            if let Some(st) = &status {
                if log.status != *st {
                    continue;
                }
            }
            
            filtered_logs.push(log);
        }
        
        // 按创建时间降序排序
        filtered_logs.sort_by(|a, b| {
            let a_time = a.created_at.clone().unwrap_or(DateTime::now());
            let b_time = b.created_at.clone().unwrap_or(DateTime::now());
            b_time.cmp(&a_time) // 降序排序
        });
        
        // 处理分页
        let mut paginated_logs = filtered_logs;
        if let Some(page_num) = page {
            if let Some(page_size_num) = page_size {
                let page = page_num.max(1);
                let page_size = page_size_num.max(1);
                let start = ((page - 1) * page_size) as usize;
                let end = min(start + page_size as usize, paginated_logs.len());
                paginated_logs = paginated_logs[start..end].to_vec();
            }
        }
        
        Ok(paginated_logs.into_iter().map(|l| self.to_usage_log_vo(l)).collect())
    }

    /// 配额预警检查
    pub async fn check_quota_warning(&self, user_id: &str) -> Result<Option<crate::domain::vo::QuotaWarningVO>> {
        self.quota_service.check_quota_warning(user_id).await
    }

    /// 获取用户等级
    async fn get_user_level(&self, _user_id: &str) -> Result<String> {
        // TODO: 从用户服务获取用户等级
        // 这里简化处理，返回默认等级
        Ok("standard".to_string())
    }

    /// 转换为用量记录VO
    fn to_usage_log_vo(&self, log: AiHubUsageLog) -> AiHubUsageLogVO {
        AiHubUsageLogVO {
            id: log.id,
            request_id: log.request_id,
            user_id: log.user_id,
            model_id: log.model_id,
            provider_id: log.provider_id,
            input_tokens: log.input_tokens,
            output_tokens: log.output_tokens,
            total_tokens: log.total_tokens,
            input_cost: log.input_cost,
            output_cost: log.output_cost,
            total_cost: log.total_cost,
            request_time: log.request_time.map(|t| t.to_string()),
            response_time: log.response_time.map(|t| t.to_string()),
            duration_ms: log.duration_ms,
            request_type: log.request_type,
            status: log.status,
            extra: log.extra,
            created_at: log.created_at.map(|t| t.to_string()),
        }
    }
}

/// 计算费用结果
#[derive(Clone)]pub struct CalculatedFee {
    pub user_id: String,
    pub model_id: String,
    pub provider_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub input_calculation: crate::domain::vo::PriceCalculationVO,
    pub output_calculation: crate::domain::vo::PriceCalculationVO,
    pub remaining_quota_before: f64,
    pub remaining_quota_after: f64,
}

use crate::error::Error;