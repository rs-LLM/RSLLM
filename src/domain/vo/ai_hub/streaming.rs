//! 流式响应VO模块
//! 提供流式响应相关的视图对象

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 流式聊天完成块
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatCompletionChunk {
    /// 响应ID
    pub id: String,
    /// 对象类型
    pub object: Option<String>,
    /// 创建时间戳
    pub created: Option<u64>,
    /// 模型名称
    pub model: String,
    /// 选择器列表
    pub choices: Vec<ChatCompletionChunkChoice>,
    /// 系统指纹（可选）
    pub system_fingerprint: Option<String>,
}

/// 流式聊天完成选择器
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatCompletionChunkChoice {
    /// 选择器索引
    pub index: i32,
    /// 增量数据
    pub delta: ChatCompletionChunkDelta,
    /// 完成原因
    pub finish_reason: Option<String>,
}

/// 流式聊天完成增量
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ChatCompletionChunkDelta {
    /// 角色
    pub role: Option<String>,
    /// 内容
    pub content: Option<String>,
    /// 工具调用（可选）
    pub tool_calls: Option<Vec<ToolCallChunk>>,
}

/// 工具调用块
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ToolCallChunk {
    /// 工具调用索引
    pub index: i32,
    /// 工具名称
    pub name: Option<String>,
    /// 参数增量
    pub arguments: Option<String>,
}

/// WebSocket消息类型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebSocketMessageType {
    /// 连接建立
    Connected,
    /// 心跳包
    Ping,
    /// 心跳响应
    Pong,
    /// 聊天完成块
    ChatChunk(ChatCompletionChunk),
    /// 聊天完成
    ChatComplete(ChatCompletion),
    /// 错误消息
    Error(String),
    /// 配额警告
    QuotaWarning(QuotaWarning),
    /// 会话结束
    Disconnected,
}

/// WebSocket聊天请求
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebSocketChatRequest {
    /// 请求类型
    pub request_type: String,
    /// 模型名称
    pub model: String,
    /// 消息列表
    pub messages: Vec<WebSocketMessage>,
    /// 是否流式
    pub stream: Option<bool>,
    /// 最大token数
    pub max_tokens: Option<i32>,
    /// 温度
    pub temperature: Option<f64>,
}

/// WebSocket消息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebSocketMessage {
    /// 角色
    pub role: String,
    /// 内容
    pub content: String,
}

/// 配额警告
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuotaWarning {
    /// 用户ID
    pub user_id: String,
    /// 剩余额度
    pub remaining_quota: f64,
    /// 使用率
    pub usage_rate: f64,
    /// 警告消息
    pub message: String,
}

/// 聊天完成（用于WebSocket完成消息）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletion {
    /// 响应ID
    pub id: String,
    /// 对象类型
    pub object: Option<String>,
    /// 创建时间戳
    pub created: Option<u64>,
    /// 模型名称
    pub model: String,
    /// 选择器列表
    pub choices: Vec<ChatCompletionChoice>,
    /// 使用统计
    pub usage: Usage,
    /// 系统指纹（可选）
    pub system_fingerprint: Option<String>,
}

/// 聊天完成选择器
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionChoice {
    /// 选择器索引
    pub index: i32,
    /// 消息
    pub message: ChatCompletionMessage,
    /// 完成原因
    pub finish_reason: Option<String>,
}

/// 聊天完成消息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionMessage {
    /// 角色
    pub role: String,
    /// 内容
    pub content: Option<String>,
    /// 工具调用（可选）
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// 工具调用
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    /// 工具名称
    pub name: String,
    /// 参数
    pub arguments: String,
}

/// 使用统计
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Usage {
    /// 输入tokens
    pub prompt_tokens: u32,
    /// 输出tokens
    pub completion_tokens: u32,
    /// 总tokens
    pub total_tokens: u32,
}