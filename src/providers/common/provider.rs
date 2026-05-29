// 通用OpenAI兼容供应商实现，支持多种供应商类型
use async_trait::async_trait;
use axum::http::StatusCode;
use base64::Engine;
use chrono::Utc;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tracing::{error, info};

use crate::domain::dto::ai_hub::content::{ChatCompletionMessage, ChatMessageContent};
use crate::domain::dto::chat::{ChatCompletionRequest, ChatCompletionResponse, ReasoningConfig};
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::dto::{ProviderConfig, ProviderType};
use crate::domain::vo::chat::ChatCompletionChoice;
use crate::domain::vo::embeddings::EmbeddingsResponse;
use crate::domain::vo::usage::Usage;
use crate::middleware::auth_axum::TOKEN_KEY;
use crate::providers::provider::Provider;
use crate::service::ai_hub::provider::oauth::{
    SelectedProviderOAuthToken, mark_provider_account_unauthorized, record_provider_request_result,
    select_provider_oauth_token,
};

/// 通用聊天补全请求结构，支持多种供应商
#[derive(Deserialize, Clone)]
struct CommonChatCompletionRequest {
    base: ChatCompletionRequest,
    reasoning_effort: Option<String>,
    extra_body: Option<serde_json::Value>,
}

impl Serialize for CommonChatCompletionRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;

        // 序列化 base 中的所有字段
        let base_json = serde_json::to_value(&self.base).map_err(serde::ser::Error::custom)?;
        if let serde_json::Value::Object(base_map) = base_json {
            for (key, value) in base_map {
                if !value.is_null() {
                    map.serialize_entry(&key, &value)?;
                }
            }
        }

        // 序列化 reasoning_effort
        if let Some(ref effort) = self.reasoning_effort {
            map.serialize_entry("reasoning_effort", effort)?;
        }

        // 合并 extra_body 中的所有字段到顶层
        if let Some(ref extra) = self.extra_body {
            if let serde_json::Value::Object(extra_map) = extra {
                for (key, value) in extra_map {
                    if !value.is_null() {
                        map.serialize_entry(&key, &value)?;
                    }
                }
            }
        }

        map.end()
    }
}

impl CommonChatCompletionRequest {
    fn from_for_provider(mut base: ChatCompletionRequest, provider_type: &ProviderType) -> Self {
        // 处理max_completion_tokens逻辑 - 如果提供了max_completion_tokens且>0，则使用它，否则回退到max_tokens
        base.max_completion_tokens = match (base.max_completion_tokens, base.max_tokens) {
            (Some(v), _) if v > 0 => Some(v),
            (_, Some(v)) if v > 0 => Some(v),
            _ => None,
        };

        base.max_tokens = None;

        let reasoning = base.reasoning.take();

        // 提取extra_body用于透传
        let mut extra_body = base.extra_body.take();

        // 处理 thinking_budget 参数，支持字符串和数字
        if let Some(extra) = &mut extra_body {
            if let serde_json::Value::Object(map) = extra {
                if let Some(thinking_budget) = map.remove("thinking_budget") {
                    let converted_budget = match thinking_budget {
                        // 如果是字符串，转换为对应的数字
                        serde_json::Value::String(s) => match s.as_str() {
                            "high" => Some(serde_json::Value::Number(1024.into())),
                            "medium" => Some(serde_json::Value::Number(512.into())),
                            "low" => Some(serde_json::Value::Number(256.into())),
                            _ => None,
                        },
                        // 如果已经是数字，保持不变
                        serde_json::Value::Number(_) => Some(thinking_budget),
                        _ => None,
                    };

                    if let Some(budget) = converted_budget {
                        map.insert("thinking_budget".to_string(), budget);
                    }
                }
            }
        }

        let normalized_effort = reasoning
            .as_ref()
            .and_then(Self::reasoning_effort_from_config);

        let normalized_budget = reasoning
            .as_ref()
            .and_then(|cfg| cfg.max_tokens)
            .or_else(|| {
                extra_body.as_ref().and_then(|extra| {
                    extra
                        .as_object()
                        .and_then(|map| map.get("thinking_budget"))
                        .and_then(|v| v.as_u64())
                        .and_then(|v| u32::try_from(v).ok())
                })
            });

        let reasoning_effort = match provider_type {
            ProviderType::OpenAI
            | ProviderType::OpenAICompatible
            | ProviderType::Codex
            | ProviderType::Qwen
            | ProviderType::Kimi
            | ProviderType::Custom => normalized_effort
                .clone()
                .or_else(|| normalized_budget.map(Self::budget_to_effort)),
            _ => None,
        };

        if *provider_type == ProviderType::Claude {
            let disabled = normalized_budget == Some(0)
                || normalized_effort
                    .as_deref()
                    .map(|v| v.eq_ignore_ascii_case("none"))
                    .unwrap_or(false);
            if let Some(map) = Self::ensure_extra_body_object(&mut extra_body) {
                if disabled {
                    map.insert(
                        "thinking".to_string(),
                        serde_json::json!({ "type": "disabled" }),
                    );
                } else if let Some(budget) = normalized_budget
                    .or_else(|| normalized_effort.as_deref().map(Self::effort_to_budget))
                {
                    map.insert(
                        "thinking".to_string(),
                        serde_json::json!({ "type": "enabled", "budget_tokens": budget }),
                    );
                }
                map.remove("thinking_budget");
            }
        }

        if matches!(
            provider_type,
            ProviderType::Gemini
                | ProviderType::GeminiCli
                | ProviderType::Vertex
                | ProviderType::AiStudio
                | ProviderType::Antigravity
        ) {
            let disabled = normalized_budget == Some(0)
                || normalized_effort
                    .as_deref()
                    .map(|v| v.eq_ignore_ascii_case("none"))
                    .unwrap_or(false);
            if let Some(map) = Self::ensure_extra_body_object(&mut extra_body) {
                let generation_config = map
                    .entry("generationConfig".to_string())
                    .or_insert_with(|| serde_json::json!({}));
                if !generation_config.is_object() {
                    *generation_config = serde_json::json!({});
                }
                if let Some(generation_map) = generation_config.as_object_mut() {
                    let thinking_config = generation_map
                        .entry("thinkingConfig".to_string())
                        .or_insert_with(|| serde_json::json!({}));
                    if !thinking_config.is_object() {
                        *thinking_config = serde_json::json!({});
                    }
                    if let Some(thinking_map) = thinking_config.as_object_mut() {
                        let budget = if disabled {
                            Some(0)
                        } else {
                            normalized_budget.or_else(|| {
                                normalized_effort.as_deref().map(Self::effort_to_budget)
                            })
                        };
                        if let Some(v) = budget {
                            thinking_map.insert("thinkingBudget".to_string(), serde_json::json!(v));
                        }

                        if let Some(exclude) = reasoning.as_ref().and_then(|cfg| cfg.exclude) {
                            thinking_map
                                .insert("includeThoughts".to_string(), serde_json::json!(!exclude));
                        }
                    }
                }
                map.remove("thinking_budget");
            }
        }

        if *provider_type == ProviderType::Iflow {
            let enable = !(normalized_budget == Some(0)
                || normalized_effort
                    .as_deref()
                    .map(|v| v.eq_ignore_ascii_case("none"))
                    .unwrap_or(false));
            if let Some(map) = Self::ensure_extra_body_object(&mut extra_body) {
                let kwargs = map
                    .entry("chat_template_kwargs".to_string())
                    .or_insert_with(|| serde_json::json!({}));
                if !kwargs.is_object() {
                    *kwargs = serde_json::json!({});
                }
                if let Some(kwargs_map) = kwargs.as_object_mut() {
                    kwargs_map.insert("enable_thinking".to_string(), serde_json::json!(enable));
                }
                map.remove("thinking_budget");
            }
        }

        Self {
            base,
            reasoning_effort,
            extra_body,
        }
    }

    fn normalize_effort(effort: &str) -> Option<String> {
        match effort.trim().to_lowercase().as_str() {
            "none" => Some("none".to_string()),
            "low" | "minimal" => Some("low".to_string()),
            "medium" | "auto" => Some("medium".to_string()),
            "high" | "xhigh" => Some("high".to_string()),
            _ => None,
        }
    }

    fn budget_to_effort(budget: u32) -> String {
        if budget <= 256 {
            "low".to_string()
        } else if budget <= 1024 {
            "medium".to_string()
        } else {
            "high".to_string()
        }
    }

    fn effort_to_budget(effort: &str) -> u32 {
        match effort {
            "low" => 256,
            "high" => 1024,
            _ => 512,
        }
    }

    fn reasoning_effort_from_config(config: &ReasoningConfig) -> Option<String> {
        if let Some(max_tokens) = config.max_tokens {
            return Some(Self::budget_to_effort(max_tokens));
        }
        config
            .effort
            .as_deref()
            .and_then(Self::normalize_effort)
            .filter(|s| !s.eq_ignore_ascii_case("none"))
    }

    fn ensure_extra_body_object(
        extra_body: &mut Option<serde_json::Value>,
    ) -> Option<&mut serde_json::Map<String, serde_json::Value>> {
        if !matches!(extra_body, Some(serde_json::Value::Object(_))) {
            *extra_body = Some(serde_json::Value::Object(serde_json::Map::new()));
        }
        match extra_body {
            Some(serde_json::Value::Object(map)) => Some(map),
            _ => None,
        }
    }
}

impl From<ChatCompletionRequest> for CommonChatCompletionRequest {
    fn from(base: ChatCompletionRequest) -> Self {
        Self::from_for_provider(base, &ProviderType::OpenAI)
    }
}

/// 通用OpenAI兼容供应商实现，支持多种供应商类型
pub struct CommonProvider {
    config: ProviderConfig,
    http_client: Client,
    decrypted_api_key: Option<String>,
}

impl CommonProvider {
    const CODEX_PROMPT_CACHE_KEY_MAX_LEN: usize = 64;

    fn is_codex_provider(&self) -> bool {
        self.config.provider_type == ProviderType::Codex
    }

    fn provider_oauth_type_name(&self) -> String {
        self.config.provider_type.to_string().to_lowercase()
    }

    fn oauth_scope_provider_id(&self) -> &str {
        let id = self.config.id.trim();
        if !id.is_empty() {
            return id;
        }
        self.config.provider_code.trim()
    }

    fn selected_oauth_token(&self) -> Option<SelectedProviderOAuthToken> {
        let provider_type = self.provider_oauth_type_name();
        if provider_type.is_empty() {
            return None;
        }

        select_provider_oauth_token(
            &self.config.config_details,
            self.oauth_scope_provider_id(),
            &provider_type,
        )
    }

    /// 获取API基础URL
    fn base_url(&self) -> String {
        if self.is_codex_provider() {
            if let Some(codex_base) = self
                .config
                .config_details
                .get("codex_base_url")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                return codex_base.to_string();
            }

            if !self.config.api_base.trim().is_empty()
                && self.config.api_base.trim().contains("backend-api/codex")
            {
                return self.config.api_base.trim().to_string();
            }

            return "https://chatgpt.com/backend-api/codex".to_string();
        }

        if !self.config.api_base.is_empty() {
            return self.config.api_base.clone();
        }

        let default_url = match self.config.provider_type {
            ProviderType::OpenAI => "https://api.openai.com/v1",
            ProviderType::Codex => "https://chatgpt.com/backend-api/codex",
            _ => "https://api.openai.com/v1",
        };

