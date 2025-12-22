//! 计费服务模块
//! 提供费用计算、配额检查和用量记录功能
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::error::{ApplicationError, ApplicationResult};
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
    ) -> ApplicationResult<CalculatedFee> {
        // 获取用户总配额信息
        let (_total_quota, used_quota, remaining_quota) = self.quota_service.get_user_quota_info(user_id).await?;
        
        if remaining_quota <= 0.0 {
            return Err(ApplicationError::QuotaExceeded {
                message: "配额不足".to_string(),
                user_id: Some(user_id.to_string()),
                required: None,
                remaining: Some(remaining_quota),
            });
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
            return Err(ApplicationError::QuotaExceeded {
                message: format!("配额不足：需要 {:.2} 分，剩余 {:.2} 分", total_cost, remaining_quota),
                user_id: Some(user_id.to_string()),
                required: Some(total_cost),
                remaining: Some(remaining_quota),
            });
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
    /// 原子操作：扣减配额并创建用量记录，任何步骤失败都会自动回滚
    pub async fn deduct_quota_and_log(
        &self,
        fee: &CalculatedFee,
        request_id: &str,
        duration_ms: i64,
        _status: &str,
        _extra: Option<serde_json::Value>,
    ) -> ApplicationResult<String> {
        // 使用事务保证原子性
        let mut tx = pool!().acquire_begin().await?;

        // 1. 扣减配额（使用事务版本）
        let deduct_dto = crate::domain::dto::DeductQuotaDTO {
            amount: fee.total_cost,
            request_id: Some(request_id.to_string()),
            remark: Some(format!("AI请求: {}", request_id)),
        };

        match self.quota_service.deduct_batch_in_tx(&mut tx, &fee.user_id, vec![deduct_dto]).await {
            Ok(_) => {
                // 2. 记录用量
                let usage_log = AiHubUsageLog {
                    id: Some(uuid::Uuid::new_v4().to_string()),
                    request_id: request_id.to_string(),
                    user_id: fee.user_id.clone(),
                    model_id: fee.model_id.clone(),
                    input_tokens: fee.input_tokens,
                    output_tokens: fee.output_tokens,
                    total_tokens: fee.total_tokens,
                    input_price: 0.0,
                    output_price: 0.0,
                    total_cost: fee.total_cost,
                    currency: Some("USD".to_string()),
                    request_method: Some("POST".to_string()),
                    request_path: Some("/api/v1/chat/completions".to_string()),
                    request_headers: None,
                    request_body: None,
                    status_code: Some(200),
                    response_time_ms: Some(duration_ms),
                    error_message: None,
                    quota_deducted: Some(true),
                    quota_snapshot: None,
                    ip_address: None,
                    user_agent: None,
                    created_at: Some(DateTime::now()),
                };

                match AiHubUsageLog::insert(&mut tx, &usage_log).await {
                    Ok(_) => {
                        tx.commit().await?;
                        log::info!("[BillingService] Deduct and log successful: user_id={}, request_id={}, amount={}",
                            fee.user_id, request_id, fee.total_cost);
                        Ok(usage_log.id.ok_or_else(|| ApplicationError::BusinessError {
                            message: "Failed to generate usage log ID".to_string(),
                            code: Some("USAGE_LOG_ID_GENERATION_FAILED".to_string()),
                            context: Some("Failed to generate usage log ID after successful insertion".to_string()),
                        })?)
                    }
                    Err(e) => {
                        // 用量记录失败，回滚配额扣减
                        tx.rollback().await?;
                        log::error!("[BillingService] Failed to insert usage log, rolling back quota deduction: user_id={}, request_id={}, error={}",
                            fee.user_id, request_id, e);
                        Err(ApplicationError::DatabaseError {
                            message: e.to_string(),
                            operation: Some("insert".to_string()),
                            table: Some("ai_hub_usage_log".to_string()),
                        })
                    }
                }
            }
            Err(e) => {
                // 配额扣减失败，回滚事务
                tx.rollback().await?;
                log::error!("[BillingService] Failed to deduct quota: user_id={}, request_id={}, error={}",
                    fee.user_id, request_id, e);
                Err(e)
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
    ) -> ApplicationResult<String> {
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
    pub async fn get_usage_log(&self, id: &str) -> ApplicationResult<AiHubUsageLogVO> {
        let logs = AiHubUsageLog::select_by_map(pool!(), rbs::value!({"id": id})).await?;
        let log = logs.into_iter().next()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "Usage log not found".to_string(),
                resource: Some("usage_log".to_string()),
                id: Some(id.to_string()),
            })?;
        Ok(self.to_usage_log_vo(log))
    }

    /// 查询用户用量记录列表
    pub async fn list_usage_logs(
        &self,
        user_id: &str,
        start_time: Option<String>,
        end_time: Option<String>,
        model_id: Option<String>,
        _request_type: Option<String>,
        _status: Option<String>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> ApplicationResult<Vec<AiHubUsageLogVO>> {
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
                let start_dt = DateTime::from_str(start).map_err(|e| ApplicationError::ValidationError {
                    message: format!("Invalid start_time: {}", e),
                    field: Some("start_time".to_string()),
                    value: Some(start.clone()),
                })?;
                if log.created_at.clone().unwrap_or(DateTime::now()) < start_dt {
                    continue;
                }
            }
            
            // 筛选结束时间
            if let Some(end) = &end_time {
                let end_dt = DateTime::from_str(end).map_err(|e| ApplicationError::ValidationError {
                    message: format!("Invalid end_time: {}", e),
                    field: Some("end_time".to_string()),
                    value: Some(end.clone()),
                })?;
                if log.created_at.clone().unwrap_or(DateTime::now()) > end_dt {
                    continue;
                }
            }
            
            // 筛选模型ID
            if let Some(model) = &model_id {
                if log.model_id != *model {
                    continue;
                }
            }
            
            // 筛选请求类型 - 由于表结构中没有request_type字段，暂时跳过此筛选
            // if let Some(req_type) = &request_type {
            //     if log.request_type != *req_type {
            //         continue;
            //     }
            // }
            
            // 筛选状态 - 由于表结构中没有status字段，暂时跳过此筛选
            // if let Some(st) = &status {
            //     if log.status != *st {
            //         continue;
            //     }
            // }
            
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
    pub async fn check_quota_warning(&self, user_id: &str) -> ApplicationResult<Option<crate::domain::vo::QuotaWarningVO>> {
        self.quota_service.check_quota_warning(user_id).await
    }

    /// 回滚预消费
    ///
    /// 当AI服务调用失败时，回滚之前预扣的配额
    pub async fn rollback_pre_consumption(&self, fee: &CalculatedFee) -> ApplicationResult<()> {
        // 使用quota_service的回滚接口
        match self.quota_service.rollback_deduct(&fee.user_id, fee.total_cost).await {
            Ok(_) => {
                log::info!("[BillingService] Pre-consumption rolled back successfully: user_id={}, amount={}",
                    fee.user_id, fee.total_cost);
                Ok(())
            }
            Err(e) => {
                log::error!("[BillingService] Failed to rollback pre-consumption: user_id={}, amount={}, error={}",
                    fee.user_id, fee.total_cost, e);
                Err(e)
            }
        }
    }

    /// 预扣减配额并记录用量（带事务支持）
    ///
    /// 用于需要在AI服务调用成功后才提交的场景
    /// 注意：此方法内部管理事务，如果AI服务调用失败，需要调用rollback_pre_consumption回滚
    pub async fn pre_deduct_quota_and_log(
        &self,
        fee: &CalculatedFee,
        request_id: &str,
        duration_ms: i64,
        _status: &str,
        _extra: Option<serde_json::Value>,
    ) -> ApplicationResult<String> {
        // 使用事务保证原子性
        let mut tx = pool!().acquire_begin().await?;

        // 1. 扣减配额
        let deduct_dto = crate::domain::dto::DeductQuotaDTO {
            amount: fee.total_cost,
            request_id: Some(request_id.to_string()),
            remark: Some(format!("AI请求预扣: {}", request_id)),
        };

        match self.quota_service.deduct_batch_in_tx(&mut tx, &fee.user_id, vec![deduct_dto]).await {
            Ok(_) => {
                // 2. 记录用量
                let usage_log = AiHubUsageLog {
                    id: Some(uuid::Uuid::new_v4().to_string()),
                    request_id: request_id.to_string(),
                    user_id: fee.user_id.clone(),
                    model_id: fee.model_id.clone(),
                    input_tokens: fee.input_tokens,
                    output_tokens: fee.output_tokens,
                    total_tokens: fee.total_tokens,
                    input_price: 0.0,
                    output_price: 0.0,
                    total_cost: fee.total_cost,
                    currency: Some("USD".to_string()),
                    request_method: Some("POST".to_string()),
                    request_path: Some("/api/v1/chat/completions".to_string()),
                    request_headers: None,
                    request_body: None,
                    status_code: Some(200),
                    response_time_ms: Some(duration_ms),
                    error_message: None,
                    quota_deducted: Some(true),
                    quota_snapshot: None,
                    ip_address: None,
                    user_agent: None,
                    created_at: Some(DateTime::now()),
                };

                match AiHubUsageLog::insert(&mut tx, &usage_log).await {
                    Ok(_) => {
                        tx.commit().await?;
                        let log_id = usage_log.id.ok_or_else(|| ApplicationError::BusinessError {
                            message: "Failed to generate usage log ID".to_string(),
                            code: Some("USAGE_LOG_ID_GENERATION_FAILED".to_string()),
                            context: Some("Failed to generate usage log ID after successful insertion".to_string()),
                        })?;
                        log::info!("[BillingService] Pre-deduct successful: user_id={}, request_id={}, amount={}",
                            fee.user_id, request_id, fee.total_cost);
                        Ok(log_id)
                    }
                    Err(e) => {
                        tx.rollback().await?;
                        log::error!("[BillingService] Failed to insert usage log, rolling back: user_id={}, request_id={}, error={}",
                            fee.user_id, request_id, e);
                        Err(ApplicationError::DatabaseError {
                            message: e.to_string(),
                            operation: Some("insert".to_string()),
                            table: Some("ai_hub_usage_log".to_string()),
                        })
                    }
                }
            }
            Err(e) => {
                tx.rollback().await?;
                log::error!("[BillingService] Failed to deduct quota: user_id={}, request_id={}, error={}",
                    fee.user_id, request_id, e);
                Err(e)
            }
        }
    }

    /// 获取用户等级
    async fn get_user_level(&self, _user_id: &str) -> ApplicationResult<String> {
        // TODO: 从用户服务获取用户等级
        // 这里简化处理，返回默认等级
        Ok("standard".to_string())
    }

    /// 转换为用量记录VO
    fn to_usage_log_vo(&self, log: AiHubUsageLog) -> AiHubUsageLogVO {
        AiHubUsageLogVO {
            id: log.id,
            request_id: Some(log.request_id),
            user_id: Some(log.user_id),
            model_id: Some(log.model_id),
            provider_id: None, // 表结构中没有此字段
            input_tokens: Some(log.input_tokens),
            output_tokens: Some(log.output_tokens),
            total_tokens: Some(log.total_tokens),
            input_cost: None, // 表结构中没有此字段
            output_cost: None, // 表结构中没有此字段
            total_cost: Some(log.total_cost),
            request_time: None, // 表结构中没有此字段
            response_time: None, // 表结构中没有此字段
            duration_ms: log.response_time_ms,
            request_type: "".to_string(), // VO要求String类型，提供默认值
            status: "".to_string(), // VO要求String类型，提供默认值
            extra: None, // 表结构中没有此字段
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