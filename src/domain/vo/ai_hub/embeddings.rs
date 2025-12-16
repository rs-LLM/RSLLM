// 用途：导入序列化和反序列化支持
// 说明：用于嵌入数据的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入JSON值类型
// 说明：用于处理灵活的嵌入向量格式
use serde_json::Value;
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：导入嵌入使用统计相关结构体
// 说明：用于统计嵌入操作的使用情况
use super::usage::EmbeddingUsage;

// 用途：嵌入响应结构体
// 说明：用于表示嵌入生成的结果响应
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct EmbeddingsResponse {
    // 用途：对象类型
    // 说明：标识响应对象的类型，固定为"list"
    pub object: String,
    // 用途：嵌入数据列表
    // 说明：包含所有生成的嵌入向量
    pub data: Vec<Embeddings>,
    // 用途：模型名称
    // 说明：生成嵌入使用的模型名称
    pub model: String,
    // 用途：使用统计
    // 说明：嵌入操作的使用统计信息
    pub usage: EmbeddingUsage,
}

// 用途：嵌入数据结构体
// 说明：用于表示单个文本的嵌入结果
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct Embeddings {
    // 用途：对象类型
    // 说明：标识对象的类型，固定为"embedding"
    pub object: String,
    // 用途：嵌入向量
    // 说明：生成的向量嵌入数据
    pub embedding: Embedding,
    // 用途：索引
    // 说明：在输入序列中的位置索引
    pub index: usize,
}

// 用途：嵌入向量枚举
// 说明：支持多种格式的嵌入向量表示
#[derive(Deserialize, Serialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum Embedding {
    // 用途：字符串表示
    // 说明：嵌入向量的字符串格式，通常是base64编码
    String(String),
    // 用途：浮点数数组
    // 说明：嵌入向量的浮点数表示
    Float(Vec<f32>),
    // 用途：JSON值
    // 说明：嵌入向量的JSON格式表示，支持复杂结构
    Json(Value),
}
