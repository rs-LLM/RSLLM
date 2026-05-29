//! 管理员统计服务模块
//! 提供管理员面板统计数据聚合和缓存功能

use crate::domain::table::ai_hub::transaction::Transaction;
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::domain::table::basic::rbac::{RbacRole, RbacUserRole};
use crate::domain::table::basic::sys_user::SysUser;
use crate::domain::vo::ai_hub::admin_stats::*;
use crate::error::Result;
use crate::pool;
use rbatis::rbdc::DateTime;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 管理员统计服务
///
/// 负责管理员面板统计数据的聚合和缓存
#[derive(Clone)]
pub struct AdminStatsService {
    /// 缓存数据
    cache: Arc<RwLock<AdminStatsCache>>,
}

/// 统计数据缓存
struct AdminStatsCache {
    /// 概览统计
    overview: Option<AdminOverviewStatsVO>,
    /// 概览统计更新时间
    overview_updated_at: Option<DateTime>,
    /// 趋势统计（按维度）
    trends: HashMap<String, AdminTrendStatsVO>,
    /// 用户统计（按维度）
    user_stats: HashMap<String, AdminUserStatsVO>,
}

impl Default for AdminStatsCache {
    fn default() -> Self {
        Self {
            overview: None,
            overview_updated_at: None,
            trends: HashMap::new(),
            user_stats: HashMap::new(),
        }
    }
}

impl AdminStatsService {
    const TOP_N: usize = 10;

