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
    pub async fn get_trend_stats(&self, dimension: TimeDimension) -> Result<AdminTrendStatsVO> {
        let key = format!("{:?}", dimension);
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
        let stats = self.calculate_trend_stats(dimension.clone()).await?;
        cache.trends.insert(key, stats.clone());

        Ok(stats)
    }

    /// 获取用户统计（带缓存）
    ///
    /// 根据时间维度获取用户统计数据，缓存有效期为1小时
    pub async fn get_user_stats(&self, dimension: TimeDimension) -> Result<AdminUserStatsVO> {
        let key = format!("{:?}", dimension);
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
        let stats = self.calculate_user_stats(dimension.clone()).await?;
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

    /// 计算概览统计
    async fn calculate_overview_stats(&self) -> Result<AdminOverviewStatsVO> {
        // 查询所有用户
        let users = SysUser::select_all(pool!()).await?;
        let total_users = users.len() as i64;

        // 使用 RBAC 角色系统统计管理员数量
        // 查询所有角色
        let roles = RbacRole::select_all(pool!()).await?;
        // 查询所有用户角色关联
        let user_roles = RbacUserRole::select_all(pool!()).await?;

        // 找到名称为 "admin" 的角色ID
        let admin_role_id = roles
            .iter()
            .find(|r| {
                r.name
                    .as_deref()
                    .map_or(false, |n| n.to_lowercase() == "admin")
            })
            .and_then(|r| r.id.clone());

        // 统计拥有 admin 角色的用户数量
        let admin_count = if let Some(admin_id) = admin_role_id {
            user_roles
                .iter()
                .filter(|ur| ur.role_id.as_deref() == Some(&admin_id))
                .filter(|ur| {
                    // 确保用户状态为启用
                    users
                        .iter()
                        .find(|u| u.id.as_deref() == ur.user_id.as_deref())
                        .map_or(false, |u| u.state == Some(1))
                })
                .count() as i64
        } else {
            0
        };

        // 查询所有交易记录（总收入）
        let transactions = Transaction::select_all(pool!()).await?;
        let total_revenue: f64 = transactions
            .iter()
            .filter(|t| t.type_ == "recharge")
            .map(|t| t.amount)
            .sum();

        // 查询所有使用记录
        let usage_logs = AiHubUsageLog::select_all(pool!()).await?;
        let total_input_tokens: i64 = usage_logs.iter().map(|l| l.input_tokens).sum();
        let total_output_tokens: i64 = usage_logs.iter().map(|l| l.output_tokens).sum();
        let total_tokens: i64 = usage_logs.iter().map(|l| l.total_tokens).sum();
        let total_consumption: f64 = usage_logs.iter().map(|l| l.total_cost).sum();

        Ok(AdminOverviewStatsVO {
            total_users,
            admin_count,
            total_revenue,
            total_input_tokens,
            total_output_tokens,
            total_tokens,
            total_consumption,
            updated_at: DateTime::now().to_string(),
        })
    }

    /// 计算趋势统计
    async fn calculate_trend_stats(&self, dimension: TimeDimension) -> Result<AdminTrendStatsVO> {
        let (start_time, end_time) = self.get_time_range(&dimension)?;

        // 查询指定时间范围内的使用记录
        let map = rbs::value! {
            "created_at >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string()))),
            "created_at <=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string()))),
        };
        let usage_logs = AiHubUsageLog::select_by_map(pool!(), map).await?;

        // 按时间分组统计
        let mut revenue_by_time: HashMap<String, f64> = HashMap::new();
        let mut tokens_by_time: HashMap<String, f64> = HashMap::new();
        let mut requests_by_time: HashMap<String, f64> = HashMap::new();

        for log in &usage_logs {
            if let Some(created_at) = &log.created_at {
                let time_key = self.get_time_key(created_at, &dimension);

                *revenue_by_time.entry(time_key.clone()).or_insert(0.0) += log.total_cost;
                *tokens_by_time.entry(time_key.clone()).or_insert(0.0) += log.total_tokens as f64;
                *requests_by_time.entry(time_key).or_insert(0.0) += 1.0;
            }
        }

        // 转换为趋势数据点
        let revenue_trend = self.to_trend_data_points(&revenue_by_time);
        let token_trend = self.to_trend_data_points(&tokens_by_time);
        let request_trend = self.to_trend_data_points(&requests_by_time);

        Ok(AdminTrendStatsVO {
            dimension,
            revenue_trend,
            token_trend,
            request_trend,
            updated_at: DateTime::now().to_string(),
        })
    }

    /// 计算用户统计
    async fn calculate_user_stats(&self, dimension: TimeDimension) -> Result<AdminUserStatsVO> {
        let (start_time, end_time) = self.get_time_range(&dimension)?;

        // 查询指定时间范围内的新增用户
        let map = rbs::value! {
            "create_date >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string()))),
            "create_date <=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string()))),
        };
        let new_users = SysUser::select_by_map(pool!(), map).await?;
        let new_users_count = new_users.len() as i64;

        // 查询活跃用户（有使用记录的用户）
        let usage_map = rbs::value! {
            "created_at >=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string()))),
            "created_at <=": rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string()))),
        };
        let usage_logs = AiHubUsageLog::select_by_map(pool!(), usage_map).await?;
        let active_users: std::collections::HashSet<_> =
            usage_logs.iter().map(|l| &l.user_id).collect();
        let active_users_count = active_users.len() as i64;

        // 按时间分组统计用户增长
        let mut users_by_time: HashMap<String, f64> = HashMap::new();
        for user in &new_users {
            if let Some(create_date) = &user.create_date {
                let time_key = self.get_time_key(create_date, &dimension);
                *users_by_time.entry(time_key).or_insert(0.0) += 1.0;
            }
        }

        let user_growth_trend = self.to_trend_data_points(&users_by_time);

        Ok(AdminUserStatsVO {
            dimension,
            new_users: new_users_count,
            active_users: active_users_count,
            user_growth_trend,
            updated_at: DateTime::now().to_string(),
        })
    }

    /// 获取时间范围
    fn get_time_range(&self, dimension: &TimeDimension) -> Result<(DateTime, DateTime)> {
        let now = DateTime::now();

        let start_time = match dimension {
            TimeDimension::Day => {
                // 1天前
                let start_timestamp = now.unix_timestamp() as i64 - (24 * 60 * 60);
                DateTime::from_timestamp(start_timestamp)
            }
            TimeDimension::Week => {
                // 7天前
                let start_timestamp = now.unix_timestamp() as i64 - (7 * 24 * 60 * 60);
                DateTime::from_timestamp(start_timestamp)
            }
            TimeDimension::Month => {
                // 30天前
                let start_timestamp = now.unix_timestamp() as i64 - (30 * 24 * 60 * 60);
                DateTime::from_timestamp(start_timestamp)
            }
        };

        Ok((start_time, now))
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

        // 按时间排序
        points.sort_by(|a, b| a.label.cmp(&b.label));
        points
    }
}

impl Default for AdminStatsService {
    fn default() -> Self {
        Self::new()
    }
}
