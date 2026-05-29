//! 用量日志视图对象模块。
//! 定义 AI Hub 请求用量记录、聚合统计、费用明细与趋势数据的视图对象。

// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：用量记录视图对象结构体
// 说明：用于API响应的用量记录数据
/// 用量记录视图对象。
/// 表示单次请求在模型调用过程中的 token、费用、状态与时序信息。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AiHubUsageLogVO {
    pub id: String,
    pub user_id: String,
    pub api_key: String,
    pub model_id: String,
    pub provider_id: String,
    pub request_type: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub input_price: f64,
    pub output_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    pub status_code: i32,
    pub error_message: Option<String>,
    pub request_time: i64,
    pub response_time: i64,
    pub response_time_ms: i64,
    pub ttfb_ms: i64,
    pub upstream_latency_ms: i64,
    pub local_postprocess_ms: i64,
    pub cache_hit: bool,
    pub cached_tokens: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_oauth_account_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_oauth_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_oauth_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_oauth_provider_type: Option<String>,

    pub created_at: String,
}

// 用途：用量统计视图对象结构体
// 说明：用于统计特定周期内的用量数据
/// 用量统计视图对象。
/// 表示指定统计周期内的请求总量、token 汇总与成功失败分布。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageStatisticsVO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：统计周期
    pub period: String,
    // 用途：总请求数
    pub total_requests: i64,
    // 用途：总输入token
    pub total_input_tokens: i64,
    // 用途：总输出token
    pub total_output_tokens: i64,
    // 用途：总token
    pub total_tokens: i64,
    // 用途：总费用（分）
    pub total_cost: f64,
    // 用途：成功请求数
    pub success_requests: i64,
    // 用途：失败请求数
    pub failed_requests: i64,
}

// 用途：费用明细视图对象结构体
// 说明：用于展示详细的费用信息
/// 费用明细视图对象。
/// 表示按模型聚合的费用构成、token 消耗与请求次数明细。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CostDetailVO {
    // 用途：模型ID
    pub model_id: String,
    // 用途：模型名称
    pub model_name: String,
    // 用途：总费用（分）
    pub total_cost: f64,
    // 用途：输入费用（分）
    pub input_cost: f64,
    // 用途：输出费用（分）
    pub output_cost: f64,
    // 用途：总token
    pub total_tokens: i64,
    // 用途：请求次数
    pub request_count: i64,
}

// 用途：用量趋势视图对象结构体
// 说明：用于展示按时间维度的用量趋势数据
/// 用量趋势视图对象。
/// 表示按时间粒度统计的 token、费用与请求量趋势点数据。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageTrendVO {
    // 用途：日期
    pub date: Option<String>,
    // 用途：总token
    pub total_tokens: Option<i64>,
    // 用途：输入token
    pub input_tokens: Option<i64>,
    // 用途：输出token
    pub output_tokens: Option<i64>,
    // 用途：总费用（分）
    pub total_cost: Option<f64>,
    // 用途：请求次数
    pub request_count: Option<i64>,
}
