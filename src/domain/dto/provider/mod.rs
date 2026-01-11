// 供应商类型相关的数据传输对象
use serde_json::Value;
use std::borrow::Cow;

/// 供应商类型枚举，定义支持的供应商类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI供应商
    OpenAI,
    /// OpenAI兼容供应商
    OpenAICompatible,
}

impl ProviderType {
    /// 从字符串转换为ProviderType枚举
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "openai-compatible" => Some(Self::OpenAICompatible),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::OpenAICompatible => write!(f, "openai-compatible"),
        }
    }
}

/// 映射供应商类型到标准化的厂商名称，用于日志和监控
pub fn get_vendor_name(provider_type: &ProviderType) -> Cow<'static, str> {
    match provider_type {
        ProviderType::OpenAI => Cow::Borrowed("OpenAI"),
        ProviderType::OpenAICompatible => Cow::Borrowed("OpenAICompatible"),
    }
}

/// 供应商配置结构，用于创建供应商实例
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// 供应商ID
    pub id: String,
    /// 供应商代号
    pub provider_code: String,
    /// 供应商名称
    pub name: String,
    /// 供应商类型
    pub provider_type: ProviderType,
    /// API基础地址
    pub api_base: String,
    /// API密钥（加密）
    pub api_key_encrypted: Option<String>,
    /// 认证配置
    pub auth_config: Option<serde_json::Value>,
    /// 配置详情
    pub config_details: Value,
    /// 状态
    pub status: Option<String>,
    /// 文档链接
    pub documentation_url: Option<String>,
}

impl From<&crate::domain::table::ai_hub::provider_config::ProviderConfig> for ProviderConfig {
    /// 从数据库ProviderConfig模型转换为ProviderConfig
    fn from(provider: &crate::domain::table::ai_hub::provider_config::ProviderConfig) -> Self {
        let provider_type =
            ProviderType::parse(&provider.provider_type).unwrap_or(ProviderType::OpenAICompatible);

        Self {
            id: provider.id.clone().unwrap_or_default(),
            provider_code: provider.provider_code.clone(),
            name: provider.name.clone(),
            provider_type,
            api_base: provider.api_base.clone(),
            api_key_encrypted: provider.api_key_encrypted.clone(),
            auth_config: provider.auth_config.clone(),
            config_details: provider.auth_config.clone().unwrap_or_default(),
            status: provider.status.clone(),
            documentation_url: provider.documentation_url.clone(),
        }
    }
}
