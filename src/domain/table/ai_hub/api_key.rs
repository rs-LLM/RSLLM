// 用途：导入日期时间类型
// 说明：用于记录密钥的创建、更新和过期时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Deserialize, Deserializer, Serialize};
// 用途：导入JSON序列化支持
// 说明：用于模型速率限制配置的JSON存储
use serde_json;
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::crud;

// 用途：自定义反序列化函数，支持从整数或布尔值反序列化为布尔类型
// 说明：兼容数据库中存储的整数1/0和布尔值true/false
fn deserialize_bool_from_int_or_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    // 使用serde_json::Value作为中间层
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

    match value {
        serde_json::Value::Bool(b) => Ok(Some(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Some(i != 0))
            } else {
                Ok(None)
            }
        }
        serde_json::Value::Null => Ok(None),
        _ => Ok(None),
    }
}

// 用途：API密钥表结构体
// 说明：用于存储和管理用户的API密钥，支持额度控制
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ApiKey {
    // 用途：密钥ID
    // 说明：API密钥的唯一标识符
    pub id: Option<String>,
    // 用途：密钥
    // 说明：完整的API密钥，用于显示和管理
    pub key: String,
    // 用途：密钥哈希
    // 说明：使用SHA256哈希的密钥，用于快速查找和验证
    pub key_hash: String,
    // 用途：用户ID
    // 说明：密钥所属用户的唯一标识符
    pub user_id: String,
    // 用途：密钥名称
    // 说明：用于标识密钥用途的友好名称
    pub name: Option<String>,
    // 用途：密钥描述
    // 说明：用于描述密钥用途的详细说明
    pub description: Option<String>,
    // 用途：密钥前缀
    // 说明：用于识别密钥的前缀（如sk-xxx）
    pub prefix: Option<String>,
    // 用途：额度限制
    // 说明：API密钥的额度限制值
    pub quota_limit: Option<f64>,
    // 用途：额度周期
    // 说明：额度计算周期，如"daily"、"monthly"
    pub quota_period: Option<String>,
    // 用途：已使用额度
    // 说明：当前周期内已使用的额度
    pub quota_used: Option<f64>,
    // 用途：额度重置时间
    // 说明：额度重置的时间戳
    pub quota_reset_time: Option<DateTime>,
    // 用途：最后使用时间
    // 说明：密钥最后一次使用的时间
    pub last_used_time: Option<DateTime>,
    // 用途：是否启用
    // 说明：控制密钥是否可用，true表示启用，false表示禁用
    #[serde(deserialize_with = "deserialize_bool_from_int_or_bool")]
    pub enabled: Option<bool>,
    // 用途：状态
    // 说明：密钥状态，如"active"、"revoked"、"expired"
    pub status: Option<String>,
    // 用途：是否启用速率限制
    // 说明：控制是否对该密钥启用速率限制功能
    #[serde(deserialize_with = "deserialize_bool_from_int_or_bool")]
    pub rate_limit_enabled: Option<bool>,
    // 用途：每分钟最大请求数
    // 说明：该密钥每分钟允许的最大请求数量
    pub max_requests_per_minute: Option<i64>,
    // 用途：每分钟最大令牌数
    // 说明：该密钥每分钟允许的最大令牌数量
    pub max_tokens_per_minute: Option<i64>,
    // 用途：模型速率限制配置
    // 说明：针对不同模型的速率限制配置，JSON格式存储
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_rate_limits: Option<serde_json::Value>,
    // 用途：权限配置
    // 说明：API Key的权限配置，JSON格式存储
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<serde_json::Value>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成ApiKey的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(ApiKey {});

// 用途：扩展方法，用于密钥生成和验证
impl ApiKey {
    /// 用途：生成新的API密钥
    /// 说明：生成格式为 "sk-" + 64位十六进制字符串的密钥
    pub fn generate_key() -> String {
        let random_bytes: [u8; 32] = rand::random();
        let hex_part: String = random_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        format!("sk-{}", hex_part)
    }

    /// 用途：计算密钥哈希
    /// 说明：使用SHA256计算密钥的哈希值，用于存储和验证
    pub fn hash_key(key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(key);
        format!("{:x}", hasher.finalize())
    }
}