    /// 创建新的统计服务实例
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(AdminStatsCache::default())),
        }
    }

    /// 获取概览统计（带缓存）
    ///
    /// 缓存有效期为1小时
    pub async fn get_overview_stats(&self) -> Result<AdminOverviewStatsVO> {
        let mut cache = self.cache.write().await;

        // 检查缓存是否有效（1小时内）
        let now = DateTime::now();
        if let Some(updated_at) = &cache.overview_updated_at {
            let elapsed_seconds = now.unix_timestamp() - updated_at.unix_timestamp();
            if elapsed_seconds < 3600 {
                if let Some(stats) = &cache.overview {
                    return Ok(stats.clone());
                }
            }
        }

        // 缓存过期或不存在，重新计算
        let stats = self.calculate_overview_stats().await?;
        cache.overview = Some(stats.clone());
        cache.overview_updated_at = Some(DateTime::now());

        Ok(stats)
    }

    /// 获取趋势统计（带缓存）
    ///
    /// 根据时间维度获取趋势数据，缓存有效期为1小时
    pub async fn get_trend_stats(
        &self,
        dimension: TimeDimension,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<AdminTrendStatsVO> {
        let key = format!("{:?}-{:?}-{:?}", dimension, start_date, end_date);
        let mut cache = self.cache.write().await;

        // 检查缓存是否有效
        if let Some(stats) = cache.trends.get(&key) {
            let updated_at = DateTime::from_str(&stats.updated_at)
                .map_err(|e| crate::error::Error::from(format!("Invalid timestamp: {}", e)))?;
            let now = DateTime::now();
            let elapsed_seconds = now.unix_timestamp() - updated_at.unix_timestamp();
            if elapsed_seconds < 3600 {
                return Ok(stats.clone());
            }
        }

        // 缓存过期或不存在，重新计算
        let stats = self
            .calculate_trend_stats(
                dimension.clone(),
                start_date.as_deref(),
                end_date.as_deref(),
            )
            .await?;
        cache.trends.insert(key, stats.clone());

        Ok(stats)
    }

    /// 获取用户统计（带缓存）
    ///
    /// 根据时间维度获取用户统计数据，缓存有效期为1小时
    pub async fn get_user_stats(
        &self,
        dimension: TimeDimension,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<AdminUserStatsVO> {
        let key = format!("{:?}-{:?}-{:?}", dimension, start_date, end_date);
        let mut cache = self.cache.write().await;

        // 检查缓存是否有效
        if let Some(stats) = cache.user_stats.get(&key) {
            let updated_at = DateTime::from_str(&stats.updated_at)
                .map_err(|e| crate::error::Error::from(format!("Invalid timestamp: {}", e)))?;
            let now = DateTime::now();
            let elapsed_seconds = now.unix_timestamp() - updated_at.unix_timestamp();
            if elapsed_seconds < 3600 {
                return Ok(stats.clone());
            }
        }

        // 缓存过期或不存在，重新计算
        let stats = self
            .calculate_user_stats(
                dimension.clone(),
                start_date.as_deref(),
                end_date.as_deref(),
            )
            .await?;
        cache.user_stats.insert(key, stats.clone());

        Ok(stats)
    }

    /// 清空缓存
    ///
    /// 强制清空所有缓存数据
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.overview = None;
        cache.overview_updated_at = None;
        cache.trends.clear();
        cache.user_stats.clear();
    }

    async fn calculate_overview_stats(&self) -> Result<AdminOverviewStatsVO> {
        let users = SysUser::select_all(pool!()).await?;
        let total_users = users.len() as i64;

        let roles = RbacRole::select_all(pool!()).await?;
        let user_roles = RbacUserRole::select_all(pool!()).await?;

        let admin_role_id = roles
            .iter()
            .find(|r| {
                r.name
                    .as_deref()
                    .map_or(false, |n| n.to_lowercase() == "admin")
            })
            .and_then(|r| r.id.clone());

        let admin_count = if let Some(admin_id) = admin_role_id {
            user_roles
                .iter()
                .filter(|ur| ur.role_id.as_deref() == Some(&admin_id))
                .filter(|ur| {
                    users
                        .iter()
                        .find(|u| u.id.as_deref() == ur.user_id.as_deref())
                        .map_or(false, |u| u.state == Some(1))
                })
                .count() as i64
        } else {
            0
        };

        let transactions = Transaction::select_all(pool!()).await?;
        let total_revenue: f64 = transactions
            .iter()
            .filter(|t| t.type_ == "recharge")
            .map(|t| t.amount)
            .sum();

        let usage_logs = AiHubUsageLog::select_all(pool!()).await?;
        let total_input_tokens: i64 = usage_logs.iter().map(|l| l.input_tokens).sum();
        let total_output_tokens: i64 = usage_logs.iter().map(|l| l.output_tokens).sum();
        let total_tokens: i64 = usage_logs.iter().map(|l| l.total_tokens).sum();
        let total_consumption: f64 = usage_logs.iter().map(|l| l.total_cost).sum();

        let total_requests = usage_logs.len() as i64;
        let successful_requests = usage_logs
            .iter()
            .filter(|l| l.status.as_deref() == Some("success"))
            .count() as i64;
        let failed_requests = total_requests - successful_requests;
        let success_rate = Self::calc_success_rate(successful_requests, total_requests);

        let response_times: Vec<i64> = usage_logs
            .iter()
            .filter_map(|l| l.response_time_ms)
            .collect();
        let avg_response_time_ms = if !response_times.is_empty() {
            response_times.iter().sum::<i64>() as f64 / response_times.len() as f64
        } else {
            0.0
        };

        let model_summary = self.aggregate_by_model(&usage_logs);
        let api_key_summary = self.aggregate_by_api_key(&usage_logs);
        let error_summary = self.aggregate_errors(&usage_logs);

        Ok(AdminOverviewStatsVO {
            total_users,
            admin_count,
            total_revenue,
            total_input_tokens,
            total_output_tokens,
            total_tokens,
            total_consumption,
            updated_at: DateTime::now().to_string(),
            total_requests,
            successful_requests,
            failed_requests,
            success_rate,
            avg_response_time_ms,
            model_summary,
            api_key_summary,
            error_summary,
        })
    }

    async fn calculate_trend_stats(
        &self,
        dimension: TimeDimension,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<AdminTrendStatsVO> {
        let (start_time, end_time) = self.resolve_time_range(&dimension, start_date, end_date)?;

        let map = rbs::value! {
            "created_at >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string()))),
            "created_at <=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string()))),
        };
        let usage_logs = AiHubUsageLog::select_by_map(pool!(), map).await?;

        let mut revenue_by_time: HashMap<String, f64> = HashMap::new();
        let mut tokens_by_time: HashMap<String, f64> = HashMap::new();
        let mut requests_by_time: HashMap<String, f64> = HashMap::new();
        let mut success_by_time: HashMap<String, f64> = HashMap::new();
        let mut failure_by_time: HashMap<String, f64> = HashMap::new();
        let mut response_time_by_time: HashMap<String, (i64, i64)> = HashMap::new();

        for log in &usage_logs {
            if let Some(created_at) = &log.created_at {
                let time_key = self.get_time_key(created_at, &dimension);

                *revenue_by_time.entry(time_key.clone()).or_insert(0.0) += log.total_cost;
                *tokens_by_time.entry(time_key.clone()).or_insert(0.0) += log.total_tokens as f64;
                *requests_by_time.entry(time_key.clone()).or_insert(0.0) += 1.0;

                if log.status.as_deref() == Some("success") {
                    *success_by_time.entry(time_key.clone()).or_insert(0.0) += 1.0;
                } else {
                    *failure_by_time.entry(time_key.clone()).or_insert(0.0) += 1.0;
                }

                if let Some(rt) = log.response_time_ms {
                    let entry = response_time_by_time.entry(time_key).or_insert((0, 0));
                    entry.0 += rt;
                    entry.1 += 1;
                }
            }
        }

        let revenue_trend = self.to_trend_data_points(&revenue_by_time);
        let token_trend = self.to_trend_data_points(&tokens_by_time);
        let request_trend = self.to_trend_data_points(&requests_by_time);
        let success_trend = self.to_trend_data_points(&success_by_time);
        let failure_trend = self.to_trend_data_points(&failure_by_time);

        let mut success_rate_by_time: HashMap<String, f64> = HashMap::new();
        for (time_key, requests) in &requests_by_time {
            let success = success_by_time.get(time_key).unwrap_or(&0.0);
            let rate = if *requests > 0.0 {
                (*success / *requests) * 100.0
            } else {
                0.0
            };
            success_rate_by_time.insert(time_key.clone(), rate);
        }
        let success_rate_trend = self.to_trend_data_points(&success_rate_by_time);

        let mut avg_response_time_by_time: HashMap<String, f64> = HashMap::new();
        for (time_key, (sum, count)) in &response_time_by_time {
            let avg = if *count > 0 {
                *sum as f64 / *count as f64
            } else {
                0.0
            };
            avg_response_time_by_time.insert(time_key.clone(), avg);
        }
        let avg_response_time_trend = self.to_trend_data_points(&avg_response_time_by_time);

        Ok(AdminTrendStatsVO {
            dimension,
            revenue_trend,
            token_trend,
            request_trend,
            updated_at: DateTime::now().to_string(),
            success_trend,
            failure_trend,
            success_rate_trend,
            avg_response_time_trend,
        })
    }

    async fn calculate_user_stats(
        &self,
        dimension: TimeDimension,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<AdminUserStatsVO> {
        let (start_time, end_time) = self.resolve_time_range(&dimension, start_date, end_date)?;

        let map = rbs::value! {
            "create_date >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string()))),
            "create_date <=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string()))),
        };
        let new_users = SysUser::select_by_map(pool!(), map).await?;
        let new_users_count = new_users.len() as i64;

        let usage_map = rbs::value! {
            "created_at >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string()))),
            "created_at <=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string()))),
        };
        let usage_logs = AiHubUsageLog::select_by_map(pool!(), usage_map).await?;
        let active_users: std::collections::HashSet<_> =
            usage_logs.iter().map(|l| &l.user_id).collect();
        let active_users_count = active_users.len() as i64;

        let mut users_by_time: HashMap<String, f64> = HashMap::new();
        for user in &new_users {
            if let Some(create_date) = &user.create_date {
                let time_key = self.get_time_key(create_date, &dimension);
                *users_by_time.entry(time_key).or_insert(0.0) += 1.0;
            }
        }
        let user_growth_trend = self.to_trend_data_points(&users_by_time);

        let total_requests = usage_logs.len() as i64;
        let successful_requests = usage_logs
            .iter()
            .filter(|l| l.status.as_deref() == Some("success"))
            .count() as i64;
        let failed_requests = total_requests - successful_requests;
        let success_rate = if total_requests > 0 {
            (successful_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let response_times: Vec<i64> = usage_logs
            .iter()
            .filter_map(|l| l.response_time_ms)
            .collect();
        let avg_response_time_ms = if !response_times.is_empty() {
            response_times.iter().sum::<i64>() as f64 / response_times.len() as f64
        } else {
            0.0
        };

        let total_consumption: f64 = usage_logs.iter().map(|l| l.total_cost).sum();
        let top_consumers = self.aggregate_top_consumers(&usage_logs, Self::TOP_N);

        Ok(AdminUserStatsVO {
            dimension,
            new_users: new_users_count,
            active_users: active_users_count,
            user_growth_trend,
            updated_at: DateTime::now().to_string(),
            total_requests,
            successful_requests,
            failed_requests,
            success_rate,
            avg_response_time_ms,
            total_consumption,
            top_consumers,
        })
    }

    /// 获取时间范围
    fn get_time_range(&self, dimension: &TimeDimension) -> Result<(DateTime, DateTime)> {
        let now = DateTime::now();

        let start_time = match dimension {
            TimeDimension::Day => {
                // 7天前（一周七天）
                let start_timestamp = now.unix_timestamp() as i64 - (7 * 24 * 60 * 60);
                DateTime::from_timestamp(start_timestamp)
            }
            TimeDimension::Week => {
                // 28天前（4周）
                let start_timestamp = now.unix_timestamp() as i64 - (28 * 24 * 60 * 60);
                DateTime::from_timestamp(start_timestamp)
            }
            TimeDimension::Month => {
                // 90天前（3个月）
                let start_timestamp = now.unix_timestamp() as i64 - (90 * 24 * 60 * 60);
                DateTime::from_timestamp(start_timestamp)
            }
        };

        Ok((start_time, now))
    }

    fn resolve_time_range(
        &self,
        dimension: &TimeDimension,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<(DateTime, DateTime)> {
        match (start_date, end_date) {
            (Some(start), Some(end)) => {
                let start = DateTime::from_str(&format!("{} 00:00:00", start)).map_err(|e| {
                    crate::error::Error::from(format!("Invalid start_date '{}': {}", start, e))
                })?;
                let end = DateTime::from_str(&format!("{} 23:59:59", end)).map_err(|e| {
                    crate::error::Error::from(format!("Invalid end_date '{}': {}", end, e))
                })?;
                Ok((start, end))
            }
            _ => self.get_time_range(dimension),
        }
    }

    /// 获取时间键
    fn get_time_key(&self, datetime: &DateTime, dimension: &TimeDimension) -> String {
        let dt_str = datetime.to_string();
        match dimension {
            TimeDimension::Day => dt_str[..10].to_string(), // YYYY-MM-DD
            TimeDimension::Week => {
                // 按周分组（取日期）
                dt_str[..10].to_string()
            }
            TimeDimension::Month => dt_str[..7].to_string(), // YYYY-MM
        }
    }

    /// 转换为趋势数据点
    fn to_trend_data_points(&self, data: &HashMap<String, f64>) -> Vec<TrendDataPointVO> {
        let mut points: Vec<TrendDataPointVO> = data
            .iter()
            .map(|(label, value)| TrendDataPointVO {
                label: label.clone(),
                value: *value,
            })
            .collect();

        points.sort_by(|a, b| a.label.cmp(&b.label));
        points
    }

    fn calc_success_rate(successful_requests: i64, total_requests: i64) -> f64 {
        if total_requests > 0 {
            (successful_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        }
    }

    fn aggregate_by_model(&self, usage_logs: &[AiHubUsageLog]) -> Vec<ModelAggregationVO> {
        let mut model_stats: HashMap<String, (i64, i64, i64, i64, i64, f64, i64, i64)> =
            HashMap::new();

        for log in usage_logs {
            let entry = model_stats
                .entry(log.model_id.clone())
                .or_insert((0, 0, 0, 0, 0, 0.0, 0, 0));
            entry.0 += 1;
            if log.status.as_deref() == Some("success") {
                entry.1 += 1;
            } else {
                entry.2 += 1;
            }
            entry.3 += log.input_tokens;
            entry.4 += log.output_tokens;
            entry.5 += log.total_cost;
            if let Some(rt) = log.response_time_ms {
                entry.6 += rt;
                entry.7 += 1;
            }
        }

        let mut result: Vec<ModelAggregationVO> = model_stats
            .into_iter()
            .map(|(model_id, stats)| {
                let success_rate = if stats.0 > 0 {
                    (stats.1 as f64 / stats.0 as f64) * 100.0
                } else {
                    0.0
                };
                let avg_response_time_ms = if stats.7 > 0 {
                    stats.6 as f64 / stats.7 as f64
                } else {
                    0.0
                };
                ModelAggregationVO {
                    model_id,
                    model_name: None,
                    request_count: stats.0,
                    successful_count: stats.1,
                    failed_count: stats.2,
                    success_rate,
                    total_input_tokens: stats.3,
                    total_output_tokens: stats.4,
                    total_cost: stats.5,
                    avg_response_time_ms,
                }
            })
            .collect();

        result.sort_by(|a, b| {
            b.request_count
                .cmp(&a.request_count)
                .then_with(|| a.model_id.cmp(&b.model_id))
        });
        result.truncate(20);
        result
    }

    fn aggregate_by_api_key(&self, usage_logs: &[AiHubUsageLog]) -> Vec<ApiKeyAggregationVO> {
        let mut key_stats: HashMap<String, (i64, i64, i64, f64, i64, i64)> = HashMap::new();

        for log in usage_logs {
            let masked = self.mask_api_key(&log.api_key);
            let entry = key_stats.entry(masked).or_insert((0, 0, 0, 0.0, 0, 0));
            entry.0 += 1;
            if log.status.as_deref() == Some("success") {
                entry.1 += 1;
            } else {
                entry.2 += 1;
            }
            entry.3 += log.total_cost;
            if let Some(rt) = log.response_time_ms {
                entry.4 += rt;
                entry.5 += 1;
            }
        }

        let mut result: Vec<ApiKeyAggregationVO> = key_stats
            .into_iter()
            .map(|(api_key_masked, stats)| {
                let success_rate = if stats.0 > 0 {
                    (stats.1 as f64 / stats.0 as f64) * 100.0
                } else {
                    0.0
                };
                let avg_response_time_ms = if stats.5 > 0 {
                    stats.4 as f64 / stats.5 as f64
                } else {
                    0.0
                };
                ApiKeyAggregationVO {
                    api_key_masked,
                    request_count: stats.0,
                    successful_count: stats.1,
                    failed_count: stats.2,
                    success_rate,
                    total_cost: stats.3,
                    avg_response_time_ms,
                }
            })
            .collect();

        result.sort_by(|a, b| {
            b.request_count
                .cmp(&a.request_count)
                .then_with(|| a.api_key_masked.cmp(&b.api_key_masked))
        });
        result.truncate(20);
        result
    }

    fn aggregate_errors(&self, usage_logs: &[AiHubUsageLog]) -> Vec<ErrorSummaryVO> {
        let mut error_stats: HashMap<String, (i64, Option<String>)> = HashMap::new();
        let total_errors: i64 = usage_logs
            .iter()
            .filter(|l| l.status.as_deref() != Some("success"))
            .count() as i64;

        for log in usage_logs {
            if log.status.as_deref() != Some("success") {
                let error_type = log
                    .status_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let entry = error_stats.entry(error_type).or_insert((0, None));
                entry.0 += 1;
                if entry.1.is_none() && log.error_message.is_some() {
                    entry.1 = log.error_message.clone();
                }
            }
        }

        let mut result: Vec<ErrorSummaryVO> = error_stats
            .into_iter()
            .map(|(error_type, (count, sample))| {
                let percentage = if total_errors > 0 {
                    (count as f64 / total_errors as f64) * 100.0
                } else {
                    0.0
                };
                ErrorSummaryVO {
                    error_type,
                    count,
                    percentage,
                    sample_message: sample,
                }
            })
            .collect();

        result.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.error_type.cmp(&b.error_type))
        });
        result.truncate(10);
        result
    }

    fn aggregate_top_consumers(
        &self,
        usage_logs: &[AiHubUsageLog],
        top_n: usize,
    ) -> Vec<UserConsumptionVO> {
        let mut user_stats: HashMap<String, (i64, i64, i64, f64, i64)> = HashMap::new();

        for log in usage_logs {
            let entry = user_stats
                .entry(log.user_id.clone())
                .or_insert((0, 0, 0, 0.0, 0));
            entry.0 += 1;
            if log.status.as_deref() == Some("success") {
                entry.1 += 1;
            } else {
                entry.2 += 1;
            }
            entry.3 += log.total_cost;
            entry.4 += log.total_tokens;
        }

        let mut result: Vec<UserConsumptionVO> = user_stats
            .into_iter()
            .map(|(user_id, stats)| {
                let success_rate = if stats.0 > 0 {
                    (stats.1 as f64 / stats.0 as f64) * 100.0
                } else {
                    0.0
                };
                UserConsumptionVO {
                    user_id,
                    username: None,
                    request_count: stats.0,
                    successful_count: stats.1,
                    failed_count: stats.2,
                    success_rate,
                    total_cost: stats.3,
                    total_tokens: stats.4,
                }
            })
            .collect();

        result.sort_by(|a, b| {
            b.total_cost
                .partial_cmp(&a.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.user_id.cmp(&b.user_id))
        });
        result.truncate(top_n);
        result
    }

    fn mask_api_key(&self, api_key: &str) -> String {
        if api_key.is_empty() {
            return "(empty)".to_string();
        }
        if api_key.len() > 8 {
            format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
        } else {
            "*".repeat(api_key.len())
        }
    }
}

impl Default for AdminStatsService {
    fn default() -> Self {
        Self::new()
    }
}
