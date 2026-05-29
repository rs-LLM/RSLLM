use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::middleware::auth_axum::TOKEN_KEY;
use crate::service::ai_hub::{RateLimitCheckResult, UserLevelModelRateLimitService};

const API_KEY_HEADER: &str = "X-API-Key";

pub async fn rate_limit_middleware(
    State(state): State<Arc<ServiceContext>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let headers = request.headers();

    let api_key = match get_api_key(headers) {
        Some(key) => key,
        None => {
            return Err(create_rate_limit_error("Missing API key".to_string()));
        }
    };

    let validation_result = state.api_key_service.validate_api_key(api_key).await;

    match validation_result {
        Ok(result) if result.valid => {
            if let Some(user_id) = result.user_id {
                let rate_limit_result = state.rate_limit_service.check_quota(&user_id).await;

                match rate_limit_result {
                    Ok(RateLimitCheckResult {
                        allowed: true,
                        warning,
                        ..
                    }) => {
                        let response = next.run(request).await;

                        if let Some(warning_msg) = warning {
                            let mut response = response;
                            if let Ok(header_value) = HeaderValue::from_str(&warning_msg) {
                                response
                                    .headers_mut()
                                    .insert("X-RateLimit-Warning", header_value);
                            }
                            Ok(response)
                        } else {
                            Ok(response)
                        }
                    }
                    Ok(RateLimitCheckResult {
                        allowed: false,
                        rpm_remaining,
                        tpm_remaining,
                        ..
                    }) => {
                        let error_msg = format!(
                            "Rate limit exceeded. RPM remaining: {}, TPM remaining: {}",
                            rpm_remaining, tpm_remaining
                        );
                        Err(create_rate_limit_error(error_msg))
                    }
                    Err(e) => Err(create_rate_limit_error(format!(
                        "Rate limit check failed: {}",
                        e
                    ))),
                }
            } else {
                Err(create_rate_limit_error("User ID not found".to_string()))
            }
        }
        Ok(result) => Err(create_rate_limit_error(
            result
                .error
                .unwrap_or_else(|| "Invalid API key".to_string()),
        )),
        Err(e) => Err(create_rate_limit_error(format!(
            "API key validation failed: {}",
            e
        ))),
    }
}

pub async fn rate_limit_with_tokens_middleware(
    State(state): State<Arc<ServiceContext>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let headers = request.headers();

    let api_key = match get_api_key(headers) {
        Some(key) => key,
        None => {
            return Err(create_rate_limit_error("Missing API key".to_string()));
        }
    };

    let validation_result = state.api_key_service.validate_api_key(api_key).await;

    match validation_result {
        Ok(result) if result.valid => {
            if let Some(user_id) = result.user_id {
                let rate_limit_result = state.rate_limit_service.check_quota(&user_id).await;

                match rate_limit_result {
                    Ok(RateLimitCheckResult {
                        allowed: true,
                        warning,
                        rpm_remaining,
                        tpm_remaining,
                    }) => {
                        let response = next.run(request).await;

                        let mut response = response;
                        if let Ok(header_value) = HeaderValue::from_str(&rpm_remaining.to_string())
                        {
                            response
                                .headers_mut()
                                .insert("X-RateLimit-RPM-Remaining", header_value);
                        }
                        if let Ok(header_value) = HeaderValue::from_str(&tpm_remaining.to_string())
                        {
                            response
                                .headers_mut()
                                .insert("X-RateLimit-TPM-Remaining", header_value);
                        }

                        if let Some(warning_msg) = warning
                            && let Ok(header_value) = HeaderValue::from_str(&warning_msg)
                        {
                            response
                                .headers_mut()
                                .insert("X-RateLimit-Warning", header_value);
                        }

                        Ok(response)
                    }
                    Ok(RateLimitCheckResult {
                        allowed: false,
                        rpm_remaining,
                        tpm_remaining,
                        ..
                    }) => {
                        let error_msg = format!(
                            "Rate limit exceeded. RPM remaining: {}, TPM remaining: {}",
                            rpm_remaining, tpm_remaining
                        );
                        Err(create_rate_limit_error(error_msg))
                    }
                    Err(e) => Err(create_rate_limit_error(format!(
                        "Rate limit check failed: {}",
                        e
                    ))),
                }
            } else {
                Err(create_rate_limit_error("User ID not found".to_string()))
            }
        }
        Ok(result) => Err(create_rate_limit_error(
            result
                .error
                .unwrap_or_else(|| "Invalid API key".to_string()),
        )),
        Err(e) => Err(create_rate_limit_error(format!(
            "API key validation failed: {}",
            e
        ))),
    }
}

