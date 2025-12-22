//! 供应商配置表结构
//! 存储AI服务供应商的完整配置信息，支持认证、限流和熔断

// 用途：导入日期时间类型
// 说明：用于记录供应商的创建和更新时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入JSON序列化支持
// 说明：用于配置详情和认证配置的JSON存储
use serde_json;
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::crud;

// 用途：供应商配置表结构体（扩展版）
// 说明：存储AI服务供应商的完整配置信息，支持认证和限流
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    // 用途：供应商ID
    // 说明：服务供应商的唯一标识符
    pub id: Option<String>,
    // 用途：供应商名称
    // 说明：服务供应商的显示名称，如"OpenAI"、"Anthropic"等
    pub name: String,
    // 用途：供应商类型
    // 说明：供应商的类型标识，如"openai"、"claude"、"custom"等
    pub provider_type: String,
    // 用途：API基础地址
    // 说明：供应商API的基础URL地址
    pub api_base: String,
    // 用途：默认API密钥（加密）
    // 说明：供应商的默认API密钥，存储为加密格式
    pub api_key_encrypted: Option<String>,
    // 用途：认证类型
    // 说明：认证方式，如"api_key"、"oauth2"、"header"等
    pub auth_type: Option<String>,
    // 用途：认证配置
    // 说明：额外的认证相关配置，JSON格式存储
    pub auth_config: Option<serde_json::Value>,
    // 用途：默认输入价格（每1K tokens）
    // 说明：供应商的默认输入定价，单位为货币单位
    pub default_input_price: Option<f64>,
    // 用途：默认输出价格（每1K tokens）
    // 说明：供应商的默认输出定价，单位为货币单位
    pub default_output_price: Option<f64>,
    // 用途：是否启用限流
    // 说明：控制是否对供应商启用速率限制
    pub rate_limit_enabled: Option<bool>,
    // 用途：最大并发请求数
    // 说明：限制同时向供应商发送的最大请求数量
    pub max_concurrent_requests: Option<i32>,
    // 用途：是否启用熔断器
    // 说明：控制是否启用熔断器机制
    pub circuit_breaker_enabled: Option<bool>,
    // 用途：失败阈值
    // 说明：触发熔断器的连续失败次数
    pub failure_threshold: Option<i32>,
    // 用途：超时时间（秒）
    // 说明：请求供应商的超时时间设置
    pub timeout_seconds: Option<i32>,
    // 用途：是否启用
    // 说明：控制供应商是否可用，true表示启用，false表示禁用
    pub enabled: Option<bool>,
    // 用途：状态
    // 说明：供应商状态，如"active"、"maintenance"、"deprecated"
    pub status: Option<String>,
    // 用途：供应商描述
    // 说明：供应商的详细描述信息
    pub description: Option<String>,
    // 用途：文档链接
    // 说明：供应商的官方文档链接
    pub documentation_url: Option<String>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成ProviderConfig的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(ProviderConfig {});

// 用途：ProviderConfig的自定义查询方法
// 说明：提供业务特定的查询方法
impl ProviderConfig {
    /// 根据名称模糊查询供应商配置
    pub async fn select_by_name_like(rb: &rbatis::RBatis, name: &str) -> rbatis::Result<Vec<ProviderConfig>> {
        let sql = "SELECT * FROM provider_config WHERE name LIKE ? ORDER BY created_at DESC";
        rb.query(sql, vec![rbs::Value::String(format!("%{}%", name))]).await
            .and_then(|v| {
                let value = v.as_array().map(|arr| {
                    arr.iter().map(|item| {
                        // 将 rbs::Value 转换为 serde_json::Value
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).unwrap_or_else(|_| ProviderConfig::default())
                    }).collect()
                }).unwrap_or_default();
                Ok(value)
            })
    }
    
    /// 根据供应商类型查询配置列表
    pub async fn select_by_provider_type(rb: &rbatis::RBatis, provider_type: &str) -> rbatis::Result<Vec<ProviderConfig>> {
        let sql = "SELECT * FROM provider_config WHERE provider_type = ? ORDER BY created_at DESC";
        rb.query(sql, vec![rbs::Value::String(provider_type.to_string())]).await
            .and_then(|v| {
                let value = v.as_array().map(|arr| {
                    arr.iter().map(|item| {
                        // 将 rbs::Value 转换为 serde_json::Value
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).unwrap_or_else(|_| ProviderConfig::default())
                    }).collect()
                }).unwrap_or_default();
                Ok(value)
            })
    }
    
    /// 分页查询启用的供应商配置
    pub async fn select_enabled_page(rb: &rbatis::RBatis, page: u64, size: u64) -> rbatis::Result<Vec<ProviderConfig>> {
        let sql = "SELECT * FROM provider_config WHERE enabled = true LIMIT ? OFFSET ?";
        rb.query(sql, vec![rbs::Value::I64(size as i64), rbs::Value::I64(((page - 1) * size) as i64)]).await
            .and_then(|v| {
                let value = v.as_array().map(|arr| {
                    arr.iter().map(|item| {
                        // 将 rbs::Value 转换为 serde_json::Value
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).unwrap_or_else(|_| ProviderConfig::default())
                    }).collect()
                }).unwrap_or_default();
                Ok(value)
            })
    }
}