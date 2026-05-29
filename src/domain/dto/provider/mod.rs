//! Provider DTO module.
//! Defines provider-related DTO types and conversion helpers used by provider configuration flows.
use serde_json::Value;
use std::borrow::Cow;

/// 供应商类型枚举，定义支持的供应商类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI供应商
    OpenAI,
    /// OpenAI兼容供应商
    OpenAICompatible,
    /// Codex (OpenAI ChatGPT OAuth)
    Codex,
    /// Anthropic Claude
    Claude,
    /// Google Gemini API
    Gemini,
    /// Google Gemini CLI (gcloud auth)
    GeminiCli,
    /// Google Vertex AI
    Vertex,
    /// Google AI Studio
    AiStudio,
    /// Alibaba Qwen (通义千问)
    Qwen,
    /// Moonshot Kimi
    Kimi,
    /// iFlow
    Iflow,
    /// Antigravity
    Antigravity,
    /// 自定义供应商
    Custom,
}

impl ProviderType {
    /// 从字符串转换为ProviderType枚举
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "openai-compatible" => Some(Self::OpenAICompatible),
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            "gemini" => Some(Self::Gemini),
            "gemini-cli" => Some(Self::GeminiCli),
            "vertex" => Some(Self::Vertex),
            "aistudio" => Some(Self::AiStudio),
            "qwen" => Some(Self::Qwen),
            "kimi" => Some(Self::Kimi),
            "iflow" => Some(Self::Iflow),
            "antigravity" => Some(Self::Antigravity),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::OpenAICompatible => write!(f, "openai-compatible"),
            Self::Codex => write!(f, "codex"),
            Self::Claude => write!(f, "claude"),
            Self::Gemini => write!(f, "gemini"),
            Self::GeminiCli => write!(f, "gemini-cli"),
            Self::Vertex => write!(f, "vertex"),
            Self::AiStudio => write!(f, "aistudio"),
            Self::Qwen => write!(f, "qwen"),
            Self::Kimi => write!(f, "kimi"),
            Self::Iflow => write!(f, "iflow"),
            Self::Antigravity => write!(f, "antigravity"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// 映射供应商类型到标准化的厂商名称，用于日志和监控
pub fn get_vendor_name(provider_type: &ProviderType) -> Cow<'static, str> {
    match provider_type {
        ProviderType::OpenAI => Cow::Borrowed("OpenAI"),
        ProviderType::OpenAICompatible => Cow::Borrowed("OpenAICompatible"),
        ProviderType::Codex => Cow::Borrowed("Codex"),
        ProviderType::Claude => Cow::Borrowed("Claude"),
        ProviderType::Gemini => Cow::Borrowed("Gemini"),
        ProviderType::GeminiCli => Cow::Borrowed("GeminiCLI"),
        ProviderType::Vertex => Cow::Borrowed("VertexAI"),
        ProviderType::AiStudio => Cow::Borrowed("AIStudio"),
        ProviderType::Qwen => Cow::Borrowed("Qwen"),
        ProviderType::Kimi => Cow::Borrowed("Kimi"),
        ProviderType::Iflow => Cow::Borrowed("iFlow"),
        ProviderType::Antigravity => Cow::Borrowed("Antigravity"),
        ProviderType::Custom => Cow::Borrowed("Custom"),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn build_db_provider(
        provider_type: &str,
    ) -> crate::domain::table::ai_hub::provider_config::ProviderConfig {
        crate::domain::table::ai_hub::provider_config::ProviderConfig {
            id: Some("p1".to_string()),
            provider_code: "custom-code".to_string(),
            name: "Custom Provider".to_string(),
            provider_type: provider_type.to_string(),
            api_base: "https://example.com/v1".to_string(),
            api_key_encrypted: None,
            auth_config: Some(serde_json::json!({"k": "v"})),
            circuit_breaker_enabled: None,
            failure_threshold: None,
            status: Some("active".to_string()),
            description: None,
            documentation_url: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn parse_should_support_custom() {
        assert_eq!(ProviderType::parse("custom"), Some(ProviderType::Custom));
    }

    #[test]
    fn parse_should_support_codex() {
        assert_eq!(ProviderType::parse("codex"), Some(ProviderType::Codex));
    }

    #[test]
    fn parse_should_support_all_new_provider_types() {
        assert_eq!(ProviderType::parse("claude"), Some(ProviderType::Claude));
        assert_eq!(ProviderType::parse("gemini"), Some(ProviderType::Gemini));
        assert_eq!(
            ProviderType::parse("gemini-cli"),
            Some(ProviderType::GeminiCli)
        );
        assert_eq!(ProviderType::parse("vertex"), Some(ProviderType::Vertex));
        assert_eq!(
            ProviderType::parse("aistudio"),
            Some(ProviderType::AiStudio)
        );
        assert_eq!(ProviderType::parse("qwen"), Some(ProviderType::Qwen));
        assert_eq!(ProviderType::parse("kimi"), Some(ProviderType::Kimi));
        assert_eq!(ProviderType::parse("iflow"), Some(ProviderType::Iflow));
        assert_eq!(
            ProviderType::parse("antigravity"),
            Some(ProviderType::Antigravity)
        );
    }

    #[test]
    fn parse_should_trim_and_support_case_insensitive_input() {
        assert_eq!(
            ProviderType::parse("  OpenAI-Compatible  "),
            Some(ProviderType::OpenAICompatible)
        );
        assert_eq!(
            ProviderType::parse("\n\tCuStOm\t\n"),
            Some(ProviderType::Custom)
        );
        assert_eq!(ProviderType::parse("  CoDeX  "), Some(ProviderType::Codex));
        assert_eq!(
            ProviderType::parse("  CLAUDE  "),
            Some(ProviderType::Claude)
        );
        assert_eq!(
            ProviderType::parse("  GEMINI-CLI  "),
            Some(ProviderType::GeminiCli)
        );
    }

    #[test]
    fn display_should_render_custom() {
        assert_eq!(ProviderType::Custom.to_string(), "custom");
    }

    #[test]
    fn display_should_render_codex() {
        assert_eq!(ProviderType::Codex.to_string(), "codex");
    }

    #[test]
    fn display_should_render_all_new_provider_types() {
        assert_eq!(ProviderType::Claude.to_string(), "claude");
        assert_eq!(ProviderType::Gemini.to_string(), "gemini");
        assert_eq!(ProviderType::GeminiCli.to_string(), "gemini-cli");
        assert_eq!(ProviderType::Vertex.to_string(), "vertex");
        assert_eq!(ProviderType::AiStudio.to_string(), "aistudio");
        assert_eq!(ProviderType::Qwen.to_string(), "qwen");
        assert_eq!(ProviderType::Kimi.to_string(), "kimi");
        assert_eq!(ProviderType::Iflow.to_string(), "iflow");
        assert_eq!(ProviderType::Antigravity.to_string(), "antigravity");
    }

    #[test]
    fn vendor_name_should_map_custom() {
        assert_eq!(get_vendor_name(&ProviderType::Custom), "Custom");
    }

    #[test]
    fn vendor_name_should_map_codex() {
        assert_eq!(get_vendor_name(&ProviderType::Codex), "Codex");
    }

    #[test]
    fn vendor_name_should_map_all_new_provider_types() {
        assert_eq!(get_vendor_name(&ProviderType::Claude), "Claude");
        assert_eq!(get_vendor_name(&ProviderType::Gemini), "Gemini");
        assert_eq!(get_vendor_name(&ProviderType::GeminiCli), "GeminiCLI");
        assert_eq!(get_vendor_name(&ProviderType::Vertex), "VertexAI");
        assert_eq!(get_vendor_name(&ProviderType::AiStudio), "AIStudio");
        assert_eq!(get_vendor_name(&ProviderType::Qwen), "Qwen");
        assert_eq!(get_vendor_name(&ProviderType::Kimi), "Kimi");
        assert_eq!(get_vendor_name(&ProviderType::Iflow), "iFlow");
        assert_eq!(get_vendor_name(&ProviderType::Antigravity), "Antigravity");
    }

    #[test]
    fn from_db_should_preserve_custom_provider_type() {
        let db_provider = build_db_provider("custom");

        let dto_provider = ProviderConfig::try_from(&db_provider).expect("custom should parse");

        assert_eq!(dto_provider.provider_type, ProviderType::Custom);
    }

    #[test]
    fn from_db_should_preserve_claude_provider_type() {
        let db_provider = build_db_provider("claude");
        let dto_provider = ProviderConfig::try_from(&db_provider).expect("claude should parse");
        assert_eq!(dto_provider.provider_type, ProviderType::Claude);
    }

    #[test]
    fn from_db_should_reject_invalid_provider_type() {
        let db_provider = build_db_provider("unknown-provider");

        let result = ProviderConfig::try_from(&db_provider);

        assert!(result.is_err());
    }

    #[test]
    fn from_db_should_accept_provider_type_with_surrounded_whitespace() {
        let db_provider = build_db_provider("  OpenAI-Compatible  ");

        let dto_provider = ProviderConfig::try_from(&db_provider)
            .expect("provider_type with whitespace should parse");

        assert_eq!(dto_provider.provider_type, ProviderType::OpenAICompatible);
    }
}

impl TryFrom<&crate::domain::table::ai_hub::provider_config::ProviderConfig> for ProviderConfig {
    type Error = String;

    /// 从数据库ProviderConfig模型转换为ProviderConfig
    fn try_from(
        provider: &crate::domain::table::ai_hub::provider_config::ProviderConfig,
    ) -> Result<Self, Self::Error> {
        let provider_type =
            ProviderType::parse(provider.provider_type.as_str()).ok_or_else(|| {
                format!(
                    "Invalid provider_type in provider_config: {}",
                    provider.provider_type
                )
            })?;

        Ok(Self {
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
        })
    }
}
