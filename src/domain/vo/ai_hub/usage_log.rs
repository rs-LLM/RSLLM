// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：用量记录VO结构体
// 说明：用于API响应的用量记录数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AiHubUsageLogVO {
    // 用途：记录ID
    pub id: Option<String>,
    // 用途：请求ID
    pub request_id: Option<String>,
    // 用途：用户ID
    pub user_id: Option<String>,
    // 用途：模型ID
    pub model_id: Option<String>,
    // 用途：提供商ID
    pub provider_id: Option<String>,
    // 用途：输入token数量
    pub input_tokens: Option<i64>,
    // 用途：输出token数量
    pub output_tokens: Option<i64>,
    // 用途：总token数量
    pub total_tokens: Option<i64>,
    // 用途：输入费用（分）
    pub input_cost: Option<f64>,
    // 用途：输出费用（分）
    pub output_cost: Option<f64>,
    // 用途：总费用（分）
    pub total_cost: Option<f64>,
    // 用途：请求时间
    pub request_time: Option<String>,
    // 用途：响应时间
    pub response_time: Option<String>,
    // 用途：处理时长（毫秒）
    pub duration_ms: Option<i64>,
    // 用途：请求类型
    pub request_type: String,
    // 用途：状态
    pub status: String,
    // 用途：扩展字段
    pub extra: Option<serde_json::Value>,
    // 用途：创建时间
    pub created_at: Option<String>,
}

// 用途：用量统计VO结构体
// 说明：用于统计特定周期内的用量数据
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

// 用途：费用明细VO结构体
// 说明：用于展示详细的费用信息
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