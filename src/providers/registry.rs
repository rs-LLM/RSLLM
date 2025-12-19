// 供应商注册表实现
// 用于管理和注册所有AI服务供应商

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::table::provider::Provider as DbProvider;
use crate::domain::dto::{ProviderConfig, ProviderType};
use crate::providers::anthropic::provider::AnthropicProvider;
use crate::providers::common::CommonProvider;
use crate::providers::provider::Provider;

/// 供应商注册表，用于管理和注册所有AI服务供应商
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    /// 创建一个新的供应商注册表
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }
    
    /// 从数据库配置创建一个新的供应商注册表
    pub fn from_db(providers: &[DbProvider]) -> Self {
        let mut registry = Self::new();
        
        for provider in providers {
            let provider_config = ProviderConfig::from(provider);
            let provider_instance = registry.create_provider(&provider_config);
            
            if let Some(instance) = provider_instance {
                registry.providers.insert(provider.id.clone().unwrap_or_default(), instance);
            }
        }
        
        registry
    }
    
    /// 根据供应商配置创建供应商实例
    fn create_provider(&self, config: &ProviderConfig) -> Option<Arc<dyn Provider>> {
        match config.provider_type {
            ProviderType::OpenAI => {
                Some(Arc::new(CommonProvider::new(config)))
            },
            ProviderType::Anthropic => {
                Some(Arc::new(AnthropicProvider::new(config)))
            },
            ProviderType::Azure => {
                Some(Arc::new(CommonProvider::new(config)))
            },
            ProviderType::Bedrock => {
                Some(Arc::new(CommonProvider::new(config)))
            },
            ProviderType::VertexAI => {
                Some(Arc::new(CommonProvider::new(config)))
            },
            ProviderType::Custom => {
                Some(Arc::new(CommonProvider::new(config)))
            },
        }
    }
    
    /// 获取供应商实例
    pub fn get_provider(&self, provider_id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(provider_id).cloned()
    }
    
    /// 获取所有供应商实例
    pub fn get_all_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.providers.values().cloned().collect()
    }
}