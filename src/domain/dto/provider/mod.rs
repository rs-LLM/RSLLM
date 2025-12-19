// 供应商类型相关的数据传输对象
use serde_json::Value;
use std::borrow::Cow;

use crate::domain::table::provider::Provider as DbProvider;

/// 供应商类型枚举，定义支持的供应商类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI供应商
    OpenAI,
    /// Anthropic供应商
    Anthropic,
    /// Azure OpenAI供应商
    Azure,
    /// AWS Bedrock供应商
    Bedrock,
    /// Google Vertex AI供应商
    VertexAI,
    /// 自定义OpenAI兼容供应商
    Custom,
}

impl ProviderType {
    /// 从字符串转换为ProviderType枚举
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "anthropic" => Some(Self::Anthropic),
            "azure" => Some(Self::Azure),
            "bedrock" => Some(Self::Bedrock),
            "vertexai" => Some(Self::VertexAI),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
    
    /// 转换为字符串
    pub fn to_string(&self) -> String {
        match self {
            Self::OpenAI => "openai".to_string(),
            Self::Anthropic => "anthropic".to_string(),
            Self::Azure => "azure".to_string(),
            Self::Bedrock => "bedrock".to_string(),
            Self::VertexAI => "vertexai".to_string(),
            Self::Custom => "custom".to_string(),
        }
    }
}

/// 映射供应商类型到标准化的厂商名称，用于日志和监控
pub fn get_vendor_name(provider_type: &ProviderType) -> Cow<'static, str> {
    match provider_type {
        ProviderType::OpenAI => Cow::Borrowed("openai"),
        ProviderType::Anthropic => Cow::Borrowed("Anthropic"),
        ProviderType::Azure => Cow::Borrowed("Azure"),
        ProviderType::Bedrock => Cow::Borrowed("AWS"),
        ProviderType::VertexAI => Cow::Borrowed("Google"),
        ProviderType::Custom => Cow::Borrowed("Custom"),
    }
}

/// 供应商配置结构，用于创建供应商实例
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// 供应商ID
    pub id: String,
    /// 供应商名称
    pub name: String,
    /// 供应商类型
    pub provider_type: ProviderType,
    /// 配置详情
    pub config_details: Value,
}

impl From<&DbProvider> for ProviderConfig {
    /// 从数据库Provider模型转换为ProviderConfig
    fn from(provider: &DbProvider) -> Self {
        let provider_type = ProviderType::from_str(&provider.provider_type)
            .unwrap_or(ProviderType::Custom);
        
        Self {
            id: provider.id.clone().unwrap_or_default(),
            name: provider.name.clone(),
            provider_type,
            config_details: provider.config_details.clone(),
        }
    }
}
