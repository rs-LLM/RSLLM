// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入哈希映射类型
// 说明：用于存储logit_bias等键值对数据
use std::collections::HashMap;
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：导入聊天消息内容相关结构体
// 说明：用于处理聊天消息的内容结构
use super::content::ChatCompletionMessage;
// 用途：导入响应格式相关结构体
// 说明：用于处理响应格式配置
use super::response_format::ResponseFormat;
// 用途：导入流式处理相关结构体
// 说明：用于处理流式响应数据
use super::streaming::ChatCompletionChunk;
// 用途：从chat模块导入工具相关结构体
// 说明：避免重复定义，统一使用chat模块的类型
use super::chat::{ReasoningConfig, ToolChoice, ToolDefinition};

// 用途：输入内容枚举
// 说明：支持多种输入类型，包括文本、消息数组等
#[derive(Deserialize, Serialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum ResponseInput {
    // 用途：文本输入
    // 说明：简单的文本字符串输入
    Text(String),
    // 用途：消息数组输入
    // 说明：包含角色和内容的消息列表
    Messages(Vec<ChatCompletionMessage>),
    // 用途：多模态输入
    // 说明：包含文本和图像等多种媒体类型的输入
    MultiModal(Vec<serde_json::Value>),
}

// 用途：Responses API请求结构体
// 说明：用于表示Responses API的请求参数，这是OpenAI推荐的新API
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ResponseCreateParams {
    // 用途：模型名称
    // 说明：用于生成响应的AI模型标识符
    pub model: String,
    // 用途：输入内容
    // 说明：请求的输入内容，可以是文本、消息数组或多模态输入
    pub input: ResponseInput,
    // 用途：指令
    // 说明：系统或开发者消息，用于指导模型的行为
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    // 用途：对话历史
    // 说明：用于多轮对话的上下文管理
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<Vec<ChatCompletionMessage>>,
    // 用途：前一个响应ID
    // 说明：链接到前一个响应以维护对话状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    // 用途：后台执行
    // 说明：是否在后台运行响应生成
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    // 用途：包含内容
    // 说明：指定要包含的额外输出数据，如网络搜索结果或代码解释器输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
    // 用途：提示词引用
    // 说明：引用提示词模板
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    // 用途：推理配置
    // 说明：配置AI模型的推理过程参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
    // 用途：工具选择
    // 说明：控制工具调用的策略和方式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    // 用途：工具定义列表
    // 说明：可用的工具定义，用于函数调用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    // 用途：温度参数
    // 说明：控制生成内容的随机性，值越高越随机
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    // 用途：核采样参数
    // 说明：控制生成内容的多样性，值越小越保守
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    // 用途：最大令牌数
    // 说明：生成内容允许的最大token数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    // 用途：最大完成令牌数
    // 说明：专门限制完成内容的token数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    // 用途：停止词列表
    // 说明：遇到这些词汇时停止生成
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    // 用途：并行工具调用
    // 说明：是否允许并行执行多个工具调用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    // 用途：存在惩罚
    // 说明：减少内容重复的参数，数值越高越避免重复
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    // 用途：频率惩罚
    // 说明：根据词汇出现频率调整生成概率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    // 用途：对数偏差
    // 说明：手动调整特定词汇的选择概率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, i32>>,
    // 用途：用户标识
    // 说明：可选的用户标识符，用于跟踪和审计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    // 用途：响应格式
    // 说明：指定响应的格式类型和结构约束
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    // 用途：流式输出
    // 说明：是否启用流式响应，实时返回生成的内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

// 用途：Responses API响应枚举
// 说明：支持流式和非流式两种响应模式
pub enum ResponseCreateResponse {
    // 用途：流式响应模式
    // 说明：用于实时流式数据接收和处理
    Stream(futures::stream::BoxStream<'static, Result<ChatCompletionChunk, reqwest::Error>>),
    // 用途：非流式响应模式
    // 说明：用于一次性完整响应处理
    NonStream(Box<crate::domain::vo::responses::Response>),
}
