// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：使用统计结构体
// 说明：用于统计AI模型生成过程中的资源使用情况
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Usage {
    // 用途：提示词令牌数量
    // 说明：输入提示词中包含的令牌数量
    pub prompt_tokens: u32,
    // 用途：完成令牌数量
    // 说明：AI生成的令牌数量
    pub completion_tokens: u32,
    // 用途：总令牌数量
    // 说明：提示词令牌和完成令牌的总和
    pub total_tokens: u32,
}

// 用途：嵌入使用统计结构体
// 说明：用于统计嵌入生成过程中的资源使用情况
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct EmbeddingUsage {
    // 用途：输入令牌数量
    // 说明：嵌入输入中包含的令牌数量
    pub prompt_tokens: u32,
    // 用途：总令牌数量
    // 说明：嵌入过程中处理的总令牌数量
    pub total_tokens: u32,
}
