//! 用户统计服务模块
//! 提供用户面板统计数据聚合和缓存功能

use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::domain::table::basic::sys_user::SysUser;
use crate::domain::vo::ai_hub::admin_stats::TimeDimension;
use crate::domain::vo::ai_hub::user_stats::*;
use crate::error::Result;
use crate::pool;
use rbatis::rbdc::DateTime;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 用户统计服务
///
/// 负责用户面板统计数据的聚合和缓存
#[derive(Clone)]
pub struct UserStatsService {
    /// 缓存数据
    cache: Arc<RwLock<UserStatsCache>>,
}

/// 统计数据缓存
struct UserStatsCache {
    /// 概览统计（按用户ID）
    overview: HashMap<String, CachedUserStats>,
    /// 趋势统计（按用户ID和维度）
    trends: HashMap<String, CachedUserTrendStats>,
}

/// 缓存的用户统计
struct CachedUserStats {
    stats: UserStatsVO,
    updated_at: DateTime,
}

/// 缓存的用户趋势统计
struct CachedUserTrendStats {
    stats: UserTrendStatsVO,
    updated_at: DateTime,
}

impl Default for UserStatsCache {
    fn default() -> Self {
        Self {
            overview: HashMap::new(),
            trends: HashMap::new(),
        }
    }
}

impl UserStatsService {
    const TOP_N: usize = 10;

