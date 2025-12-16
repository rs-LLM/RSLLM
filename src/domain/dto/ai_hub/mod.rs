// 用途：导出AI Hub DTO相关模块

// 用途：导出聊天完成相关DTO
// 说明：包含聊天对话请求和推理配置等DTO
pub mod chat;

// 用途：导出文本完成相关DTO
// 说明：包含文本生成请求等DTO
pub mod completion;

// 用途：导出消息内容相关DTO
// 说明：包含聊天消息、角色定义、内容格式等DTO
pub mod content;

// 用途：导出向量嵌入相关DTO
// 说明：包含嵌入请求等DTO
pub mod embeddings;

// 用途：导出日志概率相关DTO
// 说明：包含日志概率统计、令牌信息等DTO
pub mod logprob;

// 用途：导出响应格式相关DTO
// 说明：包含不同响应格式的定义和配置
pub mod response_format;

// 用途：导出流式处理相关DTO
// 说明：包含流式响应数据、分块处理等DTO
pub mod streaming;