pub async fn rate_limit_with_model_middleware(
    State(state): State<Arc<ServiceContext>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let headers = request.headers();

    let api_key = match get_api_key(headers) {
        Some(key) => key,
        None => {
            return Err(create_rate_limit_error("Missing API key".to_string()));
        }
    };

    let validation_result = state.api_key_service.validate_api_key(api_key).await;

    match validation_result {
        Ok(result) if result.valid => {
            if let Some(user_id) = result.user_id {
                let model_key = extract_model_key(&request);

                let rate_limit_result = if let Some(model) = model_key {
                    check_rate_limit_with_model(&state, &user_id, &model).await
                } else {
                    state
                        .rate_limit_service
                        .check_quota(&user_id)
                        .await
                        .map_err(crate::error::Error::Application)
                };

                match rate_limit_result {
                    Ok(RateLimitCheckResult {
                        allowed: true,
                        warning,
                        rpm_remaining,
                        tpm_remaining,
                    }) => {
                        let response = next.run(request).await;

                        let mut response = response;
                        if let Ok(header_value) = HeaderValue::from_str(&rpm_remaining.to_string())
                        {
                            response
                                .headers_mut()
                                .insert("X-RateLimit-RPM-Remaining", header_value);
                        }
                        if let Ok(header_value) = HeaderValue::from_str(&tpm_remaining.to_string())
                        {
                            response
                                .headers_mut()
                                .insert("X-RateLimit-TPM-Remaining", header_value);
                        }

                        if let Some(warning_msg) = warning
                            && let Ok(header_value) = HeaderValue::from_str(&warning_msg)
                        {
                            response
                                .headers_mut()
                                .insert("X-RateLimit-Warning", header_value);
                        }

                        Ok(response)
                    }
                    Ok(RateLimitCheckResult {
                        allowed: false,
                        rpm_remaining,
                        tpm_remaining,
                        ..
                    }) => {
                        let error_msg = format!(
                            "Rate limit exceeded. RPM remaining: {}, TPM remaining: {}",
                            rpm_remaining, tpm_remaining
                        );
                        Err(create_rate_limit_error(error_msg))
                    }
                    Err(e) => Err(create_rate_limit_error(format!(
                        "Rate limit check failed: {}",
                        e
                    ))),
                }
            } else {
                Err(create_rate_limit_error("User ID not found".to_string()))
            }
        }
        Ok(result) => Err(create_rate_limit_error(
            result
                .error
                .unwrap_or_else(|| "Invalid API key".to_string()),
        )),
        Err(e) => Err(create_rate_limit_error(format!(
            "API key validation failed: {}",
            e
        ))),
    }
}

async fn check_rate_limit_with_model(
    state: &Arc<ServiceContext>,
    user_id: &str,
    model_key: &str,
) -> crate::error::Result<RateLimitCheckResult> {
    let user_level_service = crate::service::ai_hub::UserLevelService::new();
    let user_level = user_level_service.get_user_level(user_id).await?;

    let model_limit_service = UserLevelModelRateLimitService::new();
    let model_limits = model_limit_service.get_limits_by_level(&user_level).await?;

    let model_limit = model_limits
        .iter()
        .find(|limit| limit.model_key == model_key && limit.enabled);

    if let Some(limit) = model_limit {
        let level_config = state
            .rate_limit_service
            .get_user_level_config(user_id)
            .await?;

        let rpm_limit = level_config.rpm_limit.min(limit.rpm_limit);
        let tpm_limit = level_config.tpm_limit.min(limit.tpm_limit);

        state
            .rate_limit_service
            .check_quota_with_custom_limits(user_id, rpm_limit, tpm_limit)
            .await
            .map_err(crate::error::Error::Application)
    } else {
        state
            .rate_limit_service
            .check_quota(user_id)
            .await
            .map_err(crate::error::Error::Application)
    }
}

fn extract_model_key(request: &Request) -> Option<String> {
    let uri = request.uri().path();
    let method = request.method();

    if method == axum::http::Method::POST && uri.contains("/chat/completions") {
        // 尝试从请求头获取模型
        if let Some(model_header) = request.headers().get("X-Model")
            && let Ok(model_str) = model_header.to_str()
        {
            return Some(model_str.to_string());
        }

        // 尝试从查询参数获取模型
        if let Some(query) = request.uri().query() {
            for param in query.split('&') {
                let mut parts = param.split('=');
                if let Some(key) = parts.next()
                    && key == "model"
                    && let Some(value) = parts.next()
                {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

fn get_api_key(h: &HeaderMap) -> Option<&str> {
    h.get(TOKEN_KEY)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| h.get(API_KEY_HEADER).and_then(|v| v.to_str().ok()))
}

fn create_rate_limit_error(message: String) -> Response {
    let error_response = Json(serde_json::json!({
        "code": "429",
        "msg": message,
        "data": null
    }));
    let mut response = error_response.into_response();
    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    response
}
