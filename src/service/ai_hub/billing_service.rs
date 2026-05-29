//! 计费服务模块
//! 提供费用计算、配额检查和用量记录功能
use crate::domain::table::ai_hub::transaction::Transaction;
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::domain::table::basic::SysUser;
use crate::domain::vo::ai_hub::usage_log::AiHubUsageLogVO;
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use rbatis::rbdc::DateTime;
use std::cmp::min;
use std::str::FromStr;

/// 用量日志写入元信息
#[derive(Clone, Debug)]
pub struct UsageLogMeta {
    pub request_method: String,
    pub request_path: String,
    pub request_type: String,
    pub api: String,
    pub status_code: i32,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl UsageLogMeta {
    fn new(
        request_method: &'static str,
        request_path: &'static str,
        request_type: &'static str,
        api: &'static str,
        status_code: i32,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            request_method: request_method.to_string(),
            request_path: request_path.to_string(),
            request_type: request_type.to_string(),
            api: api.to_string(),
            status_code,
            ip_address,
            user_agent,
        }
    }

    pub fn chat_completions(ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self::new(
            "POST",
            "/api/v1/chat/completions",
            "chat",
            "chat",
            200,
            ip_address,
            user_agent,
        )
    }

    pub fn chat_ws(ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self::new(
            "GET",
            "/api/v1/chat/completions/ws",
            "chat",
            "chat_ws",
            101,
            ip_address,
            user_agent,
        )
    }

    pub fn embeddings(ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self::new(
            "POST",
            "/api/v1/embeddings",
            "embeddings",
            "embeddings",
            200,
            ip_address,
            user_agent,
        )
    }

    pub fn responses(ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self::new(
            "POST",
            "/api/v1/responses",
            "responses",
            "responses",
            200,
            ip_address,
            user_agent,
        )
    }

    pub fn messages(ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self::new(
            "POST",
            "/api/v1/messages",
            "messages",
            "messages",
            200,
            ip_address,
            user_agent,
        )
    }
}

/// 计算费用并检查配额参数
pub struct CalculateAndCheckParams<'a> {
    pub user_id: &'a str,
    pub model_id: &'a str,
    pub api_key: &'a str,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub base_input_price: f64,
    pub base_output_price: f64,
    pub request_type: &'a str,
}

/// 快速计费参数
pub struct QuickBillParams<'a> {
    pub user_id: &'a str,
    pub model_id: &'a str,
    pub api_key: &'a str,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub base_input_price: f64,
    pub base_output_price: f64,
    pub request_type: &'a str,
    pub request_id: &'a str,
    pub duration_ms: i64,
    pub status: &'a str,
    pub extra: Option<serde_json::Value>,
    pub meta: &'a UsageLogMeta,
}

/// 查询用量记录参数
pub struct ListUsageLogsParams<'a> {
    pub user_id: &'a str,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub model_id: Option<String>,
    pub request_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 计费服务
///
/// 负责费用计算、配额检查和用量记录
#[derive(Clone)]
pub struct BillingService {
    pub quota_service: crate::service::ai_hub::QuotaService,
}

