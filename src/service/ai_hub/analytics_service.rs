//! 统计分析服务模块
//! 提供用量统计、性能分析和趋势分析功能

use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::domain::table::ai_hub::billing::AiHubBilling;
use crate::error::Result;
use crate::pool;
use rbatis::rbdc::DateTime;
use std::str::FromStr;

/// 统计分析服务
///
/// 负责用量统计、性能分析和趋势分析
#[derive(Clone)]
pub struct AnalyticsService {}

impl AnalyticsService {
    /// 查询用户用量统计
    ///
    /// 根据时间范围、模型等条件统计用户用量
    pub async fn get_user_usage_stats(
        &self,
        user_id: &str,
        start_time: Option<String>,
        end_time: Option<String>,
        model_id: Option<String>,
    ) -> Result<UserUsageStats> {
        // 构建查询条件
        let mut map = rbs::value! {
            "user_id": user_id
        };

        // 添加时间范围筛选
        if let Some(start) = &start_time {
            let start_dt = DateTime::from_str(start).map_err(|e| crate::error::Error::from(format!("Invalid start_time: {}", e)))?;
            map["created_at >="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_dt.to_string())));
        }
        if let Some(end) = &end_time {
            let end_dt = DateTime::from_str(end).map_err(|e| crate::error::Error::from(format!("Invalid end_time: {}", e)))?;
            map["created_at <="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_dt.to_string())));
        }

        // 添加模型筛选
        if let Some(model) = &model_id {
            map["model_id"] = rbs::Value::String(model.to_string());
        }

        // 查询用量记录
        let logs = AiHubUsageLog::select_by_map(pool!(), map).await?;

        if logs.is_empty() {
            return Ok(UserUsageStats::default());
        }

        // 统计数据
        let total_requests = logs.len() as i64;
        let total_input_tokens: i64 = logs.iter().map(|l| l.input_tokens).sum();
        let total_output_tokens: i64 = logs.iter().map(|l| l.output_tokens).sum();
        let total_tokens: i64 = logs.iter().map(|l| l.total_tokens).sum();
        let total_cost: f64 = logs.iter().map(|l| l.total_cost).sum();
        let avg_response_time: f64 = logs.iter()
            .filter_map(|l| l.response_time_ms)
            .map(|t| t as f64)
            .sum::<f64>() / logs.iter().filter(|l| l.response_time_ms.is_some()).count() as f64;

        // 按模型分组统计
        let mut by_model = std::collections::HashMap::new();
        for log in &logs {
            let entry = by_model.entry(&log.model_id).or_insert((0, 0, 0, 0.0));
            *entry = (
                entry.0 + 1,
                entry.1 + log.input_tokens,
                entry.2 + log.output_tokens,
                entry.3 + log.total_cost,
            );
        }

        let model_stats = by_model
            .into_iter()
            .map(|(model_id, (requests, input_tokens, output_tokens, cost))| ModelStats {
                model_id: model_id.clone(),
                requests,
                input_tokens,
                output_tokens,
                cost,
            })
            .collect();

        // 按日期分组统计
        let mut by_date = std::collections::HashMap::new();
        for log in &logs {
            if let Some(created_at) = &log.created_at {
                let date_str = created_at.to_string()[..10].to_string(); // YYYY-MM-DD
                let entry = by_date.entry(date_str).or_insert((0, 0, 0, 0.0));
                *entry = (
                    entry.0 + 1,
                    entry.1 + log.input_tokens,
                    entry.2 + log.output_tokens,
                    entry.3 + log.total_cost,
                );
            }
        }

        let daily_stats = by_date
            .into_iter()
            .map(|(date, (requests, input_tokens, output_tokens, cost))| DailyStats {
                date,
                requests,
                input_tokens,
                output_tokens,
                cost,
            })
            .collect::<Vec<DailyStats>>()
            .into_iter()
            .rev() // 最新的在前
            .collect();

        Ok(UserUsageStats {
            user_id: user_id.to_string(),
            total_requests,
            total_input_tokens,
            total_output_tokens,
            total_tokens,
            total_cost,
            avg_response_time,
            model_stats,
            daily_stats,
        })
    }

    /// 查询系统整体统计
    ///
    /// 获取系统级别的用量统计和趋势
    pub async fn get_system_stats(
        &self,
        start_time: Option<String>,
        end_time: Option<String>,
    ) -> Result<SystemStats> {
        // 构建查询条件
        let mut map = rbs::value! {};

        // 添加时间范围筛选
        if let Some(start) = &start_time {
            let start_dt = DateTime::from_str(start).map_err(|e| crate::error::Error::from(format!("Invalid start_time: {}", e)))?;
            map["created_at >="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_dt.to_string())));
        }
        if let Some(end) = &end_time {
            let end_dt = DateTime::from_str(end).map_err(|e| crate::error::Error::from(format!("Invalid end_time: {}", e)))?;
            map["created_at <="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_dt.to_string())));
        }

        // 查询用量记录
        let logs = AiHubUsageLog::select_by_map(pool!(), map).await?;

        // 查询账单记录
        let bills = AiHubBilling::select_by_map(pool!(), rbs::value! {}).await?;

        // 统计数据
        let total_requests = logs.len() as i64;
        let total_tokens: i64 = logs.iter().map(|l| l.total_tokens).sum();
        let total_revenue: f64 = bills.iter()
            .filter(|b| b.payment_status == "paid")
            .map(|b| b.total_amount)
            .sum();

        // 按用户分组统计
        let mut user_count = std::collections::HashSet::new();
        let mut by_user = std::collections::HashMap::new();
        for log in &logs {
            user_count.insert(&log.user_id);
            let entry = by_user.entry(&log.user_id).or_insert((0, 0, 0.0));
            *entry = (
                entry.0 + 1,
                entry.1 + log.total_tokens,
                entry.2 + log.total_cost,
            );
        }

        let top_users = by_user
            .into_iter()
            .map(|(user_id, (requests, tokens, cost))| UserStats {
                user_id: user_id.clone(),
                requests,
                tokens,
                cost,
            })
            .collect::<Vec<UserStats>>()
            .into_iter()
            .rev() // 按成本降序
            .take(10) // 取前10名
            .collect();

        // 按模型分组统计
        let mut by_model = std::collections::HashMap::new();
        for log in &logs {
            let entry = by_model.entry(&log.model_id).or_insert((0, 0, 0.0));
            *entry = (
                entry.0 + 1,
                entry.1 + log.total_tokens,
                entry.2 + log.total_cost,
            );
        }

        let top_models = by_model
            .into_iter()
            .map(|(model_id, (requests, _tokens, cost))| ModelStats {
                model_id: model_id.clone(),
                requests,
                input_tokens: 0,
                output_tokens: 0,
                cost,
            })
            .collect::<Vec<ModelStats>>()
            .into_iter()
            .rev() // 按成本降序
            .take(10) // 取前10名
            .collect();

        Ok(SystemStats {
            total_requests,
            total_tokens,
            total_revenue,
            active_users: user_count.len() as i64,
            top_users,
            top_models,
        })
    }

    /// 查询性能趋势分析
    ///
    /// 分析系统性能指标的趋势
    pub async fn get_performance_trends(
        &self,
        _days: i64,
    ) -> Result<PerformanceTrends> {
        let now = DateTime::now();
        let start_time = now.clone();
        
        // 构建查询条件：最近N天
        let map = rbs::value! {
            "created_at >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string())))
        };

        let logs = AiHubUsageLog::select_by_map(pool!(), map).await?;

        // 按小时分组统计性能
        let mut hourly_stats = std::collections::HashMap::new();
        for log in &logs {
            if let Some(created_at) = &log.created_at {
                let hour_str = created_at.to_string()[..13].to_string(); // YYYY-MM-DD HH
                let entry = hourly_stats.entry(hour_str).or_insert((0, 0, 0));
                *entry = (
                    entry.0 + 1,
                    entry.1 + log.response_time_ms.unwrap_or(0),
                    entry.2 + if log.error_message.is_some() { 1 } else { 0 },
                );
            }
        }

        let hourly_trends = hourly_stats
            .into_iter()
            .map(|(hour, (requests, total_time, errors))| HourlyTrend {
                hour,
                requests,
                avg_response_time: if requests > 0 {
                    total_time as f64 / requests as f64
                } else {
                    0.0
                },
                error_rate: if requests > 0 {
                    errors as f64 / requests as f64 * 100.0
                } else {
                    0.0
                },
            })
            .collect::<Vec<HourlyTrend>>()
            .into_iter()
            .rev()
            .take(24) // 最近24小时
            .collect();

        // 计算成功率
        let total_requests = logs.len();
        let failed_requests = logs.iter().filter(|l| l.error_message.is_some()).count();
        let success_rate = if total_requests > 0 {
            (total_requests - failed_requests) as f64 / total_requests as f64 * 100.0
        } else {
            100.0
        };

        // 平均响应时间
        let avg_response_time = logs.iter()
            .filter_map(|l| l.response_time_ms)
            .map(|t| t as f64)
            .sum::<f64>() / logs.iter().filter(|l| l.response_time_ms.is_some()).count() as f64;

        Ok(PerformanceTrends {
            success_rate,
            avg_response_time,
            hourly_trends,
        })
    }
}

