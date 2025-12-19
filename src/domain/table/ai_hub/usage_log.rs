// 用途：导入日期时间类型
// 说明：用于记录请求的创建和响应时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入JSON序列化支持
// 说明：用于扩展字段的JSON存储
use serde_json;
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：用量记录表结构体
// 说明：记录每次AI请求的详细用量信息，包括token数量、费用计算等
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHubUsageLog {
    // 用途：记录ID
    // 说明：用量记录的唯一标识符
    pub id: Option<String>,
    // 用途：请求ID
    // 说明：每次AI请求的唯一标识符，用于追踪和关联
    pub request_id: Option<String>,
    // 用途：用户ID
    // 说明：发起请求的用户唯一标识符
    pub user_id: Option<String>,
    // 用途：模型ID
    // 说明：使用的AI模型唯一标识符
    pub model_id: Option<String>,
    // 用途：提供商ID
    // 说明：AI服务提供商的唯一标识符
    pub provider_id: Option<String>,
    // 用途：输入token数量
    // 说明：请求中输入内容的token数量
    pub input_tokens: Option<i64>,
    // 用途：输出token数量
    // 说明：响应中生成内容的token数量
    pub output_tokens: Option<i64>,
    // 用途：总token数量
    // 说明：输入和输出token的总和
    pub total_tokens: Option<i64>,
    // 用途：输入费用（分）
    // 说明：输入token产生的费用，单位为分
    pub input_cost: Option<f64>,
    // 用途：输出费用（分）
    // 说明：输出token产生的费用，单位为分
    pub output_cost: Option<f64>,
    // 用途：总费用（分）
    // 说明：总费用，单位为分
    pub total_cost: Option<f64>,
    // 用途：请求时间
    // 说明：请求发送的时间戳
    pub request_time: Option<DateTime>,
    // 用途：响应时间
    // 说明：响应接收的时间戳
    pub response_time: Option<DateTime>,
    // 用途：处理时长（毫秒）
    // 说明：请求到响应的总处理时长
    pub duration_ms: Option<i64>,
    // 用途：请求类型
    // 说明：请求的类型，如"chat"、"completion"、"embedding"等
    pub request_type: String,
    // 用途：状态
    // 说明：请求处理状态，如"success"、"failed"、"timeout"等
    pub status: String,
    // 用途：扩展字段
    // 说明：存储额外的请求和响应信息，如模型参数、错误信息等
    pub extra: Option<serde_json::Value>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成AiHubUsageLog的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(AiHubUsageLog {});