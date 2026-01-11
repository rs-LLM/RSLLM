//! 用量记录服务模块
//! 提供用量记录查询、统计和详情查看功能
use crate::domain::dto::ai_hub::usage_log::{UsageLogQueryDTO, UsageStatisticsQueryDTO};
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::domain::vo::ai_hub::usage_log::{
    AiHubUsageLogVO, CostDetailVO, UsageStatisticsVO, UsageTrendVO,
};
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use rbatis::rbdc::DateTime;
use std::collections::HashMap;
use std::str::FromStr;

/// 用量记录服务
///
/// 负责用量记录的查询、统计和详情查看
#[derive(Clone)]
pub struct UsageLogService;

impl UsageLogService {
    /// 查询用量记录详情
    ///
    /// 根据ID获取单条用量记录的详细信息
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

    /// 查询用量记录详情（带用户验证）
    ///
    /// 根据ID获取单条用量记录的详细信息，并验证用户权限
    pub async fn get_usage_log_detail(
        &self,
        id: &str,
        user_id: String,
    ) -> ApplicationResult<AiHubUsageLogVO> {
        let logs = AiHubUsageLog::select_by_map(pool!(), rbs::value!({"id": id})).await?;
        let log = logs
            .into_iter()
            .next()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "Usage log not found".to_string(),
                resource: Some("usage_log".to_string()),
                id: Some(id.to_string()),
            })?;

        if log.user_id != user_id {
            return Err(ApplicationError::AuthError {
                message: "Access denied".to_string(),
                user_id: Some(user_id),
                operation: Some("get_usage_log_detail".to_string()),
            });
        }

        Ok(self.to_usage_log_vo(log))
    }

    /// 查询用量记录列表
    ///
    /// 分页查询用户的用量记录，支持按模型、状态、时间范围筛选
    pub async fn list_usage_logs(
        &self,
        params: UsageLogQueryDTO,
    ) -> ApplicationResult<Vec<AiHubUsageLogVO>> {
        let user_id = params
            .user_id
            .ok_or_else(|| ApplicationError::ValidationError {
                message: "user_id is required".to_string(),
                field: Some("user_id".to_string()),
                value: None,
            })?;

        let mut conditions = rbs::value! {"user_id": &user_id};

        if let Some(ref model_id) = params.model_id {
            conditions["model_id"] = model_id.clone().into();
        }

        let user_logs = AiHubUsageLog::select_by_map(pool!(), conditions).await?;

        let mut filtered_logs: Vec<AiHubUsageLog> = Vec::new();

        for log in user_logs {
            if let Some(ref start) = params.start_time {
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

            if let Some(ref end) = params.end_time {
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

            filtered_logs.push(log);
        }

        filtered_logs.sort_by(|a, b| {
            let a_time = a.created_at.clone().unwrap_or(DateTime::now());
            let b_time = b.created_at.clone().unwrap_or(DateTime::now());
            b_time.cmp(&a_time)
        });

        let mut paginated_logs = filtered_logs;
        if let Some(page_num) = params.page
            && let Some(page_size_num) = params.page_size
        {
            let page = page_num.max(1);
            let page_size = page_size_num.max(1);
            let start = ((page - 1) * page_size) as usize;
            let end = std::cmp::min(start + page_size as usize, paginated_logs.len());
            paginated_logs = paginated_logs[start..end].to_vec();
        }

        Ok(paginated_logs
            .into_iter()
            .map(|l| self.to_usage_log_vo(l))
            .collect())
    }

    /// 获取用量统计
    ///
    /// 统计用户在指定时间范围内的用量数据，包括总token数、总费用、请求次数等
    pub async fn get_usage_statistics(
        &self,
        params: UsageStatisticsQueryDTO,
    ) -> ApplicationResult<UsageStatisticsVO> {
        let user_id = params
            .user_id
            .clone()
            .ok_or_else(|| ApplicationError::ValidationError {
                message: "user_id is required".to_string(),
                field: Some("user_id".to_string()),
                value: None,
            })?;
        let mut conditions = rbs::value! {"user_id": &user_id};

        if let Some(ref start) = params.start_time {
            conditions["created_at >="] = start.clone().into();
        }

        if let Some(ref end) = params.end_time {
            conditions["created_at <="] = end.clone().into();
        }

        let logs = AiHubUsageLog::select_by_map(pool!(), conditions).await?;

        let mut total_tokens: i64 = 0;
        let mut total_input_tokens: i64 = 0;
        let mut total_output_tokens: i64 = 0;
        let mut total_cost: f64 = 0.0;
        let mut total_requests: i64 = 0;
        let mut success_requests: i64 = 0;
        let mut failed_requests: i64 = 0;

        for log in logs {
            total_tokens += log.total_tokens;
            total_input_tokens += log.input_tokens;
            total_output_tokens += log.output_tokens;
            total_cost += log.total_cost;
            total_requests += 1;

            if log.status_code == Some(200) || log.status_code == Some(0) {
                success_requests += 1;
            } else {
                failed_requests += 1;
            }
        }

        Ok(UsageStatisticsVO {
            user_id,
            period: params.period.unwrap_or_else(|| "day".to_string()),
            total_requests,
            total_input_tokens,
            total_output_tokens,
            total_tokens,
            total_cost,
            success_requests,
            failed_requests,
        })
    }

    /// 获取用量统计汇总
    ///
    /// 统计指定时间范围内的用量数据，包括总请求数、总token数、总费用等
    pub async fn get_usage_summary(
        &self,
        params: UsageStatisticsQueryDTO,
    ) -> ApplicationResult<UsageStatisticsVO> {
        self.get_usage_statistics(params).await
    }

    /// 获取费用明细
    ///
    /// 按模型分组统计费用明细
    pub async fn get_cost_details(
        &self,
        params: UsageStatisticsQueryDTO,
    ) -> ApplicationResult<Vec<CostDetailVO>> {
        let user_id = params
            .user_id
            .clone()
            .ok_or_else(|| ApplicationError::ValidationError {
                message: "user_id is required".to_string(),
                field: Some("user_id".to_string()),
                value: None,
            })?;
        let mut conditions = rbs::value! {"user_id": &user_id};

        if let Some(ref start) = params.start_time {
            conditions["created_at >="] = start.clone().into();
        }

        if let Some(ref end) = params.end_time {
            conditions["created_at <="] = end.clone().into();
        }

        let logs = AiHubUsageLog::select_by_map(pool!(), conditions).await?;

        let mut model_stats: HashMap<String, (i64, i64, i64, f64, i64)> = HashMap::new();

        for log in logs {
            let entry = model_stats
                .entry(log.model_id.clone())
                .or_insert((0, 0, 0, 0.0, 0));
            entry.0 += log.total_tokens;
            entry.1 += log.input_tokens;
            entry.2 += log.output_tokens;
            entry.3 += log.total_cost;
            entry.4 += 1;
        }

        let mut cost_details: Vec<CostDetailVO> = model_stats
            .into_iter()
            .map(
                |(
                    model_id,
                    (total_tokens, input_tokens, output_tokens, total_cost, request_count),
                )| {
                    CostDetailVO {
                        model_id: model_id.clone(),
                        model_name: model_id,
                        total_cost,
                        input_cost: total_cost * (input_tokens as f64 / total_tokens as f64),
                        output_cost: total_cost * (output_tokens as f64 / total_tokens as f64),
                        total_tokens,
                        request_count,
                    }
                },
            )
            .collect();

        cost_details.sort_by(|a, b| {
            b.total_cost
                .partial_cmp(&a.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(cost_details)
    }

    /// 获取用量趋势
    ///
    /// 按时间维度统计用量趋势数据，支持按天、周、月聚合
    pub async fn get_usage_trend(
        &self,
        params: UsageStatisticsQueryDTO,
    ) -> ApplicationResult<Vec<UsageTrendVO>> {
        let user_id = params.user_id.clone();
        let mut conditions = rbs::value! {"user_id": &user_id};

        if let Some(ref start) = params.start_time {
            conditions["created_at >="] = start.clone().into();
        }

        if let Some(ref end) = params.end_time {
            conditions["created_at <="] = end.clone().into();
        }

        let logs = AiHubUsageLog::select_by_map(pool!(), conditions).await?;

        let mut daily_stats: HashMap<String, (i64, i64, i64, f64)> = HashMap::new();

        for log in logs {
            if let Some(created_at) = log.created_at {
                let date_key = created_at.format("%Y-%m-%d").to_string();
                let entry = daily_stats.entry(date_key).or_insert((0, 0, 0, 0.0));
                entry.0 += log.total_tokens;
                entry.1 += log.input_tokens;
                entry.2 += log.output_tokens;
                entry.3 += log.total_cost;
            }
        }

        let mut trend_data: Vec<UsageTrendVO> = daily_stats
            .into_iter()
            .map(|(date, (total, input, output, cost))| UsageTrendVO {
                date: Some(date),
                total_tokens: Some(total),
                input_tokens: Some(input),
                output_tokens: Some(output),
                total_cost: Some(cost),
                request_count: None,
            })
            .collect();

        trend_data.sort_by(|a, b| a.date.cmp(&b.date));

        Ok(trend_data)
    }

    /// 获取用户用量记录总数
    ///
    /// 统计用户在指定条件下的用量记录总数
    pub async fn count_usage_logs(&self, params: UsageLogQueryDTO) -> ApplicationResult<i64> {
        let user_id = params
            .user_id
            .ok_or_else(|| ApplicationError::ValidationError {
                message: "user_id is required".to_string(),
                field: Some("user_id".to_string()),
                value: None,
            })?;

        let mut conditions = rbs::value! {"user_id": &user_id};

        if let Some(ref model_id) = params.model_id {
            conditions["model_id"] = model_id.clone().into();
        }

        let user_logs = AiHubUsageLog::select_by_map(pool!(), conditions).await?;

        let mut count = 0;
        for log in user_logs {
            if let Some(ref start) = params.start_time {
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

            if let Some(ref end) = params.end_time {
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

            count += 1;
        }

        Ok(count)
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
            status_code,
            error_message: log.error_message,
            request_time: request_time_ts,
            response_time: response_time_ts,
            created_at: created_at_str,
        }
    }
}
