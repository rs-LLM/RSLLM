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

        Ok(UserStatsVO {
            user_id: user_id.to_string(),
            balance,
            total_cost,
            total_input_tokens,
            total_output_tokens,
            total_tokens,
            updated_at: DateTime::now().to_string(),
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
                        });
                entry.input_tokens += log.input_tokens;
                entry.output_tokens += log.output_tokens;
                entry.total_tokens += log.total_tokens;
                entry.cost += log.total_cost;
            }
        }

        // 生成完整的时间键列表
        let time_keys = self.generate_time_keys(&dimension);

        // 填充缺失的数据点
        let token_trend = self.fill_missing_data_points(time_keys.clone(), &tokens_by_time);
        let cost_trend = self.fill_missing_data_points(time_keys, &tokens_by_time);

        Ok(UserTrendStatsVO {
            user_id: user_id.to_string(),
            dimension,
            token_trend,
            cost_trend,
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
                })
            })
            .collect()
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
