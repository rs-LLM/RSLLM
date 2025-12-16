// 用途：导入序列化和反序列化支持
// 说明：用于嵌入数据的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：嵌入请求结构体
// 说明：用于表示向量嵌入生成的请求参数
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct EmbeddingsRequest {
    // 用途：模型名称
    // 说明：用于生成嵌入的模型标识符
    pub model: String,
    // 用途：输入数据
    // 说明：需要生成嵌入的文本或token序列
    pub input: EmbeddingsInput,
    // 用途：用户标识
    // 说明：可选的用户标识符，用于跟踪和审计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    // 用途：编码格式
    // 说明：嵌入向量的输出格式，如"float"、"base64"等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
}

// 用途：嵌入输入枚举
// 说明：支持多种形式的嵌入输入，包括字符串、字符串数组、token ID等
#[derive(Deserialize, Serialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum EmbeddingsInput {
    // 用途：单个字符串
    // 说明：单个文本字符串的嵌入
    Single(String),
    // 用途：字符串数组
    // 说明：多个文本字符串的批量嵌入
    Multiple(Vec<String>),
    // 用途：单个token ID数组
    // 说明：单个序列的token ID嵌入
    SingleTokenIds(Vec<i32>),
    // 用途：多个token ID数组
    // 说明：多个序列的token ID批量嵌入
    MultipleTokenIds(Vec<Vec<i32>>),
}
