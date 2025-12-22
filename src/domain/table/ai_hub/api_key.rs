// 用途：导入日期时间类型
// 说明：用于记录密钥的创建、更新和过期时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入JSON序列化支持
// 说明：用于权限和模型列表的JSON存储
use serde_json;
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：API密钥表结构体
// 说明：用于存储和管理用户的API密钥，支持权限控制和安全策略
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiKey {
    // 用途：密钥ID
    // 说明：API密钥的唯一标识符
    pub id: Option<String>,
    // 用途：密钥哈希
    // 说明：使用SHA256哈希的密钥，用于快速查找，不包含原始密钥信息
    pub key_hash: String,
    // 用途：用户ID
    // 说明：密钥所属用户的唯一标识符
    pub user_id: String,
    // 用途：密钥名称
    // 说明：用于标识密钥用途的友好名称
    pub name: Option<String>,
    // 用途：密钥前缀
    // 说明：用于识别密钥的前缀（如sk-xxx）
    pub prefix: Option<String>,
    // 用途：权限列表
    // 说明：密钥拥有的权限，JSON数组格式
    pub permissions: Option<serde_json::Value>,
    // 用途：允许的模型列表
    // 说明：密钥可以访问的模型列表，JSON数组格式
    pub allowed_models: Option<serde_json::Value>,
    // 用途：禁止的模型列表
    // 说明：密钥禁止访问的模型列表，JSON数组格式
    pub denied_models: Option<serde_json::Value>,
    // 用途：是否启用限流
    // 说明：控制是否对该密钥启用速率限制
    pub rate_limit_enabled: Option<bool>,
    // 用途：每分钟最大请求数
    // 说明：限流配置，每分钟允许的最大请求数
    pub max_requests_per_minute: Option<i32>,
    // 用途：IP白名单
    // 说明：允许访问的IP列表，JSON数组格式
    pub ip_whitelist: Option<serde_json::Value>,
    // 用途：IP黑名单
    // 说明：禁止访问的IP列表，JSON数组格式
    pub ip_blacklist: Option<serde_json::Value>,
    // 用途：是否启用
    // 说明：控制密钥是否可用，true表示启用，false表示禁用
    pub enabled: Option<bool>,
    // 用途：状态
    // 说明：密钥状态，如"active"、"revoked"、"expired"
    pub status: Option<String>,
    // 用途：过期时间
    // 说明：密钥的过期时间戳
    pub expires_at: Option<DateTime>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
    // 用途：最后使用时间
    // 说明：密钥最后一次使用的时间
    pub last_used_at: Option<DateTime>,
}

// 用途：生成ApiKey的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(ApiKey {});

// 用途：扩展方法，用于密钥生成和验证
impl ApiKey {
    /// 用途：生成新的API密钥
    /// 说明：生成格式为 "sk-前缀-随机字符串" 的密钥
    pub fn generate_key(prefix: &str) -> String {
        use rand::Rng;
        let random_part: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        format!("sk-{}-{}", prefix, random_part)
    }
    
    /// 用途：计算密钥哈希
    /// 说明：使用SHA256计算密钥的哈希值，用于存储和验证
    pub fn hash_key(key: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key);
        format!("{:x}", hasher.finalize())
    }
}