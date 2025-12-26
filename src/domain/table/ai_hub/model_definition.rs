// 用途：导入日期时间类型
// 说明：用于记录模型的创建和更新时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize, Deserializer};
// 用途：导入JSON序列化支持
// 说明：用于配置详情和能力列表的JSON存储
use serde_json;
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::crud;
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

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

// 用途：AI模型定义表结构体（扩展版）
// 说明：存储AI模型的完整配置信息，支持动态定价和限流
#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct ModelDefinition {
    // 用途：模型ID
    // 说明：模型的唯一标识符
    pub id: Option<String>,
    // 用途：模型键名
    // 说明：模型的标识符，如"gpt-4-turbo"、"claude-3-sonnet"等
    pub key: String,
    // 用途：模型显示名称
    // 说明：模型的友好显示名称
    pub name: String,
    // 用途：模型类型
    // 说明：模型的类型分类，如"chat"、"completion"、"embedding"等
    pub model_type: String,
    // 用途：供应商ID
    // 说明：关联的服务提供商ID，指向provider_config表的外键
    pub provider_id: String,
    // 用途：自定义API地址
    // 说明：可选的自定义API端点地址
    pub api_endpoint: Option<String>,
    // 用途：加密的API密钥
    // 说明：模型专用的加密API密钥（可选，如果未提供则使用供应商默认）
    pub api_key_encrypted: Option<String>,
    // 用途：输入价格（每1K tokens）
    // 说明：输入tokens的价格，单位为货币单位
    pub input_price: f64,
    // 用途：输出价格（每1K tokens）
    // 说明：输出tokens的价格，单位为货币单位
    pub output_price: f64,
    // 用途：货币单位
    // 说明：价格的货币单位，默认为USD
    pub currency: Option<String>,
    // 用途：单次请求最大tokens
    // 说明：限制单次请求的最大tokens数量
    pub max_tokens_per_request: Option<i32>,
    // 用途：每分钟请求数限制
    // 说明：限制每分钟的最大请求数量
    pub max_requests_per_minute: Option<i32>,
    // 用途：是否启用
    // 说明：控制模型是否可用，true表示启用，false表示禁用
    #[serde(deserialize_with = "deserialize_bool_from_int_or_bool")]
    pub enabled: Option<bool>,
    // 用途：状态
    // 说明：模型状态，如"active"、"maintenance"、"deprecated"
    pub status: Option<String>,
    // 用途：模型描述
    // 说明：模型的详细描述信息
    pub description: Option<String>,
    // 用途：支持的能力列表
    // 说明：模型支持的功能，如["streaming", "function_calling", "json_mode"]，JSON格式存储
    pub capabilities: Option<serde_json::Value>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成ModelDefinition的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(ModelDefinition {});

// 用途：ModelDefinition的自定义查询方法
// 说明：提供业务特定的查询方法
impl ModelDefinition {
    /// 根据key查询模型（支持模糊搜索）
    pub async fn select_by_key_like(rb: &rbatis::RBatis, key: &str) -> rbatis::Result<Vec<ModelDefinition>> {
        let sql = "SELECT * FROM model_definition WHERE key LIKE ? ORDER BY created_at DESC";
        rb.query(sql, vec![rbs::Value::String(format!("%{}%", key))]).await
            .and_then(|v| {
                let value = v.as_array().map(|arr| {
                    arr.iter().map(|item| {
                        // 将 rbs::Value 转换为 serde_json::Value
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).unwrap_or_else(|_| ModelDefinition::default())
                    }).collect()
                }).unwrap_or_default();
                Ok(value)
            })
    }
    
    /// 根据供应商ID查询模型列表
    pub async fn select_by_provider_id(rb: &rbatis::RBatis, provider_id: &str) -> rbatis::Result<Vec<ModelDefinition>> {
        let sql = "SELECT * FROM model_definition WHERE provider_id = ? ORDER BY created_at DESC";
        rb.query(sql, vec![rbs::Value::String(provider_id.to_string())]).await
            .and_then(|v| {
                let value = v.as_array().map(|arr| {
                    arr.iter().map(|item| {
                        // 将 rbs::Value 转换为 serde_json::Value
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).unwrap_or_else(|_| ModelDefinition::default())
                    }).collect()
                }).unwrap_or_default();
                Ok(value)
            })
    }
    
    /// 分页查询启用的模型
    pub async fn select_enabled_page(rb: &rbatis::RBatis, page: u64, size: u64) -> rbatis::Result<Vec<ModelDefinition>> {
        let sql = "SELECT * FROM model_definition WHERE enabled = true LIMIT ? OFFSET ?";
        rb.query(sql, vec![rbs::Value::I64(size as i64), rbs::Value::I64(((page - 1) * size) as i64)]).await
            .and_then(|v| {
                let value = v.as_array().map(|arr| {
                    arr.iter().map(|item| {
                        // 将 rbs::Value 转换为 serde_json::Value
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).unwrap_or_else(|_| ModelDefinition::default())
                    }).collect()
                }).unwrap_or_default();
                Ok(value)
            })
    }
}