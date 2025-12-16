// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：响应格式结构体
// 说明：用于指定AI模型的响应格式
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ResponseFormat {
    // 用途：响应格式类型
    // 说明：指定响应的格式类型，如"json_object"、"text"等
    #[serde(rename = "type")]
    pub r#type: String,
    // 用途：JSON模式
    // 说明：当type为"json_object"时，用于指定JSON响应的结构约束
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    // 用途：严格模式
    // 说明：当type为"json_object"时，指定是否严格遵循schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}