        default_url.to_string()
    }

    /// 获取API密钥
    fn api_key(&self) -> String {
        let provider_type = self.provider_oauth_type_name();
        if let Some(selected) = select_provider_oauth_token(
            &self.config.config_details,
            self.oauth_scope_provider_id(),
            &provider_type,
        ) {
            return selected.access_token;
        }

        if self.config.provider_type == ProviderType::Codex {
            if let Some(token) = self
                .config
                .config_details
                .get("codex_oauth")
                .and_then(|v| v.get("access_token"))
                .and_then(|v| v.as_str())
                && !token.is_empty()
            {
                return token.to_string();
            }

            if let Some(token) = self
                .config
                .config_details
                .get("codex_oauth")
                .and_then(|v| v.get("access_token"))
                .and_then(|v| v.as_str())
                && !token.is_empty()
            {
                return token.to_string();
            }
        }

        // 优先使用已解密的API密钥
        if let Some(ref decrypted_key) = self.decrypted_api_key {
            log::info!(
                "[Provider] Using decrypted API key for provider {} (length: {})",
                self.config.provider_code,
                decrypted_key.len()
            );
            return decrypted_key.clone();
        }

        // 如果没有解密的密钥，尝试从加密密钥中获取（但这不应该发生）
        if let Some(ref encrypted_key) = self.config.api_key_encrypted {
            log::warn!(
                "API key not decrypted for provider {}, using encrypted key",
                self.config.id
            );
            return encrypted_key.clone();
        }

        let key = self
            .config
            .config_details
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        log::warn!(
            "Using API key from config_details for provider {} (length: {})",
            self.config.provider_code,
            key.len()
        );
        key
    }

    /// 添加认证头，OpenAI和OpenAI Compatible都使用Bearer访问令牌认证
    fn add_auth_headers(
        &self,
        request_builder: reqwest::RequestBuilder,
        selected_oauth_token: Option<&SelectedProviderOAuthToken>,
        codex_session_id: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let mut codex_account_id = String::new();
        let mut codex_selected = false;
        let provider_type = self.provider_oauth_type_name();

        let (api_key, token_type) = if let Some(selected) = selected_oauth_token {
            if self.config.provider_type == ProviderType::Codex {
                codex_account_id = selected.account_id.clone();
                codex_selected = true;
                log::debug!(
                    "[Provider] Codex selected oauth account={}, provider_code={}",
                    selected.account_key,
                    self.config.provider_code
                );
            }
            (selected.access_token.clone(), selected.token_type.clone())
        } else if self.config.provider_type == ProviderType::Codex {
            let api_key = self.api_key();
            let token_type = self
                .config
                .config_details
                .get("codex_oauth")
                .and_then(|v| v.get("token_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("Bearer")
                .to_string();
            (api_key, token_type)
        } else if let Some(token_type) = self
            .config
            .config_details
            .get(format!("{}_oauth", provider_type.replace('-', "_")))
            .and_then(|v| v.get("token_type"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
        {
            (self.api_key(), token_type)
        } else {
            (self.api_key(), "Bearer".to_string())
        };

        let injection_token_type = self
            .config
            .auth_config
            .as_ref()
            .and_then(|cfg| cfg.get("injection"));

        let token_type = injection_token_type
            .and_then(|inj| inj.get("token_type"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .unwrap_or(token_type);

        let basic_username = injection_token_type
            .and_then(|inj| inj.get("basic_username"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let basic_password = injection_token_type
            .and_then(|inj| inj.get("basic_password"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());

        log::info!(
            "[Provider] Adding Authorization header for provider {} (API key length: {})",
            self.config.provider_code,
            api_key.len()
        );

        let token_header_value = if token_type.eq_ignore_ascii_case("Basic") {
            if let (Some(username), Some(password)) = (basic_username, basic_password) {
                let encoded = base64::engine::general_purpose::STANDARD
                    .encode(format!("{}:{}", username, password));
                format!("Basic {}", encoded)
            } else {
                format!("Basic {}", api_key)
            }
        } else {
            format!("{} {}", token_type, api_key)
        };

        let mut builder = request_builder.header(TOKEN_KEY, token_header_value.clone());

        // --- auth_config 注入（header/query/cookie） ---
        if let Some(auth_config) = &self.config.auth_config {
            if let Some(injection) = auth_config.get("injection") {
                // Header: API key
                if let Some(header_key) = injection
                    .get("api_key_header")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    builder = builder.header(header_key, api_key.clone());
                }

                // Header: Authorization override
                if let Some(auth_header) = injection
                    .get("authorization_header")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    // 重新设置 Authorization header 名称（默认仍然会有 TOKEN_KEY=Authorization）
                    // 这里为了兼容特殊上游，允许追加一个自定义 Authorization-like header。
                    if auth_header != TOKEN_KEY {
                        builder = builder.header(auth_header, token_header_value.clone());
                    }
                }
            }

            // iFlow cookie
            if self.config.provider_type == ProviderType::Iflow {
                if let Some(cookie) = auth_config
                    .get("iflow")
                    .and_then(|v| v.get("cookie"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    builder = builder.header("Cookie", cookie);
                }
            }
        }

        if self.config.provider_type == ProviderType::Codex {
            let default_version = self
                .config
                .config_details
                .get("codex_version")
                .and_then(|v| v.as_str())
                .unwrap_or("1.20250206.0");
            let default_user_agent = self
                .config
                .config_details
                .get("codex_user_agent")
                .and_then(|v| v.as_str())
                .unwrap_or("codex-cli/0.1.2502121828");
            let session_id = codex_session_id
                .map(str::trim)
                .filter(|sid| !sid.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    format!(
                        "{}-{}",
                        self.config.provider_code,
                        Utc::now().timestamp_millis()
                    )
                });

            builder = builder
                .header("Version", default_version)
                .header("Session_id", session_id.clone())
                .header("User-Agent", default_user_agent)
                .header("Connection", "Keep-Alive");

            if codex_session_id
                .map(str::trim)
                .filter(|sid| !sid.is_empty())
                .is_some()
            {
                builder = builder.header("Conversation_id", session_id);
            }

            if codex_selected {
                builder = builder.header("Originator", "codex_cli_rs");
                if !codex_account_id.trim().is_empty() {
                    builder = builder.header("Chatgpt-Account-Id", codex_account_id);
                }
            }
        }

        builder
    }

    /// 构建请求URL
    fn build_request_url(&self, endpoint: &str) -> String {
        let base = format!("{}/{}", self.base_url(), endpoint);

        let Some(auth_config) = &self.config.auth_config else {
            return base;
        };
        let Some(injection) = auth_config.get("injection") else {
            return base;
        };

        let query_key = injection
            .get("api_key_query_key")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let query_auth_token = injection
            .get("api_key_query_auth_token")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty());

        if query_key.is_none() && query_auth_token.is_none() {
            return base;
        }

        let api_key = self.api_key();
        if api_key.trim().is_empty() {
            return base;
        }

        let mut url = match url::Url::parse(&base) {
            Ok(u) => u,
            Err(_) => return base,
        };

        {
            let mut pairs = url.query_pairs_mut();
            if let Some(key_name) = query_key {
                pairs.append_pair(key_name, &api_key);
            }
            if let Some(auth_name) = query_auth_token {
                pairs.append_pair(auth_name, &api_key);
            }
        }

        url.to_string()
    }

    fn codex_chat_endpoint(&self) -> &'static str {
        if self.is_codex_provider() {
            "responses"
        } else {
            "chat/completions"
        }
    }

    fn build_codex_responses_request(
        &self,
        payload: &ChatCompletionRequest,
        common_request: &CommonChatCompletionRequest,
    ) -> serde_json::Value {
        let instructions = payload
            .messages
            .iter()
            .filter(|m| m.role.eq_ignore_ascii_case("system"))
            .filter_map(|m| m.content.as_ref())
            .map(|content| match content {
                ChatMessageContent::String(s) => s.trim().to_string(),
                ChatMessageContent::Array(parts) => parts
                    .iter()
                    .filter(|p| p.r#type == "text")
                    .filter_map(|p| p.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim()
                    .to_string(),
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        let input_messages = {
            let filtered = payload
                .messages
                .iter()
                .filter(|m| !m.role.eq_ignore_ascii_case("system"))
                .cloned()
                .collect::<Vec<_>>();
            if filtered.is_empty() {
                payload.messages.clone()
            } else {
                filtered
            }
        };

        let mut obj = serde_json::Map::new();
        obj.insert(
            "model".to_string(),
            serde_json::Value::String(payload.model.clone()),
        );
        obj.insert(
            "instructions".to_string(),
            serde_json::Value::String(if instructions.is_empty() {
                "You are a helpful assistant.".to_string()
            } else {
                instructions
            }),
        );
        obj.insert(
            "input".to_string(),
            serde_json::to_value(&input_messages).unwrap_or(serde_json::Value::Array(vec![])),
        );
        obj.insert(
            "stream".to_string(),
            serde_json::Value::Bool(payload.stream.unwrap_or(false)),
        );

        if let Some(v) = payload.max_completion_tokens.or(payload.max_tokens)
            && v > 0
        {
            obj.insert(
                "max_output_tokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(v)),
            );
        }

        if let Some(extra) = &common_request.extra_body
            && let Some(extra_obj) = extra.as_object()
        {
            for (k, v) in extra_obj {
                if k == "thinking_budget" {
                    continue;
                }
                obj.insert(k.clone(), v.clone());
            }
        }

        if let Some(reasoning_effort) = &common_request.reasoning_effort {
            obj.insert(
                "reasoning".to_string(),
                serde_json::json!({ "effort": reasoning_effort }),
            );
        }

        let prompt_cache_key = obj
            .get("prompt_cache_key")
            .and_then(|v| v.as_str())
            .map(Self::normalize_prompt_cache_key)
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| self.compute_prompt_cache_key(payload));

        obj.insert(
            "prompt_cache_key".to_string(),
            serde_json::Value::String(prompt_cache_key),
        );

        obj.insert("store".to_string(), serde_json::Value::Bool(false));

        serde_json::Value::Object(obj)
    }

    fn compute_prompt_cache_key(&self, payload: &ChatCompletionRequest) -> String {
        let mut hasher = Sha256::new();
        hasher.update(payload.model.as_bytes());

        for msg in &payload.messages {
            hasher.update(msg.role.as_bytes());
            if let Some(content) = &msg.content {
                match content {
                    ChatMessageContent::String(s) => hasher.update(s.as_bytes()),
                    ChatMessageContent::Array(parts) => {
                        for part in parts {
                            hasher.update(part.r#type.as_bytes());
                            if let Some(text) = &part.text {
                                hasher.update(text.as_bytes());
                            }
                            if let Some(image_url) = &part.image_url {
                                hasher.update(image_url.url.as_bytes());
                            }
                        }
                    }
                }
            }
        }

        format!("{:x}", hasher.finalize())
    }

    fn normalize_prompt_cache_key(raw: &str) -> String {
        let normalized = raw.trim();
        if normalized.is_empty() {
            return String::new();
        }

        if normalized.len() <= Self::CODEX_PROMPT_CACHE_KEY_MAX_LEN {
            return normalized.to_string();
        }

        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn extract_codex_session_id(request_body: &serde_json::Value) -> Option<String> {
        request_body
            .get("prompt_cache_key")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .or_else(|| {
                request_body
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)
            })
    }

    fn build_chat_completion_from_codex_response(
        &self,
        payload: &ChatCompletionRequest,
        response_json: serde_json::Value,
    ) -> crate::domain::vo::chat::ChatCompletion {
        let output_text = response_json
            .get("output_text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let usage_value = response_json.get("usage").cloned().unwrap_or_default();
        let prompt_tokens = usage_value
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .or_else(|| usage_value.get("input_tokens").and_then(|v| v.as_u64()))
            .unwrap_or(0) as u32;
        let completion_tokens = usage_value
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .or_else(|| usage_value.get("output_tokens").and_then(|v| v.as_u64()))
            .unwrap_or(0) as u32;
        let total_tokens = usage_value
            .get("total_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or((prompt_tokens + completion_tokens) as u64)
            as u32;

        let message = ChatCompletionMessage {
            role: "assistant".to_string(),
            content: Some(ChatMessageContent::String(output_text)),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
            reasoning_content: None,
            extra_fields: serde_json::Value::default(),
        };

        crate::domain::vo::chat::ChatCompletion {
            id: response_json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("resp-codex")
                .to_string(),
            object: Some("chat.completion".to_string()),
            created: Some(
                response_json
                    .get("created")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(Utc::now().timestamp() as u64),
            ),
            model: payload.model.clone(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message,
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
            system_fingerprint: None,
        }
    }
}

#[async_trait]
impl Provider for CommonProvider {
    /// 创建通用供应商实例
    fn new(config: &ProviderConfig) -> Self {
        let timeout_seconds = config
            .config_details
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(120);

        let timeout = Duration::from_secs(timeout_seconds);

        let http_client = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(8))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(16)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            config: config.clone(),
            http_client,
            decrypted_api_key: None,
        }
    }

    /// 获取供应商ID
    fn id(&self) -> String {
        self.config.id.clone()
    }

    /// 获取供应商名称
    fn name(&self) -> String {
        self.config.name.clone()
    }

    /// 获取供应商类型
    fn r#type(&self) -> ProviderType {
        self.config.provider_type.clone()
    }

    /// 设置解密后的API密钥
    fn set_decrypted_api_key(&mut self, decrypted_key: String) {
        self.decrypted_api_key = Some(decrypted_key);
    }

    /// 聊天补全方法
    async fn chat_completions(
        &self,
        payload: ChatCompletionRequest,
        model_config: &serde_json::Value,
    ) -> Result<ChatCompletionResponse, StatusCode> {
        // 转换为通用请求格式
        let common_request = CommonChatCompletionRequest::from_for_provider(
            payload.clone(),
            &self.config.provider_type,
        );

        let request_url = self.build_request_url(self.codex_chat_endpoint());
        log::info!(
            "[Provider] Sending chat completion request to: {}",
            request_url
        );
        log::info!("[Provider] Request model: {}", payload.model);

        let request_body = if self.is_codex_provider() {
            self.build_codex_responses_request(&payload, &common_request)
        } else {
            serde_json::to_value(&common_request).unwrap_or_default()
        };

        let codex_session_id = if self.is_codex_provider() {
            Self::extract_codex_session_id(&request_body)
        } else {
            None
        };

        let request_builder = self.http_client.post(&request_url);
        let request_builder = if self.is_codex_provider() && payload.stream.unwrap_or(false) {
            let stream_timeout_secs = self
                .config
                .config_details
                .get("codex_stream_timeout")
                .and_then(|v| v.as_u64())
                .unwrap_or(300);
            request_builder.timeout(Duration::from_secs(stream_timeout_secs.max(30)))
        } else {
            request_builder
        };

        // 添加认证头
        let selected_oauth_token = self.selected_oauth_token();

        // 记录本次请求选中的上游 OAuth 账号（仅当 controller 透传了 trace_key 时）
        if let Some(selected) = &selected_oauth_token {
            if let Some(trace_key) = model_config
                .get("rsllm")
                .and_then(|v| v.get("trace_key"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                crate::service::ai_hub::provider::upstream_trace::record_upstream_oauth(
                    trace_key,
                    crate::service::ai_hub::provider::upstream_trace::UpstreamOAuthInfo {
                        provider_id: self.oauth_scope_provider_id().to_string(),
                        provider_type: self.provider_oauth_type_name(),
                        account_key: selected.account_key.clone(),
                        account_id: selected.account_id.clone(),
                        email: selected.email.clone(),
                    },
                );
            }
        }

        let request_builder = self.add_auth_headers(
            request_builder,
            selected_oauth_token.as_ref(),
            codex_session_id.as_deref(),
        );
        let request_builder = if self.config.provider_type == ProviderType::Codex {
            if payload.stream.unwrap_or(false) {
                request_builder.header("Accept", "text/event-stream")
            } else {
                request_builder.header("Accept", "application/json")
            }
        } else {
            request_builder
        };

        log::info!("[Provider] Sending request with model: {}", payload.model);

        // 打印完整的请求体用于调试
        let request_json = serde_json::to_string(&request_body).unwrap_or_default();
        log::info!("[Provider] Full request body: {}", request_json);

        let response = request_builder
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                if let Some(selected) = &selected_oauth_token {
                    record_provider_request_result(
                        self.oauth_scope_provider_id(),
                        &self.provider_oauth_type_name(),
                        &selected.account_key,
                        false,
                    );
                }
                error!(
                    "{} provider API request error: {e}",
                    self.r#type().to_string()
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let status = response.status();
        log::info!("[Provider] Response status: {}", status);

        if !status.is_success() {
            if let Some(selected) = &selected_oauth_token {
                record_provider_request_result(
                    self.oauth_scope_provider_id(),
                    &self.provider_oauth_type_name(),
                    &selected.account_key,
                    false,
                );
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    mark_provider_account_unauthorized(
                        self.oauth_scope_provider_id(),
                        &self.provider_oauth_type_name(),
                        &selected.account_key,
                    );
                    log::warn!(
                        "[Provider] OAuth account marked unauthorized cooldown: account_key={}, provider_code={}",
                        selected.account_key,
                        self.config.provider_code
                    );
                }
            }
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());
            log::error!("[Provider] Error response body: {}", error_body);
            return Err(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            );
        }

        if status.is_success() {
            if let Some(selected) = &selected_oauth_token {
                record_provider_request_result(
                    self.oauth_scope_provider_id(),
                    &self.provider_oauth_type_name(),
                    &selected.account_key,
                    true,
                );
            }
            // 检查是否为流式请求
            if payload.stream.unwrap_or(false) {
                if self.is_codex_provider() {
                    let model_name = payload.model.clone();
                    let byte_stream = response.bytes_stream();
                    let stream = byte_stream
                        .map(move |bytes| match bytes {
                            Ok(chunk) => {
                                let text = String::from_utf8_lossy(&chunk);
                                let mut chunks = Vec::new();

                                for line in text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if data == "[DONE]" {
                                            continue;
                                        }

                                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                                            let event_type = event
                                                .get("type")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default();

                                            if event_type == "response.output_text.delta" {
                                                let delta_text = event
                                                    .get("delta")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or_default();

                                                if !delta_text.is_empty() {
                                                    chunks.push(Ok(crate::domain::dto::ai_hub::streaming::ChatCompletionChunk {
                                                        id: event
                                                            .get("response_id")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("chatcmpl-codex")
                                                            .to_string(),
                                                        object: Some("chat.completion.chunk".to_string()),
                                                        created: Utc::now().timestamp() as u64,
                                                        model: model_name.clone(),
                                                        system_fingerprint: None,
                                                        choices: vec![crate::domain::dto::ai_hub::streaming::ChatCompletionChunkChoice {
                                                            index: 0,
                                                            delta: Some(crate::domain::dto::ai_hub::streaming::ChatCompletionChunkDelta {
                                                                role: None,
                                                                content: Some(delta_text.to_string()),
                                                                reasoning_content: None,
                                                                tool_calls: None,
                                                                extra_fields: serde_json::Value::default(),
                                                            }),
                                                            finish_reason: None,
                                                            logprobs: None,
                                                            extra_fields: serde_json::Value::default(),
                                                        }],
                                                        extra_fields: serde_json::Value::default(),
                                                    }));
                                                }
                                            } else if event_type == "response.completed" {
                                                chunks.push(Ok(crate::domain::dto::ai_hub::streaming::ChatCompletionChunk {
                                                    id: event
                                                        .get("response")
                                                        .and_then(|v| v.get("id"))
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("chatcmpl-codex")
                                                        .to_string(),
                                                    object: Some("chat.completion.chunk".to_string()),
                                                    created: Utc::now().timestamp() as u64,
                                                    model: model_name.clone(),
                                                    system_fingerprint: None,
                                                    choices: vec![crate::domain::dto::ai_hub::streaming::ChatCompletionChunkChoice {
                                                        index: 0,
                                                        delta: None,
                                                        finish_reason: Some("stop".to_string()),
                                                        logprobs: None,
                                                        extra_fields: serde_json::Value::default(),
                                                    }],
                                                    extra_fields: serde_json::Value::default(),
                                                }));
                                            }
                                        }
                                    }
                                }

                                chunks
                            }
                            Err(e) => vec![Err(e)],
                        })
                        .flat_map(futures::stream::iter);

                    return Ok(ChatCompletionResponse::Stream(Box::pin(stream)));
                }

                // 支持流式响应
                info!("{} streaming response enabled", self.r#type().to_string());

                // 获取字节流
                let byte_stream = response.bytes_stream();

                // 将字节流转换为 SSE 事件流
                let stream = byte_stream
                    .map(move |bytes| {
                        match bytes {
                            Ok(chunk) => {
                                // 将字节块转换为字符串
                                let text = String::from_utf8_lossy(&chunk);
                                info!("Raw received data: {}", text);
                                let mut chunks = Vec::new();
                                // 解析 SSE 格式的数据，收集所有事件
                                for line in text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        // 去掉 "data: " 前缀
                                        if data == "[DONE]" {
                                            // 流结束
                                            continue;
                                        } else {
                                            // 解析 JSON
                                            if let Ok(chunk) = serde_json::from_str::<crate::domain::dto::ai_hub::streaming::ChatCompletionChunk>(data) {
                                                // 调试日志：打印接收到的chunk
                                                info!("Received chunk: {}", serde_json::to_string(&chunk).unwrap_or_default());

                                                // 过滤空内容的chunk，但要保留包含额外字段的chunk
                                                let should_include = chunk.choices.iter().any(|choice| {
                                                    if let Some(ref delta) = choice.delta {
                                                        // 检查content是否非空
                                                        let has_content = delta.content.as_ref().map_or(false, |s| !s.is_empty());

                                                        // 检查reasoning_content是否非空
                                                        let has_reasoning = delta.reasoning_content.as_ref().map_or(false, |s| !s.is_empty());

                                                        // 检查extra_fields是否非空
                                                        let has_extra_fields = !delta.extra_fields.is_null() && !delta.extra_fields.as_object().map_or(true, |obj| obj.is_empty());

                                                        // 检查是否有tool_calls
                                                        let has_tool_calls = delta.tool_calls.as_ref().map_or(false, |calls| !calls.is_empty());

                                                        // 检查role是否非空（OpenAI兼容性：初始chunk可能只有role字段）
                                                        let has_role = delta.role.as_ref().map_or(false, |s| !s.is_empty());

                                                        has_content || has_reasoning || has_extra_fields || has_tool_calls || has_role
                                                    } else {
                                                        false
                                                    }
                                                }) || chunk.choices.iter().any(|choice| choice.finish_reason.is_some()) || !chunk.extra_fields.is_null() && !chunk.extra_fields.as_object().map_or(true, |obj| obj.is_empty());

                                                info!("Chunk should_include: {}", should_include);

                                                if should_include {
                                                    chunks.push(Ok(chunk));
                                                }
                                            }
                                        }
                                    }
                                }
                                chunks
                            }
                            Err(e) => vec![Err(e)],
                        }
                    })
                    .flat_map(futures::stream::iter);

                // 返回流式响应
                Ok(ChatCompletionResponse::Stream(Box::pin(stream)))
            } else {
                if self.is_codex_provider() {
                    let response_json: serde_json::Value = response.json().await.map_err(|e| {
                        error!(
                            "{} provider API response error: {e}",
                            self.r#type().to_string()
                        );
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                    let chat_response =
                        self.build_chat_completion_from_codex_response(&payload, response_json);
                    return Ok(ChatCompletionResponse::NonStream(chat_response));
                }

                // 获取响应内容作为JSON
                let chat_response: crate::domain::vo::chat::ChatCompletion =
                    response.json().await.map_err(|e| {
                        error!(
                            "{} provider API response error: {e}",
                            self.r#type().to_string()
                        );
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                // 返回非流式响应
                Ok(ChatCompletionResponse::NonStream(chat_response))
            }
        } else {
            let error_msg = response.text().await.unwrap_or_default();
            info!(
                "{} provider API request error: {}",
                self.r#type().to_string(),
                error_msg
            );
            Err(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }

    /// 嵌入生成方法
    async fn embeddings(
        &self,
        payload: EmbeddingsRequest,
        model_config: &serde_json::Value,
    ) -> Result<EmbeddingsResponse, StatusCode> {
        let request_builder = self.http_client.post(self.build_request_url("embeddings"));

        // 添加认证头
        let selected_oauth_token = self.selected_oauth_token();

        // 记录本次请求选中的上游 OAuth 账号（仅当 controller 透传了 trace_key 时）
        if let Some(selected) = &selected_oauth_token {
            if let Some(trace_key) = model_config
                .get("rsllm")
                .and_then(|v| v.get("trace_key"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                crate::service::ai_hub::provider::upstream_trace::record_upstream_oauth(
                    trace_key,
                    crate::service::ai_hub::provider::upstream_trace::UpstreamOAuthInfo {
                        provider_id: self.oauth_scope_provider_id().to_string(),
                        provider_type: self.provider_oauth_type_name(),
                        account_key: selected.account_key.clone(),
                        account_id: selected.account_id.clone(),
                        email: selected.email.clone(),
                    },
                );
            }
        }

        let request_builder =
            self.add_auth_headers(request_builder, selected_oauth_token.as_ref(), None);
        let request_builder = if self.config.provider_type == ProviderType::Codex {
            request_builder.header("Accept", "application/json")
        } else {
            request_builder
        };

        let response = request_builder.json(&payload).send().await.map_err(|e| {
            if let Some(selected) = &selected_oauth_token {
                record_provider_request_result(
                    self.oauth_scope_provider_id(),
                    &self.provider_oauth_type_name(),
                    &selected.account_key,
                    false,
                );
            }
            error!(
                "{} provider API request error: {e}",
                self.r#type().to_string()
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let status = response.status();
        if status.is_success() {
            if let Some(selected) = &selected_oauth_token {
                record_provider_request_result(
                    self.oauth_scope_provider_id(),
                    &self.provider_oauth_type_name(),
                    &selected.account_key,
                    true,
                );
            }
            response.json().await.map_err(|e| {
                error!(
                    "{} provider API response error: {e}",
                    self.r#type().to_string()
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })
        } else {
            if let Some(selected) = &selected_oauth_token {
                record_provider_request_result(
                    self.oauth_scope_provider_id(),
                    &self.provider_oauth_type_name(),
                    &selected.account_key,
                    false,
                );
            }
            let error_msg = response.text().await.unwrap_or_default();
            error!(
                "{} provider API request error: {}",
                self.r#type().to_string(),
                error_msg
            );
            Err(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dto::ai_hub::content::ChatCompletionMessage;
    use crate::domain::dto::ai_hub::content::ChatMessageContent;
    use crate::domain::dto::chat::ReasoningConfig;

    fn build_codex_provider() -> CommonProvider {
        let config = ProviderConfig {
            id: "provider-codex-test".to_string(),
            provider_code: "openai".to_string(),
            name: "OpenAI Codex".to_string(),
            provider_type: ProviderType::Codex,
            api_base: "https://chatgpt.com/backend-api/codex".to_string(),
            api_key_encrypted: Some("test-token".to_string()),
            auth_config: Some(serde_json::json!({})),
            config_details: serde_json::json!({}),
            status: Some("active".to_string()),
            documentation_url: None,
        };

        CommonProvider::new(&config)
    }

    fn build_basic_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: None,
            extra_fields: serde_json::Value::default(),
        }
    }

    #[test]
    fn test_openai_reasoning_maps_to_reasoning_effort() {
        let mut request = build_basic_request();
        request.reasoning = Some(ReasoningConfig {
            effort: Some("high".to_string()),
            max_tokens: None,
            exclude: None,
        });

        let common_request =
            CommonChatCompletionRequest::from_for_provider(request, &ProviderType::OpenAI);
        let serialized = serde_json::to_value(&common_request).unwrap();

        assert_eq!(serialized["reasoning_effort"], "high");
    }

    #[test]
    fn test_claude_reasoning_maps_to_thinking_object() {
        let mut request = build_basic_request();
        request.reasoning = Some(ReasoningConfig {
            effort: None,
            max_tokens: Some(900),
            exclude: None,
        });
        request.extra_body = Some(serde_json::json!({ "thinking_budget": "high" }));

        let common_request =
            CommonChatCompletionRequest::from_for_provider(request, &ProviderType::Claude);
        let serialized = serde_json::to_value(&common_request).unwrap();

        assert_eq!(serialized["thinking"]["type"], "enabled");
        assert_eq!(serialized["thinking"]["budget_tokens"], 900);
        assert!(serialized.get("thinking_budget").is_none());
    }

    #[test]
    fn test_gemini_reasoning_maps_to_generation_config_thinking_config() {
        let mut request = build_basic_request();
        request.reasoning = Some(ReasoningConfig {
            effort: Some("medium".to_string()),
            max_tokens: Some(640),
            exclude: Some(true),
        });

        let common_request =
            CommonChatCompletionRequest::from_for_provider(request, &ProviderType::Gemini);
        let serialized = serde_json::to_value(&common_request).unwrap();

        assert_eq!(
            serialized["generationConfig"]["thinkingConfig"]["thinkingBudget"],
            640
        );
        assert_eq!(
            serialized["generationConfig"]["thinkingConfig"]["includeThoughts"],
            false
        );
    }

    #[test]
    fn test_iflow_reasoning_maps_to_enable_thinking() {
        let mut request = build_basic_request();
        request.reasoning = Some(ReasoningConfig {
            effort: Some("low".to_string()),
            max_tokens: None,
            exclude: None,
        });

        let common_request =
            CommonChatCompletionRequest::from_for_provider(request, &ProviderType::Iflow);
        let serialized = serde_json::to_value(&common_request).unwrap();

        assert_eq!(serialized["chat_template_kwargs"]["enable_thinking"], true);
    }

    #[test]
    fn test_codex_request_contains_reasoning_effort_payload() {
        let provider = build_codex_provider();
        let mut request = build_basic_request();
        request.model = "gpt-5.3-codex".to_string();
        request.stream = Some(true);
        request.reasoning = Some(ReasoningConfig {
            effort: Some("high".to_string()),
            max_tokens: None,
            exclude: None,
        });

        let common_request =
            CommonChatCompletionRequest::from_for_provider(request.clone(), &ProviderType::Codex);
        let payload = provider.build_codex_responses_request(&request, &common_request);

        assert_eq!(payload["reasoning"]["effort"], "high");
    }

    #[test]
    fn test_common_chat_completion_request_serialization_with_extra_body() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": 1000,
                "custom_param": "value"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized["model"], "test-model");
        assert_eq!(deserialized["thinking_budget"], 1000);
        assert_eq!(deserialized["custom_param"], "value");
        assert_eq!(deserialized["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_common_chat_completion_request_without_extra_body() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: None,
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized["model"], "test-model");
        assert!(
            deserialized.get("thinking_budget").is_none(),
            "thinking_budget should not be present when extra_body is None"
        );
        assert_eq!(deserialized["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_common_chat_completion_request_extra_body_override() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "temperature": 1.0,
                "max_tokens": 200
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["temperature"], 1.0,
            "extra_body should override base temperature"
        );
        assert_eq!(
            deserialized["max_tokens"], 200,
            "extra_body should override base max_tokens"
        );
    }

    #[test]
    fn test_thinking_budget_string_conversion_high() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "high"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 1024,
            "thinking_budget 'high' should be converted to 1024"
        );
    }

    #[test]
    fn test_thinking_budget_string_conversion_medium() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "medium"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 512,
            "thinking_budget 'medium' should be converted to 512"
        );
    }

    #[test]
    fn test_thinking_budget_string_conversion_low() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "low"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 256,
            "thinking_budget 'low' should be converted to 256"
        );
    }

    #[test]
    fn test_thinking_budget_number_unchanged() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": 2048
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 2048,
            "thinking_budget number should remain unchanged"
        );
    }

    #[test]
    fn test_thinking_budget_invalid_string_removed() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "invalid"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert!(
            deserialized.get("thinking_budget").is_none(),
            "thinking_budget should be removed for invalid string"
        );
    }

    #[test]
    fn test_codex_request_includes_generated_prompt_cache_key() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let provider = build_codex_provider();
        let request = ChatCompletionRequest {
            model: "gpt-5.3-codex".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(true),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: None,
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request.clone());
        let payload = provider.build_codex_responses_request(&request, &common_request);

        let prompt_cache_key = payload
            .get("prompt_cache_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        assert!(!prompt_cache_key.is_empty());
        assert_eq!(prompt_cache_key.len(), 64);
    }

    #[test]
    fn test_codex_request_keeps_custom_prompt_cache_key() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let provider = build_codex_provider();
        let request = ChatCompletionRequest {
            model: "gpt-5.3-codex".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(true),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({"prompt_cache_key": "my-cache-key"})),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request.clone());
        let payload = provider.build_codex_responses_request(&request, &common_request);

        assert_eq!(
            payload
                .get("prompt_cache_key")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "my-cache-key"
        );
    }

    #[test]
    fn test_codex_request_normalizes_too_long_prompt_cache_key() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let provider = build_codex_provider();
        let request = ChatCompletionRequest {
            model: "gpt-5.3-codex".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(true),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "prompt_cache_key": "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-TOO-LONG"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request.clone());
        let payload = provider.build_codex_responses_request(&request, &common_request);
        let prompt_cache_key = payload
            .get("prompt_cache_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        assert_eq!(prompt_cache_key.len(), 64);
    }
}
