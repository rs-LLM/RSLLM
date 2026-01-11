// 用途：导入日期时间类型
// 说明：用于记录请求的创建和响应时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Deserialize, Serialize};
// 用途：导入JSON序列化支持
// 说明：用于扩展字段的JSON存储
use serde_json;
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::crud;

// 用途：用量记录表结构体（扩展版）
// 说明：记录每次AI请求的详细用量信息，包括token数量、费用计算、配额扣减等
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHubUsageLog {
    // 用途：记录ID
    // 说明：用量记录的唯一标识符，同时也是请求ID
    pub id: Option<String>,
    // 用途：用户ID
    // 说明：发起请求的用户唯一标识符
    pub user_id: String,
    // 用途：模型ID
    // 说明：使用的AI模型唯一标识符
    pub model_id: String,
    // 用途：API密钥
    // 说明：发起请求使用的API密钥
    pub api_key: String,
    // 用途：输入token数量
    // 说明：请求中输入内容的token数量
    pub input_tokens: i64,
    // 用途：输出token数量
    // 说明：响应中生成内容的token数量
    pub output_tokens: i64,
    // 用途：总token数量
    // 说明：输入和输出token的总和
    pub total_tokens: i64,
    // 用途：输入单价
    // 说明：输入tokens的单价（每1K tokens）
    pub input_price: f64,
    // 用途：输出单价
    // 说明：输出tokens的单价（每1K tokens）
    pub output_price: f64,
    // 用途：输入费用
    // 说明：本次请求的输入费用
    pub input_cost: Option<f64>,
    // 用途：输出费用
    // 说明：本次请求的输出费用
    pub output_cost: Option<f64>,
    // 用途：总费用
    // 说明：本次请求的总费用
    pub total_cost: f64,
    // 用途：请求时间
    // 说明：请求发起的时间
    pub request_time: Option<DateTime>,
    // 用途：响应时间
    // 说明：响应返回的时间
    pub response_time: Option<DateTime>,
    // 用途：货币单位
    // 说明：费用的货币单位，默认为USD
    pub currency: Option<String>,
    // 用途：请求方法
    // 说明：HTTP请求方法，如"POST"、"GET"
    pub request_method: Option<String>,
    // 用途：请求路径
    // 说明：API请求的路径
    pub request_path: Option<String>,
    // 用途：请求头
    // 说明：请求头信息（脱敏处理），JSON格式
    pub request_headers: Option<serde_json::Value>,
    // 用途：请求体
    // 说明：请求体内容（可选），JSON格式
    pub request_body: Option<serde_json::Value>,
    // 用途：HTTP状态码
    // 说明：响应的HTTP状态码
    pub status_code: Option<i32>,
    // 用途：请求类型
    // 说明：请求的类型，如"chat"、"completion"等
    pub request_type: Option<String>,
    // 用途：状态
    // 说明：请求的状态，如"success"、"failed"等
    pub status: Option<String>,
    // 用途：响应时间（毫秒）
    // 说明：请求到响应的总处理时长
    pub response_time_ms: Option<i64>,
    // 用途：错误信息
    // 说明：如果请求失败，记录错误信息
    pub error_message: Option<String>,
    // 用途：扩展信息
    // 说明：额外的扩展信息，JSON格式
    pub extra: Option<serde_json::Value>,
    // 用途：是否已扣减配额
    // 说明：标记是否已经扣减用户配额（0=未扣减，1=已扣减）
    pub quota_deducted: Option<i32>,
    // 用途：配额快照
    // 说明：扣减配额时的配额状态快照，JSON格式
    pub quota_snapshot: Option<serde_json::Value>,
    // 用途：请求IP地址
    // 说明：发起请求的客户端IP地址
    pub ip_address: Option<String>,
    // 用途：用户代理
    // 说明：客户端的用户代理字符串
    pub user_agent: Option<String>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
}

// 用途：生成AiHubUsageLog的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(AiHubUsageLog {});
