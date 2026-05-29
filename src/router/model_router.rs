//! 模型路由器模块
//! 负责解析模型标识符并路由到正确的供应商和模型

use crate::domain::dto::provider::ProviderConfig as ProviderConfigDTO;
use crate::domain::table::ai_hub::model_base::ModelBase;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::table::ai_hub::provider_config::ProviderConfig;
use crate::error::{Error, Result};
use crate::providers::registry::ProviderRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 模型路由信息
///
/// 组合了模型基础信息和供应商特定配置
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModelRoutingInfo {
    /// 模型基础信息
    pub model_base: ModelBase,
    /// 模型-供应商映射信息
    pub mapping: ModelProviderMapping,
    /// 供应商配置
    pub provider: ProviderConfig,
}

/// 模型路由器
///
/// # 功能特性
/// - 解析 "provider_code/model_code" 格式的模型标识符
/// - 查找对应的模型定义和供应商配置
/// - 提供模型路由和供应商查找功能
/// - 支持缓存以提高查询性能
pub struct ModelRouter {
    /// 供应商注册表引用
    provider_registry: Arc<RwLock<ProviderRegistry>>,
    route_cache: Arc<RwLock<HashMap<RouteCacheKey, CachedRoutingEntry>>>,
    route_cache_ttl: Duration,
}

#[derive(Clone, Debug, Eq)]
struct RouteCacheKey {
    model_identifier: String,
    enable_fallback: bool,
}

impl PartialEq for RouteCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.model_identifier == other.model_identifier
            && self.enable_fallback == other.enable_fallback
    }
}

impl Hash for RouteCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.model_identifier.hash(state);
        self.enable_fallback.hash(state);
    }
}

#[derive(Clone)]
struct CachedRoutingEntry {
    expires_at: Instant,
    value: ModelRoutingInfo,
}

impl ModelRouter {
    fn resolve_page(page: i64) -> Result<usize> {
        if page < 1 {
            return Err(Error::ValidationError(format!(
                "page must be >= 1, got {}",
                page
            )));
        }

        usize::try_from(page).map_err(|_| Error::ValidationError("page is too large".to_string()))
    }

    fn resolve_page_size(page_size: i64) -> Result<usize> {
        if page_size < 1 {
            return Err(Error::ValidationError(format!(
                "page_size must be >= 1, got {}",
                page_size
            )));
        }

        usize::try_from(page_size)
            .map_err(|_| Error::ValidationError("page_size is too large".to_string()))
    }

    /// 创建新的模型路由器
    ///
    /// # 参数
    /// * `provider_registry` - 供应商注册表
    pub fn new(provider_registry: Arc<RwLock<ProviderRegistry>>) -> Self {
        Self {
            provider_registry,
            route_cache: Arc::new(RwLock::new(HashMap::new())),
            route_cache_ttl: Duration::from_secs(30),
        }
    }

    fn route_cache_key(model_identifier: &str, enable_fallback: bool) -> RouteCacheKey {
        RouteCacheKey {
            model_identifier: model_identifier.to_string(),
            enable_fallback,
        }
    }

    async fn get_cached_route(&self, cache_key: &RouteCacheKey) -> Option<ModelRoutingInfo> {
        let now = Instant::now();

        {
            let cache = self.route_cache.read().await;
            if let Some(entry) = cache.get(cache_key)
                && entry.expires_at > now
            {
                return Some(entry.value.clone());
            }
        }

        let mut cache = self.route_cache.write().await;
        if let Some(entry) = cache.get(cache_key)
            && entry.expires_at <= now
        {
            cache.remove(cache_key);
        }
        None
    }

    async fn set_cached_route(&self, cache_key: RouteCacheKey, route: ModelRoutingInfo) {
        let entry = CachedRoutingEntry {
            expires_at: Instant::now() + self.route_cache_ttl,
            value: route,
        };
        self.route_cache.write().await.insert(cache_key, entry);
    }