    /// 创建新的统计服务实例
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(UserStatsCache::default())),
        }
    }

    /// 获取用户概览统计（带缓存）
    ///
    /// 缓存有效期为1小时
    pub async fn get_user_overview(&self, user_id: &str) -> Result<UserStatsVO> {
        let mut cache = self.cache.write().await;

        // 检查缓存是否有效（1小时内）
        if let Some(cached) = cache.overview.get(user_id) {
            let now = DateTime::now();
            let elapsed_seconds = now.unix_timestamp() - cached.updated_at.unix_timestamp();
            if elapsed_seconds < 3600 {
                return Ok(cached.stats.clone());
            }
        }

        // 缓存过期或不存在，重新计算
        let stats = self.calculate_user_overview(user_id).await?;
        cache.overview.insert(
            user_id.to_string(),
            CachedUserStats {
                stats: stats.clone(),
                updated_at: DateTime::now(),
            },
        );

        Ok(stats)
    }

    /// 获取用户趋势统计（带缓存）
    ///
    /// 根据时间维度获取趋势数据，缓存有效期为1小时
    pub async fn get_user_trend_stats(
        &self,
        user_id: &str,
        dimension: TimeDimension,
    ) -> Result<UserTrendStatsVO> {
        let key = format!("{}_{:?}", user_id, dimension);
        let mut cache = self.cache.write().await;

        // 检查缓存是否有效
        if let Some(cached) = cache.trends.get(&key) {
            let now = DateTime::now();
            let elapsed_seconds = now.unix_timestamp() - cached.updated_at.unix_timestamp();
            if elapsed_seconds < 3600 {
                return Ok(cached.stats.clone());
            }
        }

        // 缓存过期或不存在，重新计算
        let stats = self
            .calculate_user_trend_stats(user_id, dimension.clone())
            .await?;
        cache.trends.insert(
            key,
            CachedUserTrendStats {
                stats: stats.clone(),
                updated_at: DateTime::now(),
            },
        );

        Ok(stats)
    }

    /// 清空指定用户的缓存
    ///
    /// 强制清空指定用户的所有缓存数据
    pub async fn clear_user_cache(&self, user_id: &str) {
        let mut cache = self.cache.write().await;
        cache.overview.remove(user_id);

        // 清除该用户的所有趋势缓存
        cache
            .trends
            .retain(|key, _| !key.starts_with(&format!("{}_", user_id)));
    }

    /// 计算用户概览统计
    async fn calculate_user_overview(&self, user_id: &str) -> Result<UserStatsVO> {
        // 查询用户信息（获取余额）
        let user_map = rbs::value! {
            "id": user_id,
        };
        let users = SysUser::select_by_map(pool!(), user_map).await?;
        let balance = users.first().and_then(|u| u.balance).unwrap_or(0.0);

        // 查询该用户的所有使用记录
        let usage_map = rbs::value! {
            "user_id": user_id,
        };
        let usage_logs = AiHubUsageLog::select_by_map(pool!(), usage_map).await?;
        let total_input_tokens: i64 = usage_logs.iter().map(|l| l.input_tokens).sum();
        let total_output_tokens: i64 = usage_logs.iter().map(|l| l.output_tokens).sum();
        let total_tokens: i64 = usage_logs.iter().map(|l| l.total_tokens).sum();
        let total_cost: f64 = usage_logs.iter().map(|l| l.total_cost).sum();
        let total_requests = usage_logs.len() as i64;
        let successful_requests = usage_logs
            .iter()
            .filter(|l| l.status.as_deref() == Some("success"))
            .count() as i64;
        let failed_requests = total_requests - successful_requests;
        let success_rate = Self::calc_success_rate(successful_requests, total_requests);
        let avg_response_time_ms = Self::calc_avg_response_time_ms(&usage_logs);

        let model_summary = self.aggregate_dimension(
            &usage_logs,
            |log| {
                if log.model_id.is_empty() {
                    "unknown".to_string()
                } else {
                    log.model_id.clone()
                }
            },
            Self::TOP_N,
        );
        let provider_summary =
            self.aggregate_dimension(&usage_logs, |log| self.extract_provider(log), Self::TOP_N);
        let request_type_summary = self.aggregate_dimension(
            &usage_logs,
            |log| {
                log.request_type
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            },
            Self::TOP_N,
        );

        Ok(UserStatsVO {
            user_id: user_id.to_string(),
            balance,
            total_cost,
            total_input_tokens,
            total_output_tokens,
            total_tokens,
            updated_at: DateTime::now().to_string(),
            total_requests,
            successful_requests,
            failed_requests,
            success_rate,
            avg_response_time_ms,
            model_summary,
            provider_summary,
            request_type_summary,
        })
    }

    /// 计算用户趋势统计
    async fn calculate_user_trend_stats(
        &self,
        user_id: &str,
        dimension: TimeDimension,
    ) -> Result<UserTrendStatsVO> {
        let (start_time, end_time) = self.get_time_range(&dimension)?;

        // 查询指定时间范围内的使用记录
        let map = rbs::value! {
            "user_id": user_id,
            "created_at >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string()))),
            "created_at <=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string()))),
        };
        let usage_logs = AiHubUsageLog::select_by_map(pool!(), map).await?;

        // 按时间分组统计
        let mut tokens_by_time: HashMap<String, UserTrendDataPointVO> = HashMap::new();
        let mut response_time_by_time: HashMap<String, (i64, i64)> = HashMap::new();

        for log in &usage_logs {
            if let Some(created_at) = &log.created_at {
                let time_key = self.get_time_key(created_at, &dimension);

                // 累加token数据
                let entry =
                    tokens_by_time
                        .entry(time_key.clone())
                        .or_insert(UserTrendDataPointVO {
                            label: time_key.clone(),
                            input_tokens: 0,
                            output_tokens: 0,
                            total_tokens: 0,
                            cost: 0.0,
                            request_count: 0,
                            successful_count: 0,
                            failed_count: 0,
                            success_rate: 0.0,
                            avg_response_time_ms: 0.0,
                        });
                entry.input_tokens += log.input_tokens;
                entry.output_tokens += log.output_tokens;
                entry.total_tokens += log.total_tokens;
                entry.cost += log.total_cost;
                entry.request_count += 1;
                if log.status.as_deref() == Some("success") {
                    entry.successful_count += 1;
                } else {
                    entry.failed_count += 1;
                }
                if let Some(rt) = log.response_time_ms {
                    let rt_entry = response_time_by_time.entry(time_key).or_insert((0, 0));
                    rt_entry.0 += rt;
                    rt_entry.1 += 1;
                }
            }
        }

        for (key, point) in &mut tokens_by_time {
            point.success_rate =
                Self::calc_success_rate(point.successful_count, point.request_count);
            if let Some((sum, count)) = response_time_by_time.get(key) {
                point.avg_response_time_ms = if *count > 0 {
                    *sum as f64 / *count as f64
                } else {
                    0.0
                };
            }
        }

        // 生成完整的时间键列表
        let time_keys = self.generate_time_keys(&dimension);

        // 填充缺失的数据点
        let token_trend = self.fill_missing_data_points(time_keys.clone(), &tokens_by_time);
        let cost_trend = self.fill_missing_data_points(time_keys, &tokens_by_time);
        let request_trend = token_trend.clone();
        let success_rate_trend = token_trend.clone();
        let avg_response_time_trend = token_trend.clone();

        Ok(UserTrendStatsVO {
            user_id: user_id.to_string(),
            dimension,
            token_trend,
            cost_trend,
            request_trend,
            success_rate_trend,
            avg_response_time_trend,
            updated_at: DateTime::now().to_string(),
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

    /// 生成完整的时间键列表
    fn generate_time_keys(&self, dimension: &TimeDimension) -> Vec<String> {
        let now = DateTime::now();
        let mut time_keys = Vec::new();

        match dimension {
            TimeDimension::Day => {
                // 生成最近7天的日期
                for i in 0..7 {
                    let timestamp = now.unix_timestamp() as i64 - (i * 24 * 60 * 60);
                    let dt = DateTime::from_timestamp(timestamp);
                    let dt_str = dt.to_string();
                    let date_key = dt_str[..10].to_string(); // YYYY-MM-DD
                    time_keys.push(date_key);
                }
                time_keys.reverse(); // 从早到晚排序
            }
            TimeDimension::Week => {
                // 生成最近4周的数据（按天分组，共28天）
                for i in 0..28 {
                    let timestamp = now.unix_timestamp() as i64 - (i * 24 * 60 * 60);
                    let dt = DateTime::from_timestamp(timestamp);
                    let dt_str = dt.to_string();
                    let date_key = dt_str[..10].to_string(); // YYYY-MM-DD
                    time_keys.push(date_key);
                }
                time_keys.reverse(); // 从早到晚排序
            }
            TimeDimension::Month => {
                // 生成最近3个月的数据
                for i in 0..3 {
                    let timestamp = now.unix_timestamp() as i64 - (i * 30 * 24 * 60 * 60);
                    let dt = DateTime::from_timestamp(timestamp);
                    let dt_str = dt.to_string();
                    let month_key = dt_str[..7].to_string(); // YYYY-MM
                    time_keys.push(month_key);
                }
                time_keys.reverse(); // 从早到晚排序
            }
        }

        time_keys
    }

    /// 填充缺失的数据点
    fn fill_missing_data_points(
        &self,
        time_keys: Vec<String>,
        data_map: &HashMap<String, UserTrendDataPointVO>,
    ) -> Vec<UserTrendDataPointVO> {
        time_keys
            .into_iter()
            .map(|key| {
                data_map.get(&key).cloned().unwrap_or(UserTrendDataPointVO {
                    label: key.clone(),
                    input_tokens: 0,
                    output_tokens: 0,
                    total_tokens: 0,
                    cost: 0.0,
                    request_count: 0,
                    successful_count: 0,
                    failed_count: 0,
                    success_rate: 0.0,
                    avg_response_time_ms: 0.0,
                })
            })
            .collect()
    }

    fn calc_success_rate(successful_requests: i64, total_requests: i64) -> f64 {
        if total_requests > 0 {
            (successful_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        }
    }

    fn calc_avg_response_time_ms(logs: &[AiHubUsageLog]) -> f64 {
        let mut sum = 0_i64;
        let mut count = 0_i64;
        for log in logs {
            if let Some(rt) = log.response_time_ms {
                sum += rt;
                count += 1;
            }
        }

        if count > 0 {
            sum as f64 / count as f64
        } else {
            0.0
        }
    }

    fn aggregate_dimension<F>(
        &self,
        usage_logs: &[AiHubUsageLog],
        key_extractor: F,
        top_n: usize,
    ) -> Vec<UserDimensionAggregationVO>
    where
        F: Fn(&AiHubUsageLog) -> String,
    {
        let mut stats: HashMap<String, (i64, i64, i64, i64, f64, i64, i64)> = HashMap::new();

        for log in usage_logs {
            let key = key_extractor(log);
            let entry = stats.entry(key).or_insert((0, 0, 0, 0, 0.0, 0, 0));
            entry.0 += 1;
            if log.status.as_deref() == Some("success") {
                entry.1 += 1;
            } else {
                entry.2 += 1;
            }
            entry.3 += log.total_tokens;
            entry.4 += log.total_cost;
            if let Some(rt) = log.response_time_ms {
                entry.5 += rt;
                entry.6 += 1;
            }
        }

        let mut result: Vec<UserDimensionAggregationVO> = stats
            .into_iter()
            .map(|(key, item)| UserDimensionAggregationVO {
                key,
                request_count: item.0,
                successful_count: item.1,
                failed_count: item.2,
                success_rate: Self::calc_success_rate(item.1, item.0),
                total_tokens: item.3,
                total_cost: item.4,
                avg_response_time_ms: if item.6 > 0 {
                    item.5 as f64 / item.6 as f64
                } else {
                    0.0
                },
            })
            .collect();

        result.sort_by(|a, b| {
            b.request_count
                .cmp(&a.request_count)
                .then_with(|| a.key.cmp(&b.key))
        });
        result.truncate(top_n);
        result
    }

    fn extract_provider(&self, log: &AiHubUsageLog) -> String {
        if let Some(extra) = &log.extra {
            if let Some(provider) = extra.get("provider").and_then(|v| v.as_str()) {
                return provider.to_string();
            }
        }

        let model_id = log.model_id.as_str();
        if let Some((provider, _)) = model_id.split_once('/') {
            if !provider.is_empty() {
                return provider.to_string();
            }
        }
        if let Some((provider, _)) = model_id.split_once(':') {
            if !provider.is_empty() {
                return provider.to_string();
            }
        }
        "unknown".to_string()
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
}

impl Default for UserStatsService {
    fn default() -> Self {
        Self::new()
    }
}
