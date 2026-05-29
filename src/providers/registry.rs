// 供应商注册表实现
// 用于管理和注册所有AI服务供应商

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::dto::ProviderConfig;
use crate::domain::table::ai_hub::provider_config::ProviderConfig as DbProviderConfig;
use crate::providers::provider::Provider;
use crate::providers::{
    aistudio, antigravity, claude, codex, custom, gemini, gemini_cli, iflow, kimi, openai,
    openai_compatible, qwen, vertex,
};

/// 供应商注册表，用于管理和注册所有AI服务供应商
///
/// # 功能特性
/// - 基于 provider_code 的动态注册机制
/// - 支持运行时添加、移除和更新供应商
/// - 提供基于 provider_code 的快速查询
/// - 支持供应商状态管理（active/inactive/maintenance）
pub struct ProviderRegistry {
    /// 供应商实例映射，使用 provider_code 作为键
    providers: HashMap<String, Arc<dyn Provider>>,
    /// 供应商配置映射，使用 provider_code 作为键
    configs: HashMap<String, ProviderConfig>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistry {
    /// 创建一个新的供应商注册表
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// 从数据库配置创建一个新的供应商注册表
    ///
    /// # 参数
    /// * `providers` - 数据库中的供应商配置列表
    /// * `encryption_service` - 可选的加密服务，用于解密API密钥
    ///
    /// # 返回
    /// 包含所有活跃供应商的注册表实例
    pub fn from_db_with_encryption(
        providers: &[DbProviderConfig],
        encryption_service: Option<&crate::service::ai_hub::EncryptionService>,
    ) -> Self {
        let mut registry = Self::new();

        log::info!(
            "[ProviderRegistry] Starting to load {} providers from database",
            providers.len()
        );

        for provider in providers {
            let provider_config = match ProviderConfig::try_from(provider) {
                Ok(config) => config,
                Err(err) => {
                    log::error!(
                        "[ProviderRegistry] Skipping provider with invalid provider_type: code={}, name={}, provider_type={}, reason={}",
                        provider.provider_code,
                        provider.name,
                        provider.provider_type,
                        err
                    );
                    continue;
                }
            };

            let provider_code = provider_config.provider_code.clone();
            let provider_name = provider_config.name.clone();
            let provider_status = provider_config
                .status
                .as_deref()
                .unwrap_or("NULL")
                .to_string();

            log::debug!(
                "[ProviderRegistry] Processing provider: code={}, name={}, status={}",
                provider_code,
                provider_name,
                provider_status
            );

            // 只注册活跃状态的供应商
            if provider_config.status.as_deref() == Some("active") {
                // 尝试解密API密钥
                let decrypted_key = if let (Some(encryption_service), Some(encrypted_key)) = (
                    encryption_service,
                    provider_config.api_key_encrypted.as_ref(),
                ) {
                    match encryption_service.decrypt(encrypted_key) {
                        Ok(key) => {
                            log::info!(
                                "[ProviderRegistry] Successfully decrypted API key for provider: {}",
                                provider_code
                            );
                            Some(key)
                        }
                        Err(e) => {
                            log::error!(
                                "[ProviderRegistry] Failed to decrypt API key for provider {}: {}",
                                provider_code,
                                e
                            );
                            None
                        }
                    }
                } else {
                    None
                };

                if let Some(instance) = registry.create_provider(&provider_config, decrypted_key) {
                    registry.providers.insert(provider_code.clone(), instance);
                    registry
                        .configs
                        .insert(provider_code.clone(), provider_config);
                    log::info!(
                        "[ProviderRegistry] Successfully registered provider: {} ({})",
                        provider_code,
                        provider_name
                    );
                } else {
                    log::warn!(
                        "[ProviderRegistry] Failed to create provider instance: {} ({})",
                        provider_code,
                        provider_name
                    );
                }
            } else {
                log::debug!(
                    "[ProviderRegistry] Skipping inactive provider: {} (status={})",
                    provider_code,
                    provider_status
                );
            }
        }

        log::info!(
            "[ProviderRegistry] Provider initialization complete. Registered {} providers",
            registry.providers.len()
        );

        registry
    }

    /// 从数据库配置创建一个新的供应商注册表（不加密）
    ///
    /// # 参数
    /// * `providers` - 数据库中的供应商配置列表
    ///
    /// # 返回
    /// 包含所有活跃供应商的注册表实例
    pub fn from_db(providers: &[DbProviderConfig]) -> Self {
        Self::from_db_with_encryption(providers, None)
    }

    /// 根据供应商配置创建供应商实例
    ///
    /// # 参数
    /// * `config` - 供应商配置
    /// * `decrypted_api_key` - 可选的解密后的API密钥
    ///
    /// # 返回
    /// 供应商实例，如果创建失败则返回 None
    fn create_provider(
        &self,
        config: &ProviderConfig,
        decrypted_api_key: Option<String>,
    ) -> Option<Arc<dyn Provider>> {
        let provider: Arc<dyn Provider> = match config.provider_type {
            crate::domain::dto::ProviderType::OpenAI => {
                let mut p = openai::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::OpenAICompatible => {
                let mut p = openai_compatible::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Codex => {
                let mut p = codex::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Claude => {
                let mut p = claude::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Gemini => {
                let mut p = gemini::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::GeminiCli => {
                let mut p = gemini_cli::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Vertex => {
                let mut p = vertex::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::AiStudio => {
                let mut p = aistudio::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Qwen => {
                let mut p = qwen::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Kimi => {
                let mut p = kimi::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Iflow => {
                let mut p = iflow::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Antigravity => {
                let mut p = antigravity::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
            crate::domain::dto::ProviderType::Custom => {
                let mut p = custom::provider::build(config);
                if let Some(key) = decrypted_api_key {
                    p.set_decrypted_api_key(key);
                }
                Arc::new(p)
            }
        };
        Some(provider)
    }

    /// 动态注册供应商
    ///
    /// # 参数
    /// * `config` - 供应商配置
    /// * `decrypted_api_key` - 可选的解密后的API密钥
    ///
    /// # 返回
    /// 成功返回 Ok(())，失败返回错误信息
    pub fn register_provider(
        &mut self,
        config: ProviderConfig,
        decrypted_api_key: Option<String>,
    ) -> Result<(), String> {
        let provider_code = config.provider_code.clone();

        // 创建供应商实例
        let provider_instance = self
            .create_provider(&config, decrypted_api_key)
            .ok_or_else(|| format!("Failed to create provider instance for {}", provider_code))?;

        // 注册供应商
        self.providers
            .insert(provider_code.clone(), provider_instance);
        self.configs.insert(provider_code.clone(), config);

        Ok(())
    }

    /// 注销供应商
    ///
    /// # 参数
    /// * `provider_code` - 供应商代号
    ///
    /// # 返回
    /// 成功返回 Ok(())，供应商不存在返回错误信息
    pub fn unregister_provider(&mut self, provider_code: &str) -> Result<(), String> {
        if self.providers.remove(provider_code).is_none() {
            return Err(format!("Provider {} not found", provider_code));
        }
        self.configs.remove(provider_code);
        Ok(())
    }

    /// 更新供应商配置
    ///
    /// # 参数
    /// * `config` - 新的供应商配置
    /// * `decrypted_api_key` - 可选的解密后的API密钥
    ///
    /// # 返回
    /// 成功返回 Ok(())，失败返回错误信息
    pub fn update_provider(
        &mut self,
        config: ProviderConfig,
        decrypted_api_key: Option<String>,
    ) -> Result<(), String> {
        let provider_code = config.provider_code.clone();

        // 检查供应商是否存在
        if !self.providers.contains_key(&provider_code) {
            return Err(format!("Provider {} not found", provider_code));
        }

        // 创建新的供应商实例
        let provider_instance = self
            .create_provider(&config, decrypted_api_key)
            .ok_or_else(|| format!("Failed to create provider instance for {}", provider_code))?;

        // 更新供应商
        self.providers
            .insert(provider_code.clone(), provider_instance);
        self.configs.insert(provider_code.clone(), config);

        Ok(())
    }

    /// 根据供应商代号获取供应商实例
    ///
    /// # 参数
    /// * `provider_code` - 供应商代号
    ///
    /// # 返回
    /// 供应商实例，如果不存在则返回 None
    pub fn get_provider(&self, provider_code: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(provider_code).cloned()
    }

    /// 根据供应商代号获取供应商配置
    ///
    /// # 参数
    /// * `provider_code` - 供应商代号
    ///
    /// # 返回
    /// 供应商配置，如果不存在则返回 None
    pub fn get_provider_config(&self, provider_code: &str) -> Option<&ProviderConfig> {
        self.configs.get(provider_code)
    }

    /// 获取所有供应商实例
    ///
    /// # 返回
    /// 所有供应商实例的列表
    pub fn get_all_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.providers.values().cloned().collect()
    }

    /// 获取所有供应商配置
    ///
    /// # 返回
    /// 所有供应商配置的列表
    pub fn get_all_configs(&self) -> Vec<&ProviderConfig> {
        self.configs.values().collect()
    }

    /// 检查供应商是否存在
    ///
    /// # 参数
    /// * `provider_code` - 供应商代号
    ///
    /// # 返回
    /// 存在返回 true，否则返回 false
    pub fn contains_provider(&self, provider_code: &str) -> bool {
        self.providers.contains_key(provider_code)
    }

    /// 获取已注册供应商数量
    ///
    /// # 返回
    /// 已注册供应商的数量
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderRegistry;
    use crate::domain::table::ai_hub::provider_config::ProviderConfig as DbProviderConfig;

    fn build_db_provider(provider_code: &str, provider_type: &str) -> DbProviderConfig {
        DbProviderConfig {
            id: Some(format!("id-{}", provider_code)),
            provider_code: provider_code.to_string(),
            name: format!("{}-name", provider_code),
            provider_type: provider_type.to_string(),
            api_base: "https://example.com/v1".to_string(),
            api_key_encrypted: None,
            auth_config: None,
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
    fn from_db_should_skip_invalid_provider_type() {
        let providers = vec![
            build_db_provider("valid-custom", "custom"),
            build_db_provider("invalid", "unknown-provider"),
        ];

        let registry = ProviderRegistry::from_db(&providers);

        assert!(registry.contains_provider("valid-custom"));
        assert!(!registry.contains_provider("invalid"));
        assert_eq!(registry.provider_count(), 1);
    }

    #[test]
    fn from_db_should_accept_all_new_provider_types() {
        let new_types = [
            ("claude-provider", "claude"),
            ("gemini-provider", "gemini"),
            ("gemini-cli-provider", "gemini-cli"),
            ("vertex-provider", "vertex"),
            ("aistudio-provider", "aistudio"),
            ("qwen-provider", "qwen"),
            ("kimi-provider", "kimi"),
            ("iflow-provider", "iflow"),
            ("antigravity-provider", "antigravity"),
        ];
        let providers: Vec<_> = new_types
            .iter()
            .map(|(code, ptype)| build_db_provider(code, ptype))
            .collect();

        let registry = ProviderRegistry::from_db(&providers);

        for (code, _) in new_types {
            assert!(
                registry.contains_provider(code),
                "{} should be registered",
                code
            );
        }
        assert_eq!(registry.provider_count(), new_types.len());
    }

    #[test]
    fn from_db_should_accept_case_insensitive_provider_type() {
        let providers = vec![build_db_provider("mixed-case", "OpenAI-Compatible")];

        let registry = ProviderRegistry::from_db(&providers);

        assert!(registry.contains_provider("mixed-case"));
        assert_eq!(registry.provider_count(), 1);
    }

    #[test]
    fn from_db_should_accept_provider_type_with_surrounded_whitespace() {
        let providers = vec![build_db_provider("trimmed", "  OpenAI-Compatible  ")];

        let registry = ProviderRegistry::from_db(&providers);

        assert!(registry.contains_provider("trimmed"));
        assert_eq!(registry.provider_count(), 1);
    }

    #[test]
    fn from_db_should_accept_codex_provider_type() {
        let providers = vec![build_db_provider("codex-provider", "CoDeX")];

        let registry = ProviderRegistry::from_db(&providers);

        assert!(registry.contains_provider("codex-provider"));
        assert_eq!(registry.provider_count(), 1);
    }
}
