use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::vo::ai_hub::BalanceVO;

pub const REQUEST_CONTEXT_API_KEY_ID_HEADER: &str = "x-rsllm-api-key-id";
pub const REQUEST_CONTEXT_USER_ID_HEADER: &str = "x-rsllm-user-id";
pub const REQUEST_CONTEXT_USER_LEVEL_HEADER: &str = "x-rsllm-user-level";

#[derive(Debug, Clone)]
pub struct AiRequestContext {
    pub api_key: String,
    pub api_key_id: Option<String>,
    pub user_id: String,
    pub user_level: Option<String>,
    pub balance_snapshot: BalanceVO,
}

#[derive(Debug, Clone)]
pub struct ResolvedAiRequestIdentity {
    pub api_key: String,
    pub api_key_id: Option<String>,
    pub user_id: String,
    pub user_level: Option<String>,
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub fn attach_request_context_headers(
    headers: &mut HeaderMap,
    context: &AiRequestContext,
) -> Result<(), String> {
    let user_id_header = HeaderValue::from_str(&context.user_id)
        .map_err(|e| format!("Invalid user ID header: {}", e))?;
    headers.insert(HeaderName::from_static("x-user-id"), user_id_header.clone());
    headers.insert(
        HeaderName::from_static(REQUEST_CONTEXT_USER_ID_HEADER),
        user_id_header,
    );

    if let Some(api_key_id) = &context.api_key_id {
        let header = HeaderValue::from_str(api_key_id)
            .map_err(|e| format!("Invalid API key ID header: {}", e))?;
        headers.insert(
            HeaderName::from_static(REQUEST_CONTEXT_API_KEY_ID_HEADER),
            header,
        );
    }

    if let Some(user_level) = &context.user_level {
        let header = HeaderValue::from_str(user_level)
            .map_err(|e| format!("Invalid user level header: {}", e))?;
        headers.insert(
            HeaderName::from_static(REQUEST_CONTEXT_USER_LEVEL_HEADER),
            header,
        );
    }

    Ok(())
}

pub fn extract_api_key(headers: &HeaderMap) -> Result<String, String> {
    if let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
    {
        return Ok(token);
    }

    if let Some(token) = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        return Ok(token);
    }

    if let Some(token) = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        return Ok(token);
    }

    Err("Missing or invalid authorization header".to_string())
}

fn identity_from_headers(headers: &HeaderMap) -> Option<ResolvedAiRequestIdentity> {
    let user_id = header_value(headers, REQUEST_CONTEXT_USER_ID_HEADER)
        .or_else(|| header_value(headers, "x-user-id"))?;
    let api_key = extract_api_key(headers).ok()?;

    Some(ResolvedAiRequestIdentity {
        api_key,
        api_key_id: header_value(headers, REQUEST_CONTEXT_API_KEY_ID_HEADER),
        user_id,
        user_level: header_value(headers, REQUEST_CONTEXT_USER_LEVEL_HEADER),
    })
}

pub async fn resolve_request_identity(
    headers: &HeaderMap,
    state: &Arc<ServiceContext>,
    request_context: Option<&AiRequestContext>,
) -> Result<ResolvedAiRequestIdentity, String> {
    let api_key = extract_api_key(headers)?;

    if let Some(context) = request_context {
        return Ok(ResolvedAiRequestIdentity {
            api_key,
            api_key_id: context.api_key_id.clone(),
            user_id: context.user_id.clone(),
            user_level: context.user_level.clone(),
        });
    }

    if let Some(identity) = identity_from_headers(headers) {
        return Ok(identity);
    }

    let validation_result = state
        .api_key_service
        .validate_api_key(&api_key)
        .await
        .map_err(|e| {
            log::warn!("[AI Hub] API key validation failed: {}", e);
            "Invalid API key".to_string()
        })?;

    if !validation_result.valid {
        return Err(validation_result
            .error
            .unwrap_or_else(|| "Invalid API key".to_string()));
    }

    let user_id = validation_result
        .user_id
        .ok_or_else(|| "User ID not found".to_string())?;

    Ok(ResolvedAiRequestIdentity {
        api_key,
        api_key_id: validation_result.api_key_id,
        user_id,
        user_level: validation_result.user_level,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn build_request_context() -> AiRequestContext {
        AiRequestContext {
            api_key: "sk-test-key".to_string(),
            api_key_id: Some("key-1".to_string()),
            user_id: "user-1".to_string(),
            user_level: Some("L2".to_string()),
            balance_snapshot: BalanceVO {
                user_id: "user-1".to_string(),
                username: "tester".to_string(),
                balance: 99.0,
                subscription_quota_total: 0.0,
                subscription_quota_used: 0.0,
                subscription_quota_remaining: 0.0,
                subscription_status: "none".to_string(),
                subscription_expires_at: None,
                subscription_next_reset_at: None,
            },
        }
    }

    #[test]
    fn attach_request_context_headers_should_expose_reusable_identity_headers() {
        let context = build_request_context();
        let mut headers = HeaderMap::new();

        attach_request_context_headers(&mut headers, &context)
            .expect("request context headers should be attached");

        assert_eq!(
            header_value(&headers, "x-user-id").as_deref(),
            Some("user-1")
        );
        assert_eq!(
            header_value(&headers, REQUEST_CONTEXT_USER_ID_HEADER).as_deref(),
            Some("user-1")
        );
        assert_eq!(
            header_value(&headers, REQUEST_CONTEXT_API_KEY_ID_HEADER).as_deref(),
            Some("key-1")
        );
        assert_eq!(
            header_value(&headers, REQUEST_CONTEXT_USER_LEVEL_HEADER).as_deref(),
            Some("L2")
        );
    }

    #[test]
    fn extract_api_key_should_support_bearer_and_x_api_key_headers() {
        let mut bearer_headers = HeaderMap::new();
        bearer_headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer sk-bearer"),
        );
        assert_eq!(
            extract_api_key(&bearer_headers).expect("bearer auth should parse"),
            "sk-bearer"
        );

        let mut api_key_headers = HeaderMap::new();
        api_key_headers.insert("x-api-key", HeaderValue::from_static("sk-direct"));
        assert_eq!(
            extract_api_key(&api_key_headers).expect("x-api-key should parse"),
            "sk-direct"
        );
    }

    #[tokio::test]
    async fn resolve_request_identity_should_prefer_request_context_over_db_lookup() {
        let request_context = build_request_context();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer sk-request"),
        );

        let state = Arc::new(ServiceContext::default());
        let identity = resolve_request_identity(&headers, &state, Some(&request_context))
            .await
            .expect("request context should resolve identity");

        assert_eq!(identity.api_key, "sk-request");
        assert_eq!(identity.api_key_id.as_deref(), Some("key-1"));
        assert_eq!(identity.user_id, "user-1");
        assert_eq!(identity.user_level.as_deref(), Some("L2"));
    }

    #[tokio::test]
    async fn resolve_request_identity_should_reuse_identity_headers_when_context_is_absent() {
        let request_context = build_request_context();
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer sk-header"),
        );
        attach_request_context_headers(&mut headers, &request_context)
            .expect("request context headers should be attached");

        let state = Arc::new(ServiceContext::default());
        let identity = resolve_request_identity(&headers, &state, None)
            .await
            .expect("identity headers should resolve without a db lookup");

        assert_eq!(identity.api_key, "sk-header");
        assert_eq!(identity.api_key_id.as_deref(), Some("key-1"));
        assert_eq!(identity.user_id, "user-1");
        assert_eq!(identity.user_level.as_deref(), Some("L2"));
    }
}
