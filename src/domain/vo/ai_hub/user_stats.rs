//! 用户统计视图对象模块。
//! 定义 AI Hub 用户统计场景使用的响应视图对象。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::admin_stats::TimeDimension;

/// 用户概览统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserStatsVO {
    /// 用户ID
    pub user_id: String,
    /// 当前余额
    pub balance: f64,
    /// 总消费
    pub total_cost: f64,
    /// 总输入tokens
    pub total_input_tokens: i64,
    /// 总输出tokens
    pub total_output_tokens: i64,
    /// 总tokens
    pub total_tokens: i64,
    /// 数据更新时间
    pub updated_at: String,
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
    pub model_summary: Vec<UserDimensionAggregationVO>,
    pub provider_summary: Vec<UserDimensionAggregationVO>,
    pub request_type_summary: Vec<UserDimensionAggregationVO>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserDimensionAggregationVO {
    pub key: String,
    pub request_count: i64,
    pub successful_count: i64,
    pub failed_count: i64,
    pub success_rate: f64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub avg_response_time_ms: f64,
}

/// 用户趋势统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserTrendStatsVO {
    /// 用户ID
    pub user_id: String,
    /// 时间维度
    pub dimension: TimeDimension,
    /// Token使用趋势
    pub token_trend: Vec<UserTrendDataPointVO>,
    /// 消费趋势
    pub cost_trend: Vec<UserTrendDataPointVO>,
    /// 数据更新时间
    pub updated_at: String,
    /// 请求趋势
    pub request_trend: Vec<UserTrendDataPointVO>,
    /// 成功率趋势
    pub success_rate_trend: Vec<UserTrendDataPointVO>,
    /// 平均响应时延趋势
    pub avg_response_time_trend: Vec<UserTrendDataPointVO>,
}

/// 用户趋势数据点视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserTrendDataPointVO {
    /// 时间标签
    pub label: String,
    /// 输入tokens
    pub input_tokens: i64,
    /// 输出tokens
    pub output_tokens: i64,
    /// 总tokens
    pub total_tokens: i64,
    /// 消费金额
    pub cost: f64,
    pub request_count: i64,
    pub successful_count: i64,
    pub failed_count: i64,
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
}

/// 刷新统计响应视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RefreshStatsResponseVO {
    /// 是否刷新成功
    pub refreshed: bool,
    /// 刷新时间
    pub timestamp: String,
}
