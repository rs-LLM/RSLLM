// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：导入聊天消息内容相关结构体
// 说明：用于处理聊天消息的内容结构
use super::content::ChatCompletionMessage;
// 用途：导入日志概率相关结构体
// 说明：用于处理日志概率统计
use super::logprob::LogProbs;

// 用途：聊天完成分块结构体
// 说明：用于表示流式响应的单个分块
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatCompletionChunk {
    // 用途：响应ID
    // 说明：当前聊天的唯一标识符，用于追踪和审计
    pub id: String,
    // 用途：对象类型
    // 说明：标识响应对象的类型，固定为"chat.completion.chunk"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    // 用途：创建时间
    // 说明：响应创建的时间戳
    pub created: u64,
    // 用途：模型名称
    // 说明：生成当前响应的AI模型名称
    pub model: String,
    // 用途：系统指纹
    // 说明：用于标识特定系统配置的指纹信息
    pub system_fingerprint: Option<String>,
    // 用途：选择列表
    // 说明：包含所有生成的响应选项
    pub choices: Vec<ChatCompletionChunkChoice>,
}

// 用途：聊天完成分块选择结构体
// 说明：用于表示单个流式响应分块的选择结果
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatCompletionChunkChoice {
    // 用途：选择索引
    // 说明：当前选择在choices数组中的位置索引
    pub index: u32,
    // 用途：消息内容
    // 说明：生成的消息内容，流式响应中逐步构建
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ChatCompletionMessage>,
    // 用途：结束原因
    // 说明：生成停止的原因，如"stop"、"length"、"content_filter"等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    // 用途：日志概率
    // 说明：包含token级概率信息的可选日志概率统计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogProbs>,
}
