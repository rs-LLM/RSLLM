//! 管理员统计视图对象模块。
//! 定义 AI Hub 管理员统计场景使用的响应视图对象。

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

/// 管理员概览统计视图对象
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
    /// 总请求数
    pub total_requests: i64,
    /// 成功请求数
    pub successful_requests: i64,
    /// 失败请求数
    pub failed_requests: i64,
    /// 成功率（0-100）
    pub success_rate: f64,
    /// 平均响应时延（毫秒）
    pub avg_response_time_ms: f64,
    /// 按模型聚合摘要
    pub model_summary: Vec<ModelAggregationVO>,
    /// 按API Key聚合摘要
    pub api_key_summary: Vec<ApiKeyAggregationVO>,
    /// 错误统计摘要
    pub error_summary: Vec<ErrorSummaryVO>,
}

/// 模型聚合统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ModelAggregationVO {
    /// 模型ID
    pub model_id: String,
    /// 模型名称（可选，便于前端展示）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    /// 请求数
    pub request_count: i64,
    /// 成功请求数
    pub successful_count: i64,
    /// 失败请求数
    pub failed_count: i64,
    /// 成功率
    pub success_rate: f64,
    /// 总输入tokens
    pub total_input_tokens: i64,
    /// 总输出tokens
    pub total_output_tokens: i64,
    /// 总消费
    pub total_cost: f64,
    /// 平均响应时延（毫秒）
    pub avg_response_time_ms: f64,
}

/// API Key聚合统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApiKeyAggregationVO {
    /// API Key（脱敏显示）
    pub api_key_masked: String,
    /// 请求数
    pub request_count: i64,
    /// 成功请求数
    pub successful_count: i64,
    /// 失败请求数
    pub failed_count: i64,
    /// 成功率
    pub success_rate: f64,
    /// 总消费
    pub total_cost: f64,
    /// 平均响应时延（毫秒）
    pub avg_response_time_ms: f64,
}

/// 错误统计摘要视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ErrorSummaryVO {
    /// 错误类型/状态码
    pub error_type: String,
    /// 错误次数
    pub count: i64,
    /// 占比（0-100）
    pub percentage: f64,
    /// 最近错误信息示例
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_message: Option<String>,
}

/// 管理员趋势统计视图对象
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
    /// 成功请求趋势
    pub success_trend: Vec<TrendDataPointVO>,
    /// 失败请求趋势
    pub failure_trend: Vec<TrendDataPointVO>,
    /// 成功率趋势
    pub success_rate_trend: Vec<TrendDataPointVO>,
    /// 平均响应时延趋势
    pub avg_response_time_trend: Vec<TrendDataPointVO>,
}

/// 趋势数据点视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TrendDataPointVO {
    /// 时间标签
    pub label: String,
    /// 数值
    pub value: f64,
}

/// 详细趋势数据点视图对象（含多维度）
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DetailedTrendDataPointVO {
    /// 时间标签
    pub label: String,
    /// 请求数
    pub request_count: i64,
    /// 成功请求数
    pub successful_count: i64,
    /// 失败请求数
    pub failed_count: i64,
    /// 成功率
    pub success_rate: f64,
    /// 平均响应时延（毫秒）
    pub avg_response_time_ms: f64,
    /// 总tokens
    pub total_tokens: i64,
    /// 总消费
    pub total_cost: f64,
}

/// 管理员用户统计视图对象
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
    /// 活跃用户总请求数
    pub total_requests: i64,
    /// 活跃用户成功请求数
    pub successful_requests: i64,
    /// 活跃用户失败请求数
    pub failed_requests: i64,
    /// 活跃用户成功率
    pub success_rate: f64,
    /// 活跃用户平均响应时延
    pub avg_response_time_ms: f64,
    /// 活跃用户总消费
    pub total_consumption: f64,
    /// 用户消费排行（Top N）
    pub top_consumers: Vec<UserConsumptionVO>,
}

/// 用户消费统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserConsumptionVO {
    /// 用户ID
    pub user_id: String,
    /// 用户名（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 请求数
    pub request_count: i64,
    /// 成功请求数
    pub successful_count: i64,
    /// 失败请求数
    pub failed_count: i64,
    /// 成功率
    pub success_rate: f64,
    /// 总消费
    pub total_cost: f64,
    /// 总tokens
    pub total_tokens: i64,
}
