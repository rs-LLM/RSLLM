// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：导入日志概率相关结构体
// 说明：用于处理日志概率统计
use crate::domain::dto::logprob::LogProbs;
// 用途：导入使用统计相关结构体
// 说明：用于统计资源使用情况
use super::usage::Usage;

// 用途：文本完成响应结构体
// 说明：用于表示文本生成的完整响应，包含所有生成的内容和统计信息
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct CompletionResponse {
    // 用途：响应ID
    // 说明：当前文本生成的唯一标识符，用于追踪和审计
    pub id: String,
    // 用途：对象类型
    // 说明：标识响应对象的类型，通常为"text_completion"
    pub object: String,
    // 用途：创建时间
    // 说明：响应创建的时间戳
    pub created: u64,
    // 用途：模型名称
    // 说明：生成当前响应的AI模型名称
    pub model: String,
    // 用途：选择列表
    // 说明：包含所有生成的文本选项
    pub choices: Vec<CompletionChoice>,
    // 用途：使用统计
    // 说明：当前文本生成的资源使用统计信息
    pub usage: Usage,
}

// 用途：文本完成选择结构体
// 说明：用于表示单个文本生成的选择结果，包含生成的文本和相关信息
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct CompletionChoice {
    // 用途：生成的文本
    // 说明：AI生成的文本内容
    pub text: String,
    // 用途：选择索引
    // 说明：当前选择在choices数组中的位置索引
    pub index: u32,
    // 用途：日志概率
    // 说明：包含token级概率信息的可选日志概率统计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogProbs>,
    // 用途：结束原因
    // 说明：生成停止的原因，如"stop"、"length"、"content_filter"等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}