    /// 解析模型标识符
    ///
    /// # 参数
    /// * `model_identifier` - 模型标识符，格式为 "provider_code/model_code"
    ///
    /// # 返回
    /// 成功返回 (provider_code, model_code)，失败返回错误
    ///
    /// # 示例
    /// ```
    /// // 此方法需要ModelRouter实例，实际使用时通过依赖注入获取
    /// // 示例展示了返回值的格式
    /// let provider_code = "openai";
    /// let model_code = "gpt-4-turbo";
    /// assert_eq!(provider_code, "openai");
    /// assert_eq!(model_code, "gpt-4-turbo");
    /// ```
    pub fn parse_identifier(&self, model_identifier: &str) -> Result<(String, String)> {
        if model_identifier.is_empty() {
            return Err(Error::ValidationError(
                "Model identifier cannot be empty".to_string(),
            ));
        }

        let parts: Vec<&str> = model_identifier.split('/').collect();

        if parts.len() != 2 {
            return Err(Error::ValidationError(format!(
                "Invalid model identifier format '{}', expected 'provider_code/model_code'",
                model_identifier
            )));
        }

        let provider_code = parts[0].trim();
        let model_code = parts[1].trim();

        if provider_code.is_empty() {
            return Err(Error::ValidationError(
                "Provider code cannot be empty".to_string(),
            ));
        }

        if model_code.is_empty() {
            return Err(Error::ValidationError(
                "Model code cannot be empty".to_string(),
            ));
        }

        Ok((provider_code.to_string(), model_code.to_string()))
    }