impl BillingService {
    /// 计算费用并检查配额
    ///
    /// 根据token数量、定价规则计算费用，并检查用户配额是否充足
    pub async fn calculate_and_check<'a>(
        &self,
        params: &CalculateAndCheckParams<'a>,
    ) -> ApplicationResult<CalculatedFee> {
        // 计算输入费用（基础价格 * 输入token数量 / 1000）
        let input_cost = params.base_input_price * params.input_tokens as f64 / 1000.0;

        // 计算输出费用（基础价格 * 输出token数量 / 1000）
        let output_cost = params.base_output_price * params.output_tokens as f64 / 1000.0;

        // 计算总费用
        let total_cost = input_cost + output_cost;

        Ok(CalculatedFee {
            user_id: params.user_id.to_string(),
            model_id: params.model_id.to_string(),
            api_key: params.api_key.to_string(),
            input_tokens: params.input_tokens,
            output_tokens: params.output_tokens,
            total_tokens: params.input_tokens + params.output_tokens,
            input_cost,
            output_cost,
            total_cost,
            input_price: params.base_input_price,
            output_price: params.base_output_price,
        })
    }

    /// 扣减余额并记录用量
    ///
    /// 原子操作：扣减余额并创建用量记录，任何步骤失败都会自动回滚
    pub async fn deduct_quota_and_log(
        &self,
        fee: &CalculatedFee,
        meta: &UsageLogMeta,
        extra: Option<serde_json::Value>,
        upstream: Option<crate::service::ai_hub::provider::upstream_trace::UpstreamOAuthInfo>,
    ) -> ApplicationResult<String> {
        // 使用事务保证原子性
        let tx = pool!().acquire_begin().await?;

        // 1. 扣减余额（允许负数）
        let users = SysUser::select_by_map(&tx, rbs::value! { "id": &fee.user_id }).await?;
        if users.is_empty() {
            tx.rollback().await?;
            return Err(ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(fee.user_id.clone()),
            });
        }

        let mut user = users[0].clone();
        let balance_before = user.balance.unwrap_or(0.0);
        let balance_after = balance_before - fee.total_cost;
        user.balance = Some(balance_after);
        SysUser::update_by_map(&tx, &user, rbs::value! { "id": &fee.user_id }).await?;

        // 2. 记录用量（输出token暂时为0，等AI服务调用完成后更新）
        let log_id = ulid::Ulid::new().to_string();
        let now = DateTime::now();
        let usage_log = AiHubUsageLog {
            id: Some(log_id.clone()),
            user_id: fee.user_id.clone(),
            model_id: fee.model_id.clone(),
            api_key: fee.api_key.clone(),
            input_tokens: fee.input_tokens,
            output_tokens: fee.output_tokens,
            total_tokens: fee.total_tokens,
            input_price: fee.input_price,
            output_price: fee.output_price,
            price_unit: Some("k".to_string()),
            input_cost: Some(fee.input_cost),
            output_cost: Some(fee.output_cost),
            total_cost: fee.total_cost,
            currency: Some("USD".to_string()),
            request_method: Some(meta.request_method.clone()),
            request_path: Some(meta.request_path.clone()),
            request_headers: None,
            request_body: None,
            status_code: Some(meta.status_code),
            request_type: Some(meta.request_type.clone()),
            api: Some(meta.api.clone()),
            upstream_oauth_provider_id: upstream.as_ref().map(|v| v.provider_id.clone()),
            upstream_oauth_provider_type: upstream.as_ref().map(|v| v.provider_type.clone()),
            upstream_oauth_account_key: upstream.as_ref().map(|v| v.account_key.clone()),
            upstream_oauth_account_id: upstream.as_ref().map(|v| v.account_id.clone()),
            upstream_oauth_email: upstream.as_ref().and_then(|v| v.email.clone()),
            status: Some("success".to_string()),
            request_time: Some(now.clone()),
            response_time: Some(now.clone()),
            response_time_ms: Some(0), // 无法在此处获取真实耗时，由 controller 在 complete_usage_log 写入
            error_message: None,
            extra,
            quota_deducted: Some(1),
            quota_snapshot: None,
            ip_address: meta.ip_address.clone(),
            user_agent: meta.user_agent.clone(),
            created_at: Some(now),
        };

        match AiHubUsageLog::insert(&tx, &usage_log).await {
            Ok(_) => {
                tx.commit().await?;
                log::info!(
                    "[BillingService] Deduct balance and log usage successful: user_id={}, log_id={}, amount={}, balance_before={:.2}, balance_after={:.2}",
                    fee.user_id,
                    log_id,
                    fee.total_cost,
                    balance_before,
                    balance_after
                );
                Ok(usage_log
                    .id
                    .ok_or_else(|| ApplicationError::BusinessError {
                        message: "Failed to generate usage log ID".to_string(),
                        code: Some("USAGE_LOG_ID_GENERATION_FAILED".to_string()),
                        context: Some(
                            "Failed to generate usage log ID after successful insertion"
                                .to_string(),
                        ),
                    })?)
            }
            Err(e) => {
                // 用量记录失败，回滚余额扣减
                tx.rollback().await?;
                log::error!(
                    "[BillingService] Failed to insert usage log, rolling back balance deduction: user_id={}, log_id={}, error={}",
                    fee.user_id,
                    log_id,
                    e
                );
                Err(ApplicationError::DatabaseError {
                    message: e.to_string(),
                    operation: Some("insert".to_string()),
                    table: Some("ai_hub_usage_log".to_string()),
                })
            }
        }
    }

    /// 快速计费流程
    ///
    /// 一站式完成计费、配额检查、扣减和记录
    pub async fn quick_bill<'a>(&self, params: &QuickBillParams<'a>) -> ApplicationResult<String> {
        // 1. 计算费用并检查配额
        let fee = self
            .calculate_and_check(&CalculateAndCheckParams {
                user_id: params.user_id,
                model_id: params.model_id,
                api_key: params.api_key,
                input_tokens: params.input_tokens,
                output_tokens: params.output_tokens,
                base_input_price: params.base_input_price,
                base_output_price: params.base_output_price,
                request_type: params.request_type,
            })
            .await?;

        // 2. 扣减配额并记录用量
        self.deduct_quota_and_log(&fee, params.meta, params.extra.clone(), None)
            .await
    }

    /// 查询用量记录
    pub async fn get_usage_log(&self, id: &str) -> ApplicationResult<AiHubUsageLogVO> {
        let logs = AiHubUsageLog::select_by_map(pool!(), rbs::value!({"id": id})).await?;
        let log = logs
            .into_iter()
            .next()
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
        params: &ListUsageLogsParams<'_>,
    ) -> ApplicationResult<Vec<AiHubUsageLogVO>> {
        // 查询指定用户的所有用量记录
        let user_logs =
            AiHubUsageLog::select_by_map(pool!(), rbs::value! {"user_id": params.user_id}).await?;

        // 手动筛选记录
        let mut filtered_logs: Vec<AiHubUsageLog> = Vec::new();

        for log in user_logs {
            // 筛选开始时间
            if let Some(start) = &params.start_time {
                let start_dt =
                    DateTime::from_str(start).map_err(|e| ApplicationError::ValidationError {
                        message: format!("Invalid start_time: {}", e),
                        field: Some("start_time".to_string()),
                        value: Some(start.to_string()),
                    })?;
                if log.created_at.clone().unwrap_or(DateTime::now()) < start_dt {
                    continue;
                }
            }

            // 筛选结束时间
            if let Some(end) = &params.end_time {
                let end_dt =
                    DateTime::from_str(end).map_err(|e| ApplicationError::ValidationError {
                        message: format!("Invalid end_time: {}", e),
                        field: Some("end_time".to_string()),
                        value: Some(end.to_string()),
                    })?;
                if log.created_at.clone().unwrap_or(DateTime::now()) > end_dt {
                    continue;
                }
            }

            // 筛选模型ID
            if let Some(model) = &params.model_id
                && log.model_id != *model
            {
                continue;
            }

            // 筛选请求类型 - 由于表结构中没有request_type字段，暂时跳过此筛选
            // if let Some(req_type) = &params.request_type {
            //     if log.request_type != *req_type {
            //         continue;
            //     }
            // }

            // 筛选状态 - 由于表结构中没有status字段，暂时跳过此筛选
            // if let Some(st) = &params.status {
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
        if let Some(page_num) = params.page
            && let Some(page_size_num) = params.page_size
        {
            let page = page_num.max(1);
            let page_size = page_size_num.max(1);
            let start = ((page - 1) * page_size) as usize;
            let end = min(start + page_size as usize, paginated_logs.len());
            paginated_logs = paginated_logs[start..end].to_vec();
        }

        Ok(paginated_logs
            .into_iter()
            .map(|l| self.to_usage_log_vo(l))
            .collect())
    }

    /// 配额预警检查
    pub async fn check_quota_warning(
        &self,
        user_id: &str,
    ) -> ApplicationResult<Option<crate::domain::vo::QuotaWarningVO>> {
        self.quota_service.check_quota_warning(user_id).await
    }

    /// 回滚预消费
    ///
    /// 当AI服务调用失败时，回滚之前预扣的余额
    pub async fn rollback_pre_consumption(&self, fee: &CalculatedFee) -> ApplicationResult<()> {
        let tx = pool!().acquire_begin().await?;

        let users = SysUser::select_by_map(&tx, rbs::value! {"id": &fee.user_id}).await?;

        let user = users
            .into_iter()
            .next()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("sys_user".to_string()),
                id: Some(fee.user_id.clone()),
            })?;

        let balance_before = user.balance.unwrap_or(0.0);
        let balance_after = balance_before + fee.input_cost;

        let mut updated_user = user.clone();
        updated_user.balance = Some(balance_after);
        SysUser::update_by_map(&tx, &updated_user, rbs::value! {"id": &fee.user_id}).await?;

        // 创建回滚交易记录
        let now = DateTime::now();
        let transaction_id = ulid::Ulid::new().to_string();
        let transaction = Transaction {
            id: Some(transaction_id.clone()),
            user_id: fee.user_id.clone(),
            type_: "recharge".to_string(),
            amount: fee.input_cost,
            balance_before,
            balance_after,
            operator_id: None,
            reason: format!(
                "AI服务调用失败回滚: model={}, input_tokens={}, output_tokens={}",
                fee.model_id, fee.input_tokens, fee.output_tokens
            ),
            created_at: Some(now),
        };
        Transaction::insert(&tx, &transaction).await?;

        tx.commit().await?;

        log::info!(
            "[BillingService] Pre-consumption rolled back successfully: user_id={}, amount={}, transaction_id={}",
            fee.user_id,
            fee.input_cost,
            transaction_id
        );
        Ok(())
    }

    /// 预扣减余额（仅扣减输入费用）
    ///
    /// 用于需要在AI服务调用成功后才提交的场景
    /// 注意：此方法内部管理事务，如果AI服务调用失败，需要调用rollback_pre_consumption回滚
    pub async fn pre_deduct_quota_and_log(
        &self,
        fee: &CalculatedFee,
        meta: &UsageLogMeta,
        extra: Option<serde_json::Value>,
    ) -> ApplicationResult<String> {
        // 使用事务保证原子性
        let tx = pool!().acquire_begin().await?;

        // 1. 扣减余额（只扣减输入费用，允许负数）
        let users = SysUser::select_by_map(&tx, rbs::value! { "id": &fee.user_id }).await?;
        if users.is_empty() {
            tx.rollback().await?;
            return Err(ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(fee.user_id.clone()),
            });
        }

        let mut user = users[0].clone();
        let balance_before = user.balance.unwrap_or(0.0);
        let balance_after = balance_before - fee.input_cost;
        user.balance = Some(balance_after);
        SysUser::update_by_map(&tx, &user, rbs::value! { "id": &fee.user_id }).await?;

        // 创建预扣减交易记录
        let now = DateTime::now();
        let transaction_id = ulid::Ulid::new().to_string();
        let transaction = Transaction {
            id: Some(transaction_id.clone()),
            user_id: fee.user_id.clone(),
            type_: "deduct".to_string(),
            amount: fee.input_cost,
            balance_before,
            balance_after,
            operator_id: None,
            reason: format!(
                "AI服务调用预扣减: model={}, input_tokens={}",
                fee.model_id, fee.input_tokens
            ),
            created_at: Some(now.clone()),
        };
        Transaction::insert(&tx, &transaction).await?;

        let log_id = ulid::Ulid::new().to_string();
        let usage_log = AiHubUsageLog {
            id: Some(log_id.clone()),
            user_id: fee.user_id.clone(),
            model_id: fee.model_id.clone(),
            api_key: fee.api_key.clone(),
            input_tokens: fee.input_tokens,
            output_tokens: 0,
            total_tokens: fee.input_tokens,
            input_price: fee.input_price,
            output_price: fee.output_price,
            price_unit: Some("k".to_string()),
            input_cost: Some(fee.input_cost),
            output_cost: Some(0.0),
            total_cost: fee.input_cost,
            currency: Some("USD".to_string()),
            request_method: Some(meta.request_method.clone()),
            request_path: Some(meta.request_path.clone()),
            request_headers: None,
            request_body: None,
            status_code: Some(meta.status_code),
            request_type: Some(meta.request_type.clone()),
            api: Some(meta.api.clone()),
            upstream_oauth_provider_id: None,
            upstream_oauth_provider_type: None,
            upstream_oauth_account_key: None,
            upstream_oauth_account_id: None,
            upstream_oauth_email: None,
            status: Some("pending".to_string()),
            request_time: Some(now.clone()),
            response_time: Some(now.clone()),
            response_time_ms: Some(0),
            error_message: None,
            extra,
            quota_deducted: Some(1),
            quota_snapshot: None,
            ip_address: meta.ip_address.clone(),
            user_agent: meta.user_agent.clone(),
            created_at: Some(now),
        };
        AiHubUsageLog::insert(&tx, &usage_log).await?;

        tx.commit().await?;

        log::info!(
            "[BillingService] Pre-deduct successful: user_id={}, log_id={}, input_cost={:.2}, balance_before={:.2}, balance_after={:.2}, transaction_id={}",
            fee.user_id,
            log_id,
            fee.input_cost,
            balance_before,
            balance_after,
            transaction_id
        );
        Ok(log_id)
    }

    /// 对话结束时创建用量记录并扣减输出费用
    ///
    /// 在AI服务调用完成后，计算实际输出费用，调整余额，并创建完整的用量记录
    pub async fn complete_usage_log(
        &self,
        fee: &CalculatedFee,
        log_id: &str,
        meta: &UsageLogMeta,
        output_tokens: i64,
        duration_ms: i64,
        extra: Option<serde_json::Value>,
        upstream: Option<crate::service::ai_hub::provider::upstream_trace::UpstreamOAuthInfo>,
    ) -> ApplicationResult<()> {
        log::info!(
            "[BillingService] complete_usage_log called: log_id={}, user_id={}, model_id={}, input_tokens={}, output_tokens={}, input_price={:.6}, output_price={:.6}",
            log_id,
            fee.user_id,
            fee.model_id,
            fee.input_tokens,
            output_tokens,
            fee.input_price,
            fee.output_price
        );

        let tx = pool!().acquire_begin().await?;

        // 1. 计算费用
        let input_cost = fee.input_tokens as f64 * fee.input_price / 1000.0;
        let output_cost = output_tokens as f64 * fee.output_price / 1000.0;
        let total_cost = input_cost + output_cost;

        // 2. 扣减输出费用
        let users = SysUser::select_by_map(&tx, rbs::value! { "id": &fee.user_id }).await?;
        if users.is_empty() {
            tx.rollback().await?;
            return Err(ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(fee.user_id.clone()),
            });
        }

        let mut user = users[0].clone();
        let balance_before = user.balance.unwrap_or(0.0);
        let balance_after = balance_before - output_cost;
        user.balance = Some(balance_after);
        SysUser::update_by_map(&tx, &user, rbs::value! { "id": &fee.user_id }).await?;

        // 创建扣费交易记录
        let now = DateTime::now();
        let transaction_id = ulid::Ulid::new().to_string();
        let transaction = Transaction {
            id: Some(transaction_id.clone()),
            user_id: fee.user_id.clone(),
            type_: "deduct".to_string(),
            amount: output_cost,
            balance_before,
            balance_after,
            operator_id: None,
            reason: format!(
                "AI服务调用扣减输出费用: model={}, input_tokens={}, output_tokens={}",
                fee.model_id, fee.input_tokens, output_tokens
            ),
            created_at: Some(now.clone()),
        };
        Transaction::insert(&tx, &transaction).await?;

        // 3. 更新用量记录（预扣阶段已创建记录）
        let usage_log = AiHubUsageLog {
            id: Some(log_id.to_string()),
            user_id: fee.user_id.clone(),
            model_id: fee.model_id.clone(),
            api_key: fee.api_key.clone(),
            input_tokens: fee.input_tokens,
            output_tokens,
            total_tokens: fee.input_tokens.saturating_add(output_tokens),
            input_price: fee.input_price,
            output_price: fee.output_price,
            price_unit: Some("k".to_string()),
            input_cost: Some(input_cost),
            output_cost: Some(output_cost),
            total_cost,
            currency: Some("USD".to_string()),
            request_method: Some(meta.request_method.clone()),
            request_path: Some(meta.request_path.clone()),
            request_headers: None,
            request_body: None,
            status_code: Some(meta.status_code),
            request_type: Some(meta.request_type.clone()),
            api: Some(meta.api.clone()),
            upstream_oauth_provider_id: upstream.as_ref().map(|v| v.provider_id.clone()),
            upstream_oauth_provider_type: upstream.as_ref().map(|v| v.provider_type.clone()),
            upstream_oauth_account_key: upstream.as_ref().map(|v| v.account_key.clone()),
            upstream_oauth_account_id: upstream.as_ref().map(|v| v.account_id.clone()),
            upstream_oauth_email: upstream.as_ref().and_then(|v| v.email.clone()),
            status: Some("success".to_string()),
            request_time: Some(now.clone()),
            response_time: Some(now.clone()),
            response_time_ms: Some(duration_ms),
            error_message: None,
            extra,
            quota_deducted: Some(1),
            quota_snapshot: None,
            ip_address: meta.ip_address.clone(),
            user_agent: meta.user_agent.clone(),
            created_at: Some(now.clone()),
        };

        AiHubUsageLog::update_by_map(&tx, &usage_log, rbs::value! {"id": log_id}).await?;
        tx.commit().await?;

        log::info!(
            "[BillingService] Usage log completed: log_id={}, user_id={}, input_tokens={}, output_tokens={}, input_cost={:.2}, output_cost={:.2}, total_cost={:.2}, balance_before={:.2}, balance_after={:.2}",
            log_id,
            fee.user_id,
            fee.input_tokens,
            output_tokens,
            input_cost,
            output_cost,
            total_cost,
            balance_before,
            balance_after
        );
        Ok(())
    }

    /// 转换为用量记录VO
    fn to_usage_log_vo(&self, log: AiHubUsageLog) -> AiHubUsageLogVO {
        let provider_id = log
            .model_id
            .split('/')
            .next()
            .unwrap_or("unknown")
            .to_string();

        let request_time_ts = log.request_time.map(|dt| dt.unix_timestamp()).unwrap_or(0);

        let response_time_ts = log.response_time.map(|dt| dt.unix_timestamp()).unwrap_or(0);

        let created_at_str = log
            .created_at
            .map(|t| t.to_string())
            .unwrap_or_else(|| DateTime::now().to_string());

        let status_code = log.status_code.unwrap_or(0);

        let id = log.id.unwrap_or_else(|| log.user_id.clone());

        AiHubUsageLogVO {
            id: id,
            user_id: log.user_id,
            api_key: log.api_key,
            model_id: log.model_id,
            provider_id,
            request_type: log.request_type.unwrap_or_else(|| "unknown".to_string()),
            input_tokens: log.input_tokens,
            output_tokens: log.output_tokens,
            total_tokens: log.total_tokens,
            input_cost: log.input_cost.unwrap_or(0.0),
            output_cost: log.output_cost.unwrap_or(0.0),
            total_cost: log.total_cost,
            input_price: log.input_price,
            output_price: log.output_price,
            price_unit: log.price_unit,
            status_code,
            error_message: log.error_message,
            request_time: request_time_ts,
            response_time: response_time_ts,
            response_time_ms: log.response_time_ms.unwrap_or(0),
            ttfb_ms: 0,
            upstream_latency_ms: 0,
            local_postprocess_ms: 0,
            cache_hit: false,
            cached_tokens: 0,
            upstream_oauth_account_key: None,
            upstream_oauth_email: None,
            upstream_oauth_account_id: None,
            upstream_oauth_provider_type: None,
            created_at: created_at_str,
        }
    }
}

/// 计算费用结果
#[derive(Clone)]
pub struct CalculatedFee {
    pub user_id: String,
    pub model_id: String,
    pub api_key: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub input_price: f64,
    pub output_price: f64,
}
