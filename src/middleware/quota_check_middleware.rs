use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::service::ai_hub::RateLimitCheckResult;

const API_KEY_HEADER: &str = "X-API-Key";

pub async fn quota_check_middleware(
    State(state): State<Arc<ServiceContext>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let headers = request.headers();

    let api_key = match get_api_key(headers) {
        Some(key) => key,
        None => {
            return Err(create_quota_error("Missing API key".to_string()));
        }
    };

    let validation_result = state.api_key_service.validate_api_key(api_key).await;

    match validation_result {
        Ok(result) if result.valid => {
            if let Some(user_id) = result.user_id {
                let rate_limit_result = state.rate_limit_service.check_quota(&user_id).await;

                match rate_limit_result {
                    Ok(RateLimitCheckResult { allowed: true, .. }) => {
                        let balance = state.balance_service.get_balance(&user_id).await;

                        match balance {
                            Ok(balance_vo) if balance_vo.balance < 0.0 => {
                                let error_msg = format!(
                                    "Insufficient balance. Current balance: {:.2}",
                                    balance_vo.balance
                                );
                                return Err(create_balance_error(error_msg));
                            }
                            Ok(_) => {
                                let mut request = request;
                                let user_id_header = axum::http::HeaderValue::from_str(&user_id)
                                    .map_err(|e| {
                                        create_quota_error(format!("Invalid user ID header: {}", e))
                                    })?;
                                request.headers_mut().insert("x-user-id", user_id_header);
                                let response = next.run(request).await;
                                Ok(response)
                            }
                            Err(e) => Err(create_quota_error(format!(
                                "Failed to check balance: {}",
                                e
                            ))),
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
                        Err(create_quota_error(error_msg))
                    }
                    Err(e) => Err(create_quota_error(format!("Quota check failed: {}", e))),
                }
            } else {
                Err(create_quota_error("User ID not found".to_string()))
            }
        }
        Ok(result) => Err(create_quota_error(
            result
                .error
                .unwrap_or_else(|| "Invalid API key".to_string()),
        )),
        Err(e) => Err(create_quota_error(format!(
            "API key validation failed: {}",
            e
        ))),
    }
}

pub async fn quota_check_with_tokens_middleware(
    State(state): State<Arc<ServiceContext>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let headers = request.headers();

    let api_key = match get_api_key(headers) {
        Some(key) => key,
        None => {
            return Err(create_quota_error("Missing API key".to_string()));
        }
    };

    let validation_result = state.api_key_service.validate_api_key(api_key).await;

    match validation_result {
        Ok(result) if result.valid => {
            if let Some(user_id) = result.user_id {
                let rate_limit_result = state.rate_limit_service.check_quota(&user_id).await;

                match rate_limit_result {
                    Ok(RateLimitCheckResult { allowed: true, .. }) => {
                        let balance = state.balance_service.get_balance(&user_id).await;

                        match balance {
                            Ok(balance_vo) if balance_vo.balance < 0.0 => {
                                let error_msg = format!(
                                    "Insufficient balance. Current balance: {:.2}",
                                    balance_vo.balance
                                );
                                return Err(create_balance_error(error_msg));
                            }
                            Ok(_) => {
                                let mut request = request;
                                let user_id_header = axum::http::HeaderValue::from_str(&user_id)
                                    .map_err(|e| {
                                        create_quota_error(format!("Invalid user ID header: {}", e))
                                    })?;
                                request.headers_mut().insert("x-user-id", user_id_header);
                                let response = next.run(request).await;
                                Ok(response)
                            }
                            Err(e) => Err(create_quota_error(format!(
                                "Failed to check balance: {}",
                                e
                            ))),
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
                        Err(create_quota_error(error_msg))
                    }
                    Err(e) => Err(create_quota_error(format!("Quota check failed: {}", e))),
                }
            } else {
                Err(create_quota_error("User ID not found".to_string()))
            }
        }
        Ok(result) => Err(create_quota_error(
            result
                .error
                .unwrap_or_else(|| "Invalid API key".to_string()),
        )),
        Err(e) => Err(create_quota_error(format!(
            "API key validation failed: {}",
            e
        ))),
    }
}

fn get_api_key(h: &HeaderMap) -> Option<&str> {
    h.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| h.get(API_KEY_HEADER).and_then(|v| v.to_str().ok()))
}

fn create_quota_error(message: String) -> Response {
    let error_response = Json(serde_json::json!({
        "code": "429",
        "msg": message,
        "data": null
    }));
    let mut response = error_response.into_response();
    *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
    response
}

fn create_balance_error(message: String) -> Response {
    let error_response = Json(serde_json::json!({
        "code": "402",
        "msg": message,
        "data": null
    }));
    let mut response = error_response.into_response();
    *response.status_mut() = StatusCode::PAYMENT_REQUIRED;
    response
}