/// 用户用量统计
#[derive(Clone, Debug, Default)]
pub struct UserUsageStats {
    pub user_id: String,
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub avg_response_time: f64,
    pub model_stats: Vec<ModelStats>,
    pub daily_stats: Vec<DailyStats>,
}

/// 模型统计
#[derive(Clone, Debug)]
pub struct ModelStats {
    pub model_id: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

/// 日统计
#[derive(Clone, Debug)]
pub struct DailyStats {
    pub date: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost: f64,
}

/// 系统统计
#[derive(Clone, Debug)]
pub struct SystemStats {
    pub total_requests: i64,
    pub total_tokens: i64,
    pub total_revenue: f64,
    pub active_users: i64,
    pub top_users: Vec<UserStats>,
    pub top_models: Vec<ModelStats>,
}

/// 用户统计
#[derive(Clone, Debug)]
pub struct UserStats {
    pub user_id: String,
    pub requests: i64,
    pub tokens: i64,
    pub cost: f64,
}

/// 性能趋势
#[derive(Clone, Debug)]
pub struct PerformanceTrends {
    pub success_rate: f64,
    pub avg_response_time: f64,
    pub hourly_trends: Vec<HourlyTrend>,
}

/// 小时趋势
#[derive(Clone, Debug)]
pub struct HourlyTrend {
    pub hour: String,
    pub requests: i64,
    pub avg_response_time: f64,
    pub error_rate: f64,
}