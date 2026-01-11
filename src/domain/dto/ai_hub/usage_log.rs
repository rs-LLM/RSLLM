// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：创建用量记录DTO
// 说明：用于创建新的用量记录
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateUsageLogDTO {
    // 用途：请求ID
    pub request_id: Option<String>,
    // 用途：用户ID
    pub user_id: String,
    // 用途：模型ID
    pub model_id: String,
    // 用途：提供商ID
    pub provider_id: String,
    // 用途：输入token数量
    pub input_tokens: i64,
    // 用途：输出token数量
    pub output_tokens: i64,
    // 用途：输入费用（分）
    pub input_cost: f64,
    // 用途：输出费用（分）
    pub output_cost: f64,
    // 用途：总费用（分）
    pub total_cost: f64,
    // 用途：请求时间
    pub request_time: Option<String>,
    // 用途：响应时间
    pub response_time: Option<String>,
    // 用途：处理时长（毫秒）
    pub duration_ms: i64,
    // 用途：请求类型
    pub request_type: String,
    // 用途：状态
    pub status: String,
    // 用途：扩展字段
    pub extra: Option<serde_json::Value>,
}

// 用途：更新用量记录DTO
// 说明：用于更新现有的用量记录
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateUsageLogDTO {
    // 用途：输入token数量
    pub input_tokens: Option<i64>,
    // 用途：输出token数量
    pub output_tokens: Option<i64>,
    // 用途：输入费用（分）
    pub input_cost: Option<f64>,
    // 用途：输出费用（分）
    pub output_cost: Option<f64>,
    // 用途：总费用（分）
    pub total_cost: Option<f64>,
    // 用途：响应时间
    pub response_time: Option<String>,
    // 用途：处理时长（毫秒）
    pub duration_ms: Option<i64>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：扩展字段
    pub extra: Option<serde_json::Value>,
}

// 用途：用量记录查询DTO
// 说明：用于查询用量记录列表
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageLogQueryDTO {
    // 用途：用户ID
    pub user_id: Option<String>,
    // 用途：模型ID
    pub model_id: Option<String>,
    // 用途：提供商ID
    pub provider_id: Option<String>,
    // 用途：请求类型
    pub request_type: Option<String>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：开始时间
    pub start_time: Option<String>,
    // 用途：结束时间
    pub end_time: Option<String>,
    // 用途：页码
    pub page: Option<i64>,
    // 用途：每页数量
    pub page_size: Option<i64>,
}

// 用途：用量统计查询DTO
// 说明：用于查询用量统计数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UsageStatisticsQueryDTO {
    // 用途：用户ID
    pub user_id: Option<String>,
    // 用途：统计周期
    pub period: Option<String>,
    // 用途：开始时间
    pub start_time: Option<String>,
    // 用途：结束时间
    pub end_time: Option<String>,
    // 用途：模型ID列表
    pub model_ids: Option<Vec<String>>,
    // 用途：提供商ID列表
    pub provider_ids: Option<Vec<String>>,
}
