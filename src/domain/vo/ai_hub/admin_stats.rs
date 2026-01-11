//! 管理员统计VO模块
//! 提供管理员面板统计相关的视图对象

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 时间维度枚举
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TimeDimension {
    /// 日维度
    Day,
    /// 周维度
    Week,
    /// 月维度
    Month,
}

/// 管理员概览统计VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminOverviewStatsVO {
    /// 总用户数
    pub total_users: i64,
    /// 管理员数
    pub admin_count: i64,
    /// 总收入
    pub total_revenue: f64,
    /// 总输入tokens
    pub total_input_tokens: i64,
    /// 总输出tokens
    pub total_output_tokens: i64,
    /// 总tokens
    pub total_tokens: i64,
    /// 总消费
    pub total_consumption: f64,
    /// 数据更新时间
    pub updated_at: String,
}

/// 管理员趋势统计VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminTrendStatsVO {
    /// 时间维度
    pub dimension: TimeDimension,
    /// 收入趋势
    pub revenue_trend: Vec<TrendDataPointVO>,
    /// Token使用趋势
    pub token_trend: Vec<TrendDataPointVO>,
    /// 请求数趋势
    pub request_trend: Vec<TrendDataPointVO>,
    /// 数据更新时间
    pub updated_at: String,
}

/// 趋势数据点VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TrendDataPointVO {
    /// 时间标签
    pub label: String,
    /// 数值
    pub value: f64,
}

/// 管理员用户统计VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminUserStatsVO {
    /// 时间维度
    pub dimension: TimeDimension,
    /// 新增用户数
    pub new_users: i64,
    /// 活跃用户数
    pub active_users: i64,
    /// 用户增长趋势
    pub user_growth_trend: Vec<TrendDataPointVO>,
    /// 数据更新时间
    pub updated_at: String,
}
