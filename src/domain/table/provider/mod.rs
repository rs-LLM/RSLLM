// 用途：导入日期时间类型
// 说明：用于记录服务提供商的创建和更新时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Deserialize, Deserializer, Serialize};
// 用途：导入JSON序列化支持
// 说明：用于配置详情和模型参数的JSON存储
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

// 用途：AI服务提供商配置表结构体
// 说明：用于存储各种AI服务提供商的基础配置信息，如OpenAI、Claude等
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provider {
    // 用途：提供商ID
    // 说明：服务提供商的唯一标识符，用于区分不同的AI服务提供商
    pub id: Option<String>,
    // 用途：提供商名称
    // 说明：服务提供商的显示名称，如"OpenAI"、"Anthropic"等
    pub name: String,
    // 用途：提供商类型
    // 说明：服务提供商的类型标识，存储为字符串，对应ProviderType枚举概念
    pub provider_type: String,
    // 用途：配置详情
    // 说明：存储提供商的详细配置信息，包括API密钥、端点URL等，JSON格式存储
    pub config_details: serde_json::Value,
    // 用途：是否启用
    // 说明：控制当前提供商配置是否可用，true表示启用，false表示禁用
    // 支持从整数或布尔值反序列化，兼容数据库中的整数存储
    #[serde(deserialize_with = "deserialize_bool_from_int_or_bool")]
    pub enabled: Option<bool>,
    // 用途：基础定价（每1000 tokens）
    // 说明：供应商的基础定价，单位为分/1000 tokens
    pub base_price: Option<f64>,
    // 用途：上下文定价（每1000 tokens）
    // 说明：上下文 tokens 的定价，单位为分/1000 tokens
    pub context_price: Option<f64>,
    // 用途：输出定价（每1000 tokens）
    // 说明：输出 tokens 的定价，单位为分/1000 tokens
    pub output_price: Option<f64>,
    // 用途：创建时间
    // 说明：记录当前提供商配置的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录当前提供商配置的最后修改时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成Provider的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(Provider {});

// 用途：AI模型定义表结构体
// 说明：用于存储AI模型的基础信息和配置参数
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelDefinition {
    // 用途：模型ID
    // 说明：模型的唯一标识符，用于区分不同的AI模型
    pub id: Option<String>,
    // 用途：模型键名
    // 说明：模型的标识键，如"gpt-4"、"claude-3-sonnet"等
    pub key: String,
    // 用途：模型类型
    // 说明：模型的类型分类，如"chat"、"completion"、"embedding"等
    pub model_type: String,
    // 用途：提供商ID
    // 说明：关联的服务提供商ID，指向Provider表的外键
    pub provider_id: Option<String>,
    // 用途：配置详情
    // 说明：模型的特定配置参数，如温度、最大tokens等，JSON格式存储
    pub config_details: Option<serde_json::Value>,
    // 用途：是否启用
    // 说明：控制当前模型是否可用，true表示启用，false表示禁用
    #[serde(deserialize_with = "deserialize_bool_from_int_or_bool")]
    pub enabled: Option<bool>,
    // 用途：创建时间
    // 说明：记录模型的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录模型的最后修改时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成ModelDefinition的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(ModelDefinition {});

// 用途：AI处理管道表结构体
// 说明：用于存储AI处理流程的配置信息，支持多步骤处理
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pipeline {
    // 用途：管道ID
    // 说明：管道的唯一标识符，用于区分不同的处理管道
    pub id: Option<String>,
    // 用途：管道名称
    // 说明：管道的显示名称，用于标识管道功能
    pub name: String,
    // 用途：管道类型
    // 说明：管道的类型分类，如"chat"、"completion"、"custom"等
    pub pipeline_type: String,
    // 用途：管道描述
    // 说明：管道的功能描述，说明管道的用途和处理流程
    pub description: Option<String>,
    // 用途：是否启用
    // 说明：控制当前管道是否可用，true表示启用，false表示禁用
    #[serde(deserialize_with = "deserialize_bool_from_int_or_bool")]
    pub enabled: Option<bool>,
    // 用途：创建时间
    // 说明：记录管道的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录管道的最后修改时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成Pipeline的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(Pipeline {});

// 用途：管道插件配置表结构体
// 说明：用于存储管道中各个插件的具体配置信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelinePluginConfig {
    // 用途：插件配置ID
    // 说明：插件配置的唯一标识符
    pub id: Option<String>,
    // 用途：管道ID
    // 说明：关联的管道ID，指向Pipeline表的外键
    pub pipeline_id: Option<String>,
    // 用途：插件类型
    // 说明：插件的类型标识，如"preprocessor"、"model"、"postprocessor"等
    pub plugin_type: String,
    // 用途：配置数据
    // 说明：插件的详细配置参数，JSON格式存储，支持复杂的配置结构
    pub config_data: serde_json::Value,
    // 用途：是否启用
    // 说明：控制当前插件是否可用，true表示启用，false表示禁用
    #[serde(deserialize_with = "deserialize_bool_from_int_or_bool")]
    pub enabled: Option<bool>,
    // 用途：管道中顺序
    // 说明：插件在管道中的执行顺序，数值越小越先执行
    pub order_in_pipeline: Option<i32>,
    // 用途：创建时间
    // 说明：记录插件配置的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录插件配置的最后修改时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成PipelinePluginConfig的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(PipelinePluginConfig {});
