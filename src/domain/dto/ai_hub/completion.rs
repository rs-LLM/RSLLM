// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入哈希映射类型
// 说明：用于存储logit_bias等键值对数据
use std::collections::HashMap;
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：文本完成请求结构体
// 说明：用于表示文本生成的请求参数，包含所有可配置的选项
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct CompletionRequest {
    // 用途：模型名称
    // 说明：用于生成文本的AI模型标识符
    pub model: String,
    // 用途：提示文本
    // 说明：用于引导AI生成内容的输入提示
    pub prompt: String,
    // 用途：后缀文本
    // 说明：添加到生成文本末尾的可选后缀
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    // 用途：最大令牌数
    // 说明：生成文本允许的最大token数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    // 用途：温度参数
    // 说明：控制生成内容的随机性，值越高越随机
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    // 用途：核采样参数
    // 说明：控制词汇选择的多样性，与temperature配合使用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    // 用途：生成数量
    // 说明：并行生成的文本数量，用于多样化输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    // 用途：流式输出
    // 说明：是否启用流式响应，实时返回生成的文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    // 用途：日志概率数量
    // 说明：返回每个位置顶级候选token的数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<u32>,
    // 用途：回显提示
    // 说明：是否在输出中包含原始提示文本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>,
    // 用途：停止词列表
    // 说明：遇到这些词汇时停止生成
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    // 用途：存在惩罚
    // 说明：减少内容重复的参数，数值越高越避免重复
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    // 用途：频率惩罚
    // 说明：根据词汇出现频率调整生成概率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    // 用途：最佳结果数
    // 说明：从生成的文本中选取最佳结果的候选数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<u32>,
    // 用途：对数偏差
    // 说明：手动调整特定词汇的选择概率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, i32>>,
    // 用途：用户标识
    // 说明：可选的用户标识符，用于跟踪和审计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}
