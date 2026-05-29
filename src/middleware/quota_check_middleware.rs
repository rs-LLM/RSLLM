use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::middleware::auth_axum::TOKEN_KEY;
use crate::service::ai_hub::{AiRequestContext, attach_request_context_headers};

const API_KEY_HEADER: &str = "X-API-Key";

pub async fn quota_check_middleware(
    State(state): State<Arc<ServiceContext>>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let headers = request.headers();

    let api_key = match get_api_key(headers) {
        Some(key) => key.to_string(),
        None => {
            return Err(create_quota_error_for_request(
                &request,
                StatusCode::UNAUTHORIZED,
                "Missing API key".to_string(),
            ));
        }
    };

    let validation_result = state.api_key_service.validate_api_key(&api_key).await;

    match validation_result {
        Ok(result) if result.valid => {
            if let Some(user_id) = result.user_id {
                match state.balance_service.get_balance(&user_id).await {
                    Ok(balance_vo) if balance_vo.balance < 0.0 => {
                        let error_msg = format!(
                            "Insufficient balance. Current balance: {:.2}",
                            balance_vo.balance
                        );
                        Err(create_quota_error_for_request(
                            &request,
                            StatusCode::PAYMENT_REQUIRED,
                            error_msg,
                        ))
                    }
                    Ok(balance_snapshot) => {
                        let request_context = AiRequestContext {
                            api_key: api_key.clone(),
                            api_key_id: result.api_key_id,
                            user_id,
                            user_level: result.user_level,
                            balance_snapshot,
                        };
                        attach_request_context_headers(request.headers_mut(), &request_context)
                            .map_err(|e| {
                                create_quota_error_for_request(&request, StatusCode::BAD_REQUEST, e)
                            })?;
                        request.extensions_mut().insert(request_context);
                        Ok(next.run(request).await)
                    }
                    Err(e) => Err(create_quota_error_for_request(
                        &request,
                        StatusCode::BAD_REQUEST,
                        format!("Failed to check balance: {}", e),
                    )),
                }
            } else {
                Err(create_quota_error_for_request(
                    &request,
                    StatusCode::UNAUTHORIZED,
                    "User ID not found".to_string(),
                ))
            }
        }
        Ok(result) => Err(create_quota_error_for_request(
            &request,
            StatusCode::UNAUTHORIZED,
            result
                .error
                .unwrap_or_else(|| "Invalid API key".to_string()),
        )),
        Err(e) => Err(create_quota_error_for_request(
            &request,
            StatusCode::UNAUTHORIZED,
            format!("API key validation failed: {}", e),
        )),
    }
}

pub async fn quota_check_with_tokens_middleware(
    State(state): State<Arc<ServiceContext>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    quota_check_middleware(State(state), request, next).await
}

fn is_claude_messages_request(request: &Request) -> bool {
    matches!(request.uri().path(), "/v1/messages" | "/api/v1/messages")
}

fn create_claude_error(status: StatusCode, message: String) -> Response {
    let error_type = match status {
        StatusCode::BAD_REQUEST => "invalid_request_error",
        StatusCode::UNAUTHORIZED => "authentication_error",
        StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
        StatusCode::PAYMENT_REQUIRED => "billing_error",
        _ => "api_error",
    };

    let mut response = Json(serde_json::json!({
        "error": {
            "type": error_type,
            "message": message,
        }
    }))
    .into_response();
    *response.status_mut() = status;
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("anthropic-version"),
        axum::http::HeaderValue::from_static("2023-06-01"),
    );
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("cache-control"),
        axum::http::HeaderValue::from_static("no-store, no-cache"),
    );
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("pragma"),
        axum::http::HeaderValue::from_static("no-cache"),
    );
    response.headers_mut().insert(
        axum::http::HeaderName::from_static("x-accel-buffering"),
        axum::http::HeaderValue::from_static("no"),
    );
    response
}

fn create_quota_error_for_request(
    request: &Request,
    status: StatusCode,
    message: String,
) -> Response {
    if is_claude_messages_request(request) {
        create_claude_error(status, message)
    } else {
        let error_response = Json(serde_json::json!({
            "code": status.as_u16().to_string(),
            "msg": message,
            "data": null
        }));
        let mut response = error_response.into_response();
        *response.status_mut() = status;
        response
    }
}

fn get_api_key(h: &HeaderMap) -> Option<&str> {
    h.get(TOKEN_KEY)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| h.get(API_KEY_HEADER).and_then(|v| v.to_str().ok()))
        .or_else(|| h.get("x-api-key").and_then(|v| v.to_str().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, body::to_bytes, http::Request as HttpRequest};

    #[tokio::test]
    async fn create_quota_error_for_claude_messages_uses_claude_shape() {
        let request = Request::from(
            HttpRequest::builder()
                .uri("/v1/messages")
                .body(Body::empty())
                .unwrap(),
        );

        let response = create_quota_error_for_request(
            &request,
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded".to_string(),
        );

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get("anthropic-version")
                .and_then(|v| v.to_str().ok()),
            Some("2023-06-01")
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["type"], "rate_limit_error");
        assert_eq!(json["error"]["message"], "Rate limit exceeded");
    }

    #[tokio::test]
    async fn create_quota_error_for_non_claude_messages_uses_common_shape() {
        let request = Request::from(
            HttpRequest::builder()
                .uri("/v1/chat/completions")
                .body(Body::empty())
                .unwrap(),
        );

        let response = create_quota_error_for_request(
            &request,
            StatusCode::UNAUTHORIZED,
            "Missing API key".to_string(),
        );

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get("anthropic-version").is_none());

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "401");
        assert_eq!(json["msg"], "Missing API key");
        assert!(json["data"].is_null());
    }
}
