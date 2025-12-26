// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入ToSchema trait
// 说明：用于OpenAPI文档生成
use utoipa::ToSchema;

// 用途：计费标准VO结构体
// 说明：用于API响应的计费标准数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PricingVO {
    // 用途：计费标准ID
    pub id: Option<String>,
    // 用途：模型ID
    pub model_id: String,
    // 用途：模型名称
    pub model_name: Option<String>,
    // 用途：输入价格（每百万token）
    pub input_price: f64,
    // 用途：输出价格（每百万token）
    pub output_price: f64,
    // 用途：状态
    pub status: String,
    // 用途：描述
    pub description: Option<String>,
    // 用途：创建时间
    pub created_at: Option<String>,
    // 用途：更新时间
    pub updated_at: Option<String>,
}
