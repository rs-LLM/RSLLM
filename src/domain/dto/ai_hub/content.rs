// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：聊天消息内容枚举
// 说明：支持字符串和数组两种形式的内容格式，用于灵活的消息内容表示
#[derive(Deserialize, Serialize, Clone, ToSchema, Debug)]
#[serde(untagged)]
pub enum ChatMessageContent {
    // 用途：字符串内容
    // 说明：简单的文本消息内容
    String(String),
    // 用途：数组内容
    // 说明：复合消息内容，支持多种类型的内容部分
    Array(Vec<ChatMessageContentPart>),
}

// 用途：聊天消息内容部分结构体
// 说明：用于表示复合消息内容的单个部分，支持文本、图像等不同类型
#[derive(Deserialize, Serialize, Clone, ToSchema, Debug)]
pub struct ChatMessageContentPart {
    // 用途：内容类型
    // 说明：内容部分的类型，如"text"、"image_url"等
    #[serde(rename = "type")]
    pub r#type: String,
    // 用途：文本内容
    // 说明：文本类型的内容部分
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    // 用途：图像URL
    // 说明：图像类型的内容部分，包含URL信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
}

// 用途：图像URL结构体
// 说明：用于表示图像的URL信息
#[derive(Deserialize, Serialize, Clone, ToSchema, Debug)]
pub struct ImageUrl {
    // 用途：URL地址
    // 说明：图像的URL链接
    pub url: String,
    // 用途：可选的图像描述
    // 说明：对图像内容的可选描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// 用途：聊天消息工具调用结构体
// 说明：用于表示聊天消息中的工具调用信息
#[derive(Deserialize, Serialize, Clone, ToSchema, Debug)]
pub struct ChatMessageToolCall {
    // 用途：调用ID
    // 说明：工具调用的唯一标识符
    pub id: String,
    // 用途：函数名称
    // 说明：被调用的函数名称
    pub function: ChatMessageFunction,
    // 用途：工具调用类型
    // 说明：工具调用的类型，固定为"function"
    #[serde(rename = "type")]
    pub r#type: String,
}

// 用途：聊天消息函数结构体
// 说明：用于表示被调用的函数信息
#[derive(Deserialize, Serialize, Clone, ToSchema, Debug)]
pub struct ChatMessageFunction {
    // 用途：函数名称
    // 说明：函数的名称
    pub name: String,
    // 用途：函数参数
    // 说明：传递给函数的参数，JSON格式
    pub arguments: String,
}

// 用途：聊天完成消息结构体
// 说明：用于表示完整的聊天消息，包含角色、内容、工具调用等字段
#[derive(Deserialize, Serialize, Clone, ToSchema, Debug)]
pub struct ChatCompletionMessage {
    // 用途：消息角色
    // 说明：标识消息的发送者角色，如"user"、"assistant"、"system"等
    pub role: String,
    // 用途：消息内容
    // 说明：消息的具体内容，支持字符串和复合格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ChatMessageContent>,
    // 用途：发送者名称
    // 说明：消息发送者的标识名称，可选字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    // 用途：工具调用列表
    // 说明：消息中包含的工具调用，支持函数调用等功能
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatMessageToolCall>>,
    // 用途：工具调用ID
    // 说明：关联的工具调用标识符，用于工具调用的追踪和结果匹配
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    // 用途：拒绝内容
    // 说明：模型拒绝回答时的拒绝理由或内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    // 用途：推理内容
    // 说明：模型的深度思考过程，用于Thinking Mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    // 用途：额外字段
    // 说明：用于接收模型特定的额外字段，如小米模型的reasoning_content等
    #[serde(default = "serde_json::Value::default", flatten)]
    pub extra_fields: serde_json::Value,
}
