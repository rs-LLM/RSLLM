//! 供应商配置表结构
//! 存储AI服务供应商的完整配置信息，支持认证、限流和熔断

// 用途：导入日期时间类型
// 说明：用于记录供应商的创建和更新时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Deserialize, Deserializer, Serialize};
// 用途：导入JSON序列化支持
// 说明：用于配置详情和认证配置的JSON存储
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

// 用途：供应商配置表结构体（扩展版）
// 说明：存储AI服务供应商的完整配置信息，支持认证和限流
#[derive(Clone, Debug, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ProviderConfig {
    // 用途：供应商ID
    // 说明：服务供应商的唯一标识符
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    // 用途：供应商代号
    // 说明：供应商的唯一代号（全英文，可包含"-"分隔符，如"openai"、"azure-openai"）
    pub provider_code: String,
    // 用途：供应商名称
    // 说明：服务供应商的显示名称，如"OpenAI"、"Azure OpenAI"等
    pub name: String,
    // 用途：供应商类型
    // 说明：供应商的类型标识，如"openai"、"claude"、"custom"等
    pub provider_type: String,
    // 用途：API基础地址
    // 说明：供应商API的基础URL地址
    pub api_base: String,
    // 用途：默认API密钥（加密）
    // 说明：供应商的默认API密钥，存储为加密格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    // 用途：认证配置
    // 说明：额外的认证相关配置，JSON格式存储
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_config: Option<serde_json::Value>,
    // 用途：是否启用熔断器
    // 说明：控制是否启用熔断器机制
    #[serde(
        deserialize_with = "deserialize_bool_from_int_or_bool",
        skip_serializing_if = "Option::is_none"
    )]
    pub circuit_breaker_enabled: Option<bool>,
    // 用途：失败阈值
    // 说明：触发熔断器的连续失败次数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_threshold: Option<i32>,
    // 用途：状态
    // 说明：供应商状态，如"active"、"inactive"、"maintenance"、"deprecated"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    // 用途：供应商描述
    // 说明：供应商的详细描述信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // 用途：文档链接
    // 说明：供应商的官方文档链接
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    // 用途：创建时间
    // 说明：记录的创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}

// 用途：生成ProviderConfig的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(ProviderConfig {});

// 用途：ProviderConfig的自定义查询方法
// 说明：提供业务特定的查询方法
impl ProviderConfig {
    /// 根据供应商代号查询配置
    pub async fn select_by_provider_code(
        rb: &rbatis::RBatis,
        provider_code: &str,
    ) -> rbatis::Result<Option<ProviderConfig>> {
        let sql = "SELECT * FROM provider_config WHERE provider_code = ? LIMIT 1";
        rb.query(sql, vec![rbs::Value::String(provider_code.to_string())])
            .await
            .map(|v| {
                if let Some(arr) = v.as_array()
                    && let Some(item) = arr.first()
                {
                    let json_value = serde_json::to_value(item).unwrap_or_default();
                    let config = serde_json::from_value(json_value)
                        .unwrap_or_else(|_| ProviderConfig::default());
                    return Some(config);
                }
                None
            })
    }

    /// 根据名称模糊查询供应商配置
    pub async fn select_by_name_like(
        rb: &rbatis::RBatis,
        name: &str,
    ) -> rbatis::Result<Vec<ProviderConfig>> {
        let sql = "SELECT * FROM provider_config WHERE name LIKE ? ORDER BY created_at DESC";
        rb.query(sql, vec![rbs::Value::String(format!("%{}%", name))])
            .await
            .map(|v| {
                v.as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|item| {
                                // 将 rbs::Value 转换为 serde_json::Value
                                let json_value = serde_json::to_value(item).unwrap_or_default();
                                serde_json::from_value(json_value)
                                    .unwrap_or_else(|_| ProviderConfig::default())
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
    }

    /// 根据供应商类型查询配置列表
    pub async fn select_by_provider_type(
        rb: &rbatis::RBatis,
        provider_type: &str,
    ) -> rbatis::Result<Vec<ProviderConfig>> {
        let sql = "SELECT * FROM provider_config WHERE provider_type = ? ORDER BY created_at DESC";
        rb.query(sql, vec![rbs::Value::String(provider_type.to_string())])
            .await
            .map(|v| {
                v.as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|item| {
                                // 将 rbs::Value 转换为 serde_json::Value
                                let json_value = serde_json::to_value(item).unwrap_or_default();
                                serde_json::from_value(json_value)
                                    .unwrap_or_else(|_| ProviderConfig::default())
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
    }

    /// 分页查询启用的供应商配置
    pub async fn select_enabled_page(
        rb: &rbatis::RBatis,
        page: u64,
        size: u64,
    ) -> rbatis::Result<Vec<ProviderConfig>> {
        let sql = "SELECT * FROM provider_config WHERE status = 'active' LIMIT ? OFFSET ?";
        rb.query(
            sql,
            vec![
                rbs::Value::I64(size as i64),
                rbs::Value::I64(((page - 1) * size) as i64),
            ],
        )
        .await
        .map(|v| {
            v.as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|item| {
                            let json_value = serde_json::to_value(item).unwrap_or_default();
                            serde_json::from_value(json_value)
                                .unwrap_or_else(|_| ProviderConfig::default())
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    }
}