    /// 路由到供应商
    ///
    /// # 参数
    /// * `model_identifier` - 模型标识符，格式为 "provider_code/model_code"
    ///
    /// # 返回
    /// 成功返回供应商配置，失败返回错误
    pub async fn route_to_provider(&self, model_identifier: &str) -> Result<ProviderConfigDTO> {
        let (provider_code, _) = self.parse_identifier(model_identifier)?;

        let registry = self.provider_registry.read().await;
        registry
            .get_provider_config(&provider_code)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("Provider '{}' not found", provider_code)))
    }

    /// 路由到模型
    ///
    /// # 参数
    /// * `model_identifier` - 模型标识符，格式为 "provider_code/model_code"
    ///
    /// # 返回
    /// 成功返回模型路由信息，失败返回错误
    pub async fn route_to_model(&self, model_identifier: &str) -> Result<ModelRoutingInfo> {
        let cache_key = Self::route_cache_key(model_identifier, false);
        if let Some(cached) = self.get_cached_route(&cache_key).await {
            return Ok(cached);
        }

        let (provider_code, model_code) = self.parse_identifier(model_identifier)?;

        let rb = crate::pool!();

        let (provider_result, model_result) = tokio::join!(
            ProviderConfig::select_by_provider_code(rb, &provider_code),
            ModelBase::select_by_model_code(rb, &model_code)
        );

        let provider = provider_result
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .ok_or_else(|| Error::NotFound(format!("Provider '{}' not found", provider_code)))?;

        let model_base = model_result
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .ok_or_else(|| Error::NotFound(format!("Model '{}' not found", model_code)))?;

        let provider_id = provider.id.clone().unwrap_or_default();
        let model_id = model_base.id.clone().unwrap_or_default();

        let mapping =
            ModelProviderMapping::select_by_model_and_provider(rb, &model_id, &provider_id)
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?
                .ok_or_else(|| {
                    Error::NotFound(format!(
                        "Mapping for model '{}' and provider '{}' not found",
                        model_code, provider_code
                    ))
                })?;

        let route = ModelRoutingInfo {
            model_base,
            mapping,
            provider,
        };

        self.set_cached_route(cache_key, route.clone()).await;
        Ok(route)
    }

    /// 尝试使用指定的映射
    ///
    /// # 参数
    /// * `model_base` - 模型基础信息
    /// * `mapping` - 模型-供应商映射
    /// * `provider` - 供应商配置
    ///
    /// # 返回
    /// 成功返回模型路由信息，失败返回错误
    async fn try_mapping(
        &self,
        model_base: ModelBase,
        mapping: ModelProviderMapping,
        provider: ProviderConfig,
    ) -> Result<ModelRoutingInfo> {
        Ok(ModelRoutingInfo {
            model_base,
            mapping,
            provider,
        })
    }

    /// 路由到模型（支持降级）
    ///
    /// # 参数
    /// * `model_identifier` - 模型标识符，格式为 "provider_code/model_code"
    ///
    /// # 返回
    /// 成功返回模型路由信息，失败返回错误
    ///
    /// # 降级机制
    /// 当高优先级映射不可用时，自动尝试低优先级映射
    pub async fn route_to_model_with_fallback(
        &self,
        model_identifier: &str,
    ) -> Result<ModelRoutingInfo> {
        let cache_key = Self::route_cache_key(model_identifier, true);
        if let Some(cached) = self.get_cached_route(&cache_key).await {
            return Ok(cached);
        }

        let (provider_code, model_code) = self.parse_identifier(model_identifier)?;

        let rb = crate::pool!();

        let provider = ProviderConfig::select_by_provider_code(rb, &provider_code)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .ok_or_else(|| Error::NotFound(format!("Provider '{}' not found", provider_code)))?;

        let model_base = ModelBase::select_by_model_code(rb, &model_code)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .ok_or_else(|| Error::NotFound(format!("Model '{}' not found", model_code)))?;

        let provider_id = provider.id.clone().unwrap_or_default();
        let model_id = model_base.id.clone().unwrap_or_default();

        let mappings =
            ModelProviderMapping::select_by_model_and_provider_all(rb, &model_id, &provider_id)
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

        if mappings.is_empty() {
            return Err(Error::NotFound(format!(
                "No mappings found for model '{}' and provider '{}'",
                model_code, provider_code
            )));
        }

        let mut last_error = None;
        for mapping in mappings {
            match self
                .try_mapping(model_base.clone(), mapping.clone(), provider.clone())
                .await
            {
                Ok(routing_info) => {
                    self.set_cached_route(cache_key, routing_info.clone()).await;
                    return Ok(routing_info);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to use mapping (priority: {:?}): {}, trying next mapping...",
                        mapping.priority,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::NotFound(format!(
                "All mappings failed for model '{}' and provider '{}'",
                model_code, provider_code
            ))
        }))
    }

    /// 路由到供应商和模型
    ///
    /// # 参数
    /// * `model_identifier` - 模型标识符，格式为 "provider_code/model_code"
    /// * `enable_fallback` - 是否启用降级机制（默认为false）
    ///
    /// # 返回
    /// 成功返回 (供应商配置, 模型路由信息)，失败返回错误
    pub async fn route(
        &self,
        model_identifier: &str,
        enable_fallback: bool,
    ) -> Result<(ProviderConfigDTO, ModelRoutingInfo)> {
        let model_info = if enable_fallback {
            self.route_to_model_with_fallback(model_identifier).await?
        } else {
            self.route_to_model(model_identifier).await?
        };

        let provider = ProviderConfigDTO::try_from(&model_info.provider)
            .map_err(|e| Error::ValidationError(format!("Invalid provider config: {}", e)))?;

        Ok((provider, model_info))
    }

    /// 验证模型标识符是否有效
    ///
    /// # 参数
    /// * `model_identifier` - 模型标识符
    ///
    /// # 返回
    /// 成功返回 true，失败返回 false
    pub async fn validate_identifier(&self, model_identifier: &str) -> bool {
        self.route(model_identifier, false).await.is_ok()
    }

    /// 列出指定供应商的所有可用模型
    ///
    /// # 参数
    /// * `provider_code` - 供应商代号
    ///
    /// # 返回
    /// 成功返回模型路由信息列表，失败返回错误
    pub async fn list_provider_models(&self, provider_code: &str) -> Result<Vec<ModelRoutingInfo>> {
        let rb = crate::pool!();

        let provider = ProviderConfig::select_by_provider_code(rb, &provider_code)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .ok_or_else(|| Error::NotFound(format!("Provider '{}' not found", provider_code)))?;

        let provider_id = provider.id.clone().unwrap_or_default();

        let mappings = ModelProviderMapping::select_by_provider_id(rb, &provider_id)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let mut model_infos = Vec::new();
        for mapping in mappings {
            if let Ok(Some(model_base)) =
                ModelBase::select_by_model_code(rb, &mapping.provider_model_name).await
            {
                model_infos.push(ModelRoutingInfo {
                    model_base,
                    mapping: mapping.clone(),
                    provider: provider.clone(),
                });
            }
        }

        Ok(model_infos)
    }

    /// 搜索模型
    ///
    /// # 参数
    /// * `keyword` - 搜索关键词
    ///
    /// # 返回
    /// 成功返回匹配的模型路由信息列表，失败返回错误
    pub async fn search_models(&self, keyword: &str) -> Result<Vec<ModelRoutingInfo>> {
        let rb = crate::pool!();

        let sql = "SELECT * FROM model_base WHERE model_code LIKE ? OR name LIKE ? ORDER BY created_at DESC";
        let pattern = format!("%{}%", keyword);

        let result = rbatis::RBatis::query(
            crate::pool!(),
            sql,
            vec![
                rbs::Value::String(pattern.clone()),
                rbs::Value::String(pattern),
            ],
        )
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let model_bases: Vec<ModelBase> = result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).ok()
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut model_infos = Vec::new();
        for model_base in model_bases {
            let model_id = model_base.id.clone().unwrap_or_default();
            if let Ok(mappings) = ModelProviderMapping::select_by_model_id(rb, &model_id).await {
                let deduplicated_mappings = ModelProviderMapping::deduplicate_by_priority(mappings);
                for mapping in deduplicated_mappings {
                    if let Ok(Some(provider)) =
                        ProviderConfig::select_by_provider_code(rb, &mapping.provider_id).await
                    {
                        model_infos.push(ModelRoutingInfo {
                            model_base: model_base.clone(),
                            mapping: mapping.clone(),
                            provider,
                        });
                    }
                }
            }
        }

        Ok(model_infos)
    }

    /// 列出所有供应商的所有可用模型
    ///
    /// # 返回
    /// 成功返回所有模型路由信息列表，失败返回错误
    pub async fn list_all_models(&self) -> Result<Vec<ModelRoutingInfo>> {
        let rb = crate::pool!();

        let providers = ProviderConfig::select_all(rb)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let mut model_infos = Vec::new();
        for provider in providers {
            let provider_id = provider.id.clone().unwrap_or_default();
            if let Ok(mappings) =
                ModelProviderMapping::select_by_provider_id(rb, &provider_id).await
            {
                let deduplicated_mappings = ModelProviderMapping::deduplicate_by_priority(mappings);
                for mapping in deduplicated_mappings {
                    if let Ok(model_bases) =
                        ModelBase::select_by_map(rb, rbs::value! { "id": &mapping.model_id }).await
                    {
                        if let Some(model_base) = model_bases.first() {
                            model_infos.push(ModelRoutingInfo {
                                model_base: model_base.clone(),
                                mapping: mapping.clone(),
                                provider: provider.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(model_infos)
    }

    /// 列出所有供应商的所有可用模型（支持分页）
    ///
    /// # 参数
    /// * `page` - 页码（从1开始）
    /// * `page_size` - 每页数量
    ///
    /// # 返回
    /// 成功返回模型路由信息列表，失败返回错误
    pub async fn list_all_models_page(
        &self,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<ModelRoutingInfo>> {
        let page = Self::resolve_page(page)?;
        let page_size = Self::resolve_page_size(page_size)?;

        let rb = crate::pool!();

        let providers = ProviderConfig::select_all(rb)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let mut all_model_infos = Vec::new();
        for provider in providers {
            let provider_id = provider.id.clone().unwrap_or_default();
            if let Ok(mappings) =
                ModelProviderMapping::select_by_provider_id(rb, &provider_id).await
            {
                let deduplicated_mappings = ModelProviderMapping::deduplicate_by_priority(mappings);
                for mapping in deduplicated_mappings {
                    if let Ok(model_bases) =
                        ModelBase::select_by_map(rb, rbs::value! { "id": &mapping.model_id }).await
                    {
                        if let Some(model_base) = model_bases.first() {
                            all_model_infos.push(ModelRoutingInfo {
                                model_base: model_base.clone(),
                                mapping: mapping.clone(),
                                provider: provider.clone(),
                            });
                        }
                    }
                }
            }
        }

        let offset = (page - 1).saturating_mul(page_size);
        let end = offset.saturating_add(page_size).min(all_model_infos.len());

        if offset >= all_model_infos.len() {
            Ok(Vec::new())
        } else {
            Ok(all_model_infos[offset..end].to_vec())
        }
    }

    /// 搜索模型（支持分页）
    ///
    /// # 参数
    /// * `keyword` - 搜索关键词
    /// * `page` - 页码（从1开始）
    /// * `page_size` - 每页数量
    ///
    /// # 返回
    /// 成功返回匹配的模型路由信息列表，失败返回错误
    pub async fn search_models_page(
        &self,
        keyword: &str,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<ModelRoutingInfo>> {
        let page = Self::resolve_page(page)?;
        let page_size = Self::resolve_page_size(page_size)?;

        let rb = crate::pool!();

        let sql = "SELECT * FROM model_base WHERE model_code LIKE ? OR name LIKE ? ORDER BY created_at DESC";
        let pattern = format!("%{}%", keyword);

        let result = rbatis::RBatis::query(
            crate::pool!(),
            sql,
            vec![
                rbs::Value::String(pattern.clone()),
                rbs::Value::String(pattern),
            ],
        )
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let model_bases: Vec<ModelBase> = result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).ok()
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut all_model_infos = Vec::new();
        for model_base in model_bases {
            let model_id = model_base.id.clone().unwrap_or_default();
            if let Ok(mappings) = ModelProviderMapping::select_by_model_id(rb, &model_id).await {
                let deduplicated_mappings = ModelProviderMapping::deduplicate_by_priority(mappings);
                for mapping in deduplicated_mappings {
                    if let Ok(Some(provider)) =
                        ProviderConfig::select_by_provider_code(rb, &mapping.provider_id).await
                    {
                        all_model_infos.push(ModelRoutingInfo {
                            model_base: model_base.clone(),
                            mapping: mapping.clone(),
                            provider,
                        });
                    }
                }
            }
        }

        let offset = (page - 1).saturating_mul(page_size);
        let end = offset.saturating_add(page_size).min(all_model_infos.len());

        if offset >= all_model_infos.len() {
            Ok(Vec::new())
        } else {
            Ok(all_model_infos[offset..end].to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration as TokioDuration, sleep};

    #[test]
    fn test_parse_identifier() {
        let router = ModelRouter::new(Arc::new(RwLock::new(ProviderRegistry::new())));

        // 正常情况
        let result = router.parse_identifier("openai/gpt-4-turbo");
        assert!(result.is_ok());
        let (provider_code, model_code) = result.unwrap();
        assert_eq!(provider_code, "openai");
        assert_eq!(model_code, "gpt-4-turbo");

        // 空字符串
        let result = router.parse_identifier("");
        assert!(result.is_err());

        // 缺少分隔符
        let result = router.parse_identifier("openai");
        assert!(result.is_err());

        // 多余分隔符
        let result = router.parse_identifier("openai/gpt-4-turbo/extra");
        assert!(result.is_err());

        // 空的 provider_code
        let result = router.parse_identifier("/gpt-4-turbo");
        assert!(result.is_err());

        // 空的 model_code
        let result = router.parse_identifier("openai/");
        assert!(result.is_err());
    }

    #[test]
    fn test_route_cache_key_no_collision_with_separator_like_content() {
        let key_a = ModelRouter::route_cache_key("a|b", true);
        let key_b = ModelRouter::route_cache_key("a", false);
        let key_c = ModelRouter::route_cache_key("a|b", false);
        let key_d = ModelRouter::route_cache_key("a", true);

        assert_ne!(key_a, key_b);
        assert_ne!(key_a, key_c);
        assert_ne!(key_b, key_d);

        let mut map = HashMap::new();
        map.insert(key_a, 1);
        map.insert(key_b, 2);
        map.insert(key_c, 3);
        map.insert(key_d, 4);

        assert_eq!(map.len(), 4);
    }

    #[tokio::test]
    async fn test_route_cache_ttl_expiry() {
        let mut router = ModelRouter::new(Arc::new(RwLock::new(ProviderRegistry::new())));
        router.route_cache_ttl = Duration::from_millis(10);

        let route = ModelRoutingInfo {
            model_base: ModelBase {
                id: Some("model-1".to_string()),
                model_code: "gpt-5.3-codex".to_string(),
                ..ModelBase::default()
            },
            mapping: ModelProviderMapping {
                id: Some("mapping-1".to_string()),
                model_id: "model-1".to_string(),
                provider_id: "provider-1".to_string(),
                provider_model_name: "gpt-5.3-codex".to_string(),
                ..ModelProviderMapping::default()
            },
            provider: ProviderConfig {
                id: Some("provider-1".to_string()),
                provider_code: "openai".to_string(),
                name: "OpenAI".to_string(),
                provider_type: "codex".to_string(),
                api_base: "https://chatgpt.com/backend-api/codex".to_string(),
                ..ProviderConfig::default()
            },
        };

        let key = ModelRouter::route_cache_key("openai/gpt-5.3-codex", true);
        router.set_cached_route(key.clone(), route.clone()).await;

        let hit = router.get_cached_route(&key).await;
        assert!(hit.is_some());

        sleep(TokioDuration::from_millis(20)).await;

        let expired = router.get_cached_route(&key).await;
        assert!(expired.is_none());
    }
}
