//! 用量统计视图对象模块。
//! 定义聊天与嵌入场景的 token 使用量统计结构，供 AI Hub 接口对外返回。

// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

/// 通用使用统计结构体。
/// 表示文本生成类请求的输入、输出与总 token 消耗。
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

/// 嵌入使用统计结构体。
/// 表示嵌入请求的输入与总 token 消耗。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct EmbeddingUsage {
    // 用途：输入令牌数量
    // 说明：嵌入输入中包含的令牌数量
    pub prompt_tokens: u32,
    // 用途：总令牌数量
    // 说明：嵌入过程中处理的总令牌数量
    pub total_tokens: u32,
}
