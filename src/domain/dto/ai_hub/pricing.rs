// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Deserialize, Serialize};
// 用途：导入ToSchema trait
// 说明：用于OpenAPI文档生成
use utoipa::ToSchema;

// 用途：创建计费标准DTO
// 说明：用于创建新的计费标准
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreatePricingDTO {
    // 用途：模型ID
    pub model_id: String,
    // 用途：输入价格（每百万token）
    pub input_price: f64,
    // 用途：输出价格（每百万token）
    pub output_price: f64,
    // 用途：状态
    pub status: String,
    // 用途：描述
    pub description: Option<String>,
}

// 用途：更新计费标准DTO
// 说明：用于更新现有的计费标准
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdatePricingDTO {
    // 用途：计费标准ID
    pub id: String,
    // 用途：输入价格（每百万token）
    pub input_price: Option<f64>,
    // 用途：输出价格（每百万token）
    pub output_price: Option<f64>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：描述
    pub description: Option<String>,
}

// 用途：查询计费标准DTO
// 说明：用于查询计费标准列表
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QueryPricingDTO {
    // 用途：模型ID（可选）
    pub model_id: Option<String>,
    // 用途：状态（可选）
    pub status: Option<String>,
    // 用途：页码
    pub page: Option<u64>,
    // 用途：每页数量
    pub page_size: Option<u64>,
}
