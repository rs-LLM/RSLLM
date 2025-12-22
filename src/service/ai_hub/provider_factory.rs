//! AI服务提供商工厂模块
//! 负责创建和管理不同AI服务提供商的实例

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{Result, Error};
use crate::service::ai_hub::provider_trait::{AIProvider, ChatCompletionRequest, ChatCompletionResponse};
use crate::service::ai_hub::provider_trait::{CompletionRequest, CompletionResponse};
use crate::service::ai_hub::provider_trait::{EmbeddingRequest, EmbeddingResponse, ModelInfo};
use crate::service::ai_hub::provider_trait::{ChatCompletionChunk};

/// 适配器工厂 trait
///
/// 定义创建AI服务提供商实例的统一接口
#[async_trait]
pub trait ProviderFactory: Send + Sync {
    /// 创建AI服务提供商实例
    async fn create_provider(&self, config: ProviderConfig) -> Result<Box<dyn AIProvider>>;
    
    /// 验证配置是否有效
    fn validate_config(&self, config: &ProviderConfig) -> Result<()>;
}

/// 通用适配器工厂
///
/// 管理所有AI服务提供商的创建和配置
pub struct GenericProviderFactory {
    providers: RwLock<HashMap<String, Arc<dyn AIProvider>>>,
}

impl GenericProviderFactory {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
        }
    }
    
    /// 注册提供商实例
    pub async fn register_provider(
        &self,
        provider_id: String,
        provider: Arc<dyn AIProvider>,
    ) {
        let mut providers = self.providers.write().await;
        providers.insert(provider_id, provider);
    }
    
    /// 获取已注册的提供商
    pub async fn get_provider(&self, provider_id: &str) -> Option<Arc<dyn AIProvider>> {
        let providers = self.providers.read().await;
        providers.get(provider_id).cloned()
    }
    
    /// 移除提供商
    pub async fn remove_provider(&self, provider_id: &str) {
        let mut providers = self.providers.write().await;
        providers.remove(provider_id);
    }
}

#[async_trait]
impl ProviderFactory for GenericProviderFactory {
    async fn create_provider(&self, config: ProviderConfig) -> Result<Box<dyn AIProvider>> {
        self.validate_config(&config)?;
        
        // 检查是否已存在该提供商的实例
        {
            let providers = self.providers.read().await;
            if let Some(provider) = providers.get(&config.provider_id) {
                return Ok(Box::new(ProviderWrapper::new(provider.clone(), config)));
            }
        }
        
        // 根据提供商类型创建新实例
        let provider: Arc<dyn AIProvider> = match config.provider_type.as_str() {
            "openai" => {
                Arc::new(crate::providers::openai_provider::OpenAIProvider::new(config.clone())?)
            }
            "claude" => {
                Arc::new(crate::providers::claude_provider::ClaudeProvider::new(config.clone())?)
            }
            "custom" => {
                Arc::new(CustomProvider::new(config.clone())?)
            }
            _ => {
                return Err(Error::ConfigError(format!("Unknown provider type: {}", config.provider_type)));
            }
        };
        
        // 创建包装器并缓存
        let wrapper = ProviderWrapper::new(provider.clone(), config.clone());
        
        // 缓存提供商实例
        let mut providers = self.providers.write().await;
        providers.insert(config.provider_id.clone(), provider.clone());
        
        Ok(Box::new(wrapper))
    }
    
    fn validate_config(&self, config: &ProviderConfig) -> Result<()> {
        if config.provider_id.is_empty() {
            return Err(Error::ConfigError("Provider ID cannot be empty".to_string()));
        }
        
        if config.provider_type.is_empty() {
            return Err(Error::ConfigError("Provider type cannot be empty".to_string()));
        }
        
        if config.api_base.is_empty() {
            return Err(Error::ConfigError("API base cannot be empty".to_string()));
        }
        
        if config.api_key.is_empty() {
            return Err(Error::ConfigError("API key cannot be empty".to_string()));
        }
        
        Ok(())
    }
}

/// 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_id: String,
    pub provider_type: String,
    pub api_base: String,
    pub api_key: String,
    pub timeout_seconds: Option<u64>,
    pub max_retries: Option<u32>,
    pub rate_limit: Option<u32>,
    pub circuit_breaker_threshold: Option<u32>,
    pub custom_headers: Option<HashMap<String, String>>,
}

/// 提供商包装器
///
/// 为AIProvider添加配置管理和装饰器功能
pub struct ProviderWrapper {
    provider: Arc<dyn AIProvider>,
}

impl ProviderWrapper {
    pub fn new(provider: Arc<dyn AIProvider>, _config: ProviderConfig) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl AIProvider for ProviderWrapper {
    async fn chat_completions(&self, req: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        // 这里可以添加重试、限流、熔断等装饰器逻辑
        self.provider.chat_completions(req).await
    }
    
    async fn completions(&self, req: &CompletionRequest) -> Result<CompletionResponse> {
        self.provider.completions(req).await
    }
    
    async fn embeddings(&self, req: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        self.provider.embeddings(req).await
    }
    
    async fn chat_stream(&self, req: &ChatCompletionRequest) -> Result<Box<dyn futures::Stream<Item = Result<ChatCompletionChunk>> + Send + Unpin>> {
        self.provider.chat_stream(req).await
    }
    
    async fn health_check(&self) -> Result<bool> {
        self.provider.health_check().await
    }
    
    fn get_model_info(&self) -> ModelInfo {
        self.provider.get_model_info()
    }
}

/// 自定义提供商
///
/// 用于支持自定义AI服务提供商
pub struct CustomProvider {
}

impl CustomProvider {
    pub fn new(_config: ProviderConfig) -> Result<Self> {
        Ok(Self {})
    }
}

#[async_trait]
impl AIProvider for CustomProvider {
    async fn chat_completions(&self, _req: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        Err(Error::BusinessError("Custom provider chat completions not implemented".to_string()))
    }
    
    async fn completions(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        Err(Error::BusinessError("Custom provider completions not implemented".to_string()))
    }
    
    async fn embeddings(&self, _req: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(Error::BusinessError("Custom provider embeddings not implemented".to_string()))
    }
    
    async fn chat_stream(&self, _req: &ChatCompletionRequest) -> Result<Box<dyn futures::Stream<Item = Result<ChatCompletionChunk>> + Send + Unpin>> {
        Err(Error::BusinessError("Custom provider chat stream not implemented".to_string()))
    }
    
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
    
    fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            provider: "custom".to_string(),
            capabilities: vec!["custom".to_string()],
            max_tokens: 4096,
            streaming: false,
        }
    }
}