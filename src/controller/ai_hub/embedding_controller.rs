//! AI Hub 嵌入控制器模块
//!
//! 提供OpenAI兼容的嵌入生成API接口

use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use ulid::Ulid;

// 导入相关类型
use crate::context::ServiceContext;
use crate::controller::ai_hub;
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::dto::validation::Validator;
use crate::domain::table::ai_hub::model_base::ModelBase;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::vo::ai_hub::responses::OpenAIErrorResponse;
use crate::domain::vo::embeddings::{Embedding, Embeddings, EmbeddingsResponse};
use crate::domain::vo::usage::EmbeddingUsage;
use crate::service::ai_hub::rate_limit_service::RateLimitCheckResult;
use crate::service::ai_hub::{AiRequestContext, resolve_request_identity};
use crate::service::{Content, TokenCounter};

/// 检测错误是否为429 Too Many Requests
fn is_rate_limit_error(error: &str) -> bool {
    error.contains("429")
        || error.contains("Too Many Requests")
        || error.contains("1302")
        || error.contains("并发数过高")
}

fn openai_error_response(status: StatusCode, message: impl Into<String>) -> Response {
    let mut response = Json(OpenAIErrorResponse::new(status.as_u16(), message)).into_response();
    *response.status_mut() = status;
    response
}

/// 获取模型的所有可用映射（按priority降序）
async fn get_all_mappings(
    model_id: &str,
    provider_id: &str,
) -> std::result::Result<Vec<ModelProviderMapping>, String> {
    let rb = crate::pool!();
    ModelProviderMapping::select_by_model_and_provider_all(rb, model_id, provider_id)
        .await
        .map_err(|e| format!("Failed to query mappings: {}", e))
}

/// 嵌入生成接口
///
/// 提供OpenAI兼容的嵌入生成功能
#[utoipa::path(
    post,
    path = "/api/v1/embeddings",
    request_body = EmbeddingsRequest,
    responses(
        (status = 200, description = "嵌入生成成功", body = EmbeddingsResponse),
        (status = 400, description = "参数错误", body = OpenAIErrorResponse),
        (status = 401, description = "未授权", body = OpenAIErrorResponse),
        (status = 429, description = "请求过多", body = OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = OpenAIErrorResponse)
    ),
    tag = "embeddings",
    security(
        ("api_key" = [])
    )
)]
#[axum::debug_handler]
pub async fn embeddings(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    Json(req): Json<EmbeddingsRequest>,
) -> Response {
    let start_time = std::time::Instant::now();

    let usage_log_id = Ulid::new().to_string();
    log::info!("[AI Hub] Embeddings request: {}", usage_log_id);

    // 兼容：保留 request_id 变量供历史日志/调试使用，但 trace_key 统一使用 usage_log_id。
    let request_id = usage_log_id.clone();
    let _ = request_id;

    // 1. 用户认证
    let (user_id, api_key) = match authenticate_user(
        &headers,
        &state,
        request_context.as_ref().map(|context| &context.0),
    )
    .await
    {
        Ok((id, key)) => (id, key),
        Err(e) => return openai_error_response(StatusCode::UNAUTHORIZED, e),
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);

    // 2. 输入验证
    match Validator::validate_embeddings_request(&req.model, &req.input) {
        Ok(_) => log::info!("[AI Hub] Input validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Input validation failed: {}", e);
            return openai_error_response(StatusCode::BAD_REQUEST, "输入参数不合法");
        }
    }

    // 3. Token计算
    let (input_tokens, input_text_count) = match calculate_tokens(&req, &state.model_router).await {
        Ok(result) => result,
        Err(e) => {
            log::error!("[AI Hub] Token calculation failed: {}", e);
            return openai_error_response(StatusCode::INTERNAL_SERVER_ERROR, "Token 计算失败");
        }
    };
    log::info!(
        "[AI Hub] Token calculation: input={}, model={}",
        input_tokens,
        req.model
    );

    // 3.5. 检查速率限制（使用实际的token数量）
    let input_tokens_i32 = input_tokens as i32;
    let rate_limit_result = state
        .rate_limit_service
        .precheck_request_tokens(&user_id, input_tokens_i32)
        .await;
    match rate_limit_result {
        Ok(RateLimitCheckResult {
            allowed: true,
            rpm_remaining,
            tpm_remaining,
            ..
        }) => {
            log::info!(
                "[AI Hub] Rate limit check passed: RPM remaining={}, TPM remaining={}",
                rpm_remaining,
                tpm_remaining
            );
        }
        Ok(RateLimitCheckResult {
            allowed: false,
            rpm_remaining,
            tpm_remaining,
            warning,
        }) => {
            let error_msg = if let Some(w) = warning {
                format!(
                    "Rate limit exceeded: {}. RPM remaining: {}, TPM remaining: {}",
                    w, rpm_remaining, tpm_remaining
                )
            } else {
                format!(
                    "Rate limit exceeded. RPM remaining: {}, TPM remaining: {}",
                    rpm_remaining, tpm_remaining
                )
            };
            log::warn!("[AI Hub] {}", error_msg);
            return openai_error_response(StatusCode::TOO_MANY_REQUESTS, error_msg);
        }
        Err(e) => {
            log::error!("[AI Hub] Rate limit check failed: {}", e);
            return openai_error_response(StatusCode::INTERNAL_SERVER_ERROR, "限流服务异常");
        }
    }

    // 4. 预消费和配额检查
    let billing_service = &state.billing_service;
    let (base_input_price, base_output_price) = get_pricing(&req.model, &state.model_router).await;

    // 嵌入通常没有输出token，但为了统一处理，设置为0
    let fee = match billing_service
        .calculate_and_check(&crate::service::ai_hub::CalculateAndCheckParams {
            user_id: &user_id,
            model_id: &req.model,
            api_key: &api_key,
            input_tokens,
            output_tokens: 0,
            base_input_price,
            base_output_price,
            request_type: "embeddings",
        })
        .await
    {
        Ok(fee) => fee,
        Err(e) => {
            log::warn!("[AI Hub] Pre-consumption check failed: {}", e);

            let (status, message) = match &e {
                crate::error::ApplicationError::QuotaExceeded { message, .. }
                | crate::error::ApplicationError::BalanceExceeded { message, .. }
                | crate::error::ApplicationError::BillingError { message, .. }
                | crate::error::ApplicationError::PriceRuleError { message, .. }
                | crate::error::ApplicationError::BusinessError { message, .. }
                | crate::error::ApplicationError::ValidationError { message, .. }
                | crate::error::ApplicationError::NotFound { message, .. } => {
                    (StatusCode::BAD_REQUEST, message.clone())
                }
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "计费服务异常".to_string(),
                ),
            };

            return openai_error_response(status, message);
        }
    };

    log::info!(
        "[AI Hub] Pre-consumption check passed: cost={:.2}",
        fee.total_cost
    );

    // 5. 调用AI服务
    let response = match call_provider(&state, &req, &user_id, &usage_log_id).await {
        Ok(resp) => resp,
        Err(e) => {
            log::error!("[AI Hub] Provider call failed: {}", e);
            return openai_error_response(StatusCode::INTERNAL_SERVER_ERROR, "AI 服务调用失败");
        }
    };

    // 6. 实际扣费和记录用量
    let _duration_ms = start_time.elapsed().as_millis() as i64;
    let (ip_address, user_agent) = ai_hub::extract_client_meta(&headers);
    let upstream =
        crate::service::ai_hub::provider::upstream_trace::get_upstream_oauth(&usage_log_id);

    let extra = {
        let mut map = serde_json::Map::new();
        map.insert(
            "model".to_string(),
            serde_json::Value::String(req.model.clone()),
        );
        map.insert(
            "input_count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input_text_count)),
        );
        if let Some(upstream) = &upstream {
            map.insert(
                "upstream_oauth".to_string(),
                serde_json::json!({
                    "provider_id": upstream.provider_id,
                    "provider_type": upstream.provider_type,
                    "account_key": upstream.account_key,
                    "account_id": upstream.account_id,
                    "email": upstream.email,
                }),
            );
        }
        Some(serde_json::Value::Object(map))
    };

    let meta =
        crate::service::ai_hub::UsageLogMeta::embeddings(ip_address.clone(), user_agent.clone());

    let logged_usage_log_id = match billing_service
        .deduct_quota_and_log(&fee, &meta, extra, upstream.clone())
        .await
    {
        Ok(id) => id,
        Err(_e) => {
            return openai_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "扣费失败，请稍后重试",
            );
        }
    };

    log::info!("[AI Hub] Usage logged: {}", logged_usage_log_id);

    if let Some(upstream) = upstream {
        crate::service::ai_hub::provider::upstream_trace::remove_upstream_oauth(&usage_log_id);
        crate::service::ai_hub::provider::oauth::record_provider_token_usage(
            &upstream.provider_id,
            &upstream.provider_type,
            &upstream.account_key,
            fee.input_tokens,
            0,
        );
    }

    // 7. 返回响应
    Json(response).into_response()
}

/// 用户认证
async fn authenticate_user(
    headers: &HeaderMap,
    state: &Arc<ServiceContext>,
    request_context: Option<&AiRequestContext>,
) -> std::result::Result<(String, String), String> {
    let identity = resolve_request_identity(headers, state, request_context).await?;
    Ok((identity.user_id, identity.api_key))
}

/// 计算token数量
async fn calculate_tokens(
    req: &EmbeddingsRequest,
    model_router: &crate::router::model_router::ModelRouter,
) -> std::result::Result<(i64, usize), String> {
    let mut total_tokens = 0;

    let model_def = match model_router.route_to_model(&req.model).await {
        Ok(def) => def,
        Err(e) => {
            log::warn!(
                "[AI Hub] Failed to get model definition for {}: {}",
                req.model,
                e
            );
            return Err(format!("Model not found: {}", req.model));
        }
    };

    let input_count = match &req.input {
        crate::domain::dto::embeddings::EmbeddingsInput::Single(text) => {
            let content = Content::Text(text.clone());
            let meta =
                TokenCounter::count_content_tokens(&content, &req.model, false, Some(&model_def))
                    .map_err(|e| e.to_string())?;
            total_tokens += meta.input_tokens;
            1
        }
        crate::domain::dto::embeddings::EmbeddingsInput::Multiple(texts) => {
            for text in texts {
                let content = Content::Text(text.clone());
                let meta = TokenCounter::count_content_tokens(
                    &content,
                    &req.model,
                    false,
                    Some(&model_def),
                )
                .map_err(|e| e.to_string())?;
                total_tokens += meta.input_tokens;
            }
            texts.len()
        }
        crate::domain::dto::embeddings::EmbeddingsInput::SingleTokenIds(token_ids) => {
            total_tokens += token_ids.len() as i64;
            1
        }
        crate::domain::dto::embeddings::EmbeddingsInput::MultipleTokenIds(token_ids_list) => {
            for token_ids in token_ids_list {
                total_tokens += token_ids.len() as i64;
            }
            token_ids_list.len()
        }
    };

    Ok((total_tokens, input_count))
}

/// 获取模型定价
async fn get_pricing(
    model: &str,
    model_router: &crate::router::model_router::ModelRouter,
) -> (f64, f64) {
    match model_router.route_to_model(model).await {
        Ok(model_info) => {
            let input_price = model_info.model_base.input_price;
            let output_price = model_info.model_base.output_price;
            (input_price, output_price)
        }
        Err(e) => {
            log::warn!(
                "[AI Hub] Failed to get model definition for {}: {}, using default pricing",
                model,
                e
            );
            // 默认定价
            (0.0001, 0.0)
        }
    }
}

/// 调用AI Provider获取真实响应（支持降级）
async fn call_provider_with_fallback(
    state: &Arc<ServiceContext>,
    req: &EmbeddingsRequest,
    _user_id: &str,
    trace_key: &str,
) -> std::result::Result<EmbeddingsResponse, String> {
    let model_router = &state.model_router;

    let (provider_config, model_info) = model_router
        .route(&req.model, true)
        .await
        .map_err(|e| format!("Failed to route model: {}", e))?;

    let rb = crate::pool!();

    let model_base = ModelBase::select_by_model_code(rb, &model_info.model_base.model_code)
        .await
        .map_err(|e| format!("Failed to query model_base: {}", e))?
        .ok_or_else(|| {
            format!(
                "Model '{}' not found in model_base",
                model_info.model_base.model_code
            )
        })?;

    let model_id = model_base
        .id
        .ok_or_else(|| "Model ID not found".to_string())?;
    let provider_id = provider_config.id.as_str();

    let mappings = get_all_mappings(&model_id, provider_id).await?;

    if mappings.is_empty() {
        return Err("No mappings found for this model".to_string());
    }

    let mut last_error = String::new();

    for (index, mapping) in mappings.iter().enumerate() {
        log::info!(
            "[AI Hub] Trying mapping {}/{}: provider_model_name={}, priority={:?}",
            index + 1,
            mappings.len(),
            mapping.provider_model_name,
            mapping.priority
        );

        let provider_registry = state.provider_registry.read().await;

        let provider = match provider_registry.get_provider(&provider_config.provider_code) {
            Some(p) => p,
            None => {
                last_error = format!(
                    "Provider not found for provider_code: {}",
                    provider_config.provider_code
                );
                log::warn!("[AI Hub] {}", last_error);
                continue;
            }
        };

        let provider_req = crate::domain::dto::embeddings::EmbeddingsRequest {
            model: mapping.provider_model_name.clone(),
            input: req.input.clone(),
            encoding_format: req.encoding_format.clone(),
            user: req.user.clone(),
        };

        let model_config = serde_json::json!({
            "rsllm": {
                "trace_key": trace_key,
                "provider_id": provider_id,
                "provider_type": provider_config.provider_type.to_string(),
            }
        });

        match provider.embeddings(provider_req, &model_config).await {
            Ok(response) => {
                log::info!(
                    "[AI Hub] Successfully called provider with mapping {}/{}",
                    index + 1,
                    mappings.len()
                );
                return convert_from_provider_response(response);
            }
            Err(e) => {
                last_error = format!("Provider error: {}", e);
                log::error!(
                    "[AI Hub] Mapping {}/{} failed: {}",
                    index + 1,
                    mappings.len(),
                    last_error
                );

                if is_rate_limit_error(&last_error) {
                    log::info!("[AI Hub] Rate limit error detected, trying next mapping...");
                    continue;
                } else {
                    return Err(last_error);
                }
            }
        }
    }

    Err(format!(
        "All {} mappings failed. Last error: {}",
        mappings.len(),
        last_error
    ))
}

/// 调用AI Provider获取真实响应
async fn call_provider(
    state: &Arc<ServiceContext>,
    req: &EmbeddingsRequest,
    _user_id: &str,
    trace_key: &str,
) -> std::result::Result<EmbeddingsResponse, String> {
    call_provider_with_fallback(state, req, _user_id, trace_key).await
}

/// 转换从Provider响应类型
fn convert_from_provider_response(
    response: crate::domain::vo::embeddings::EmbeddingsResponse,
) -> std::result::Result<EmbeddingsResponse, String> {
    Ok(EmbeddingsResponse {
        object: response.object,
        data: response
            .data
            .into_iter()
            .map(|item| Embeddings {
                object: item.object,
                embedding: match item.embedding {
                    crate::domain::vo::embeddings::Embedding::Float(vec) => Embedding::Float(vec),
                    crate::domain::vo::embeddings::Embedding::String(s) => Embedding::String(s),
                    crate::domain::vo::embeddings::Embedding::Json(json) => Embedding::Json(json),
                },
                index: item.index,
            })
            .collect(),
        model: response.model,
        usage: EmbeddingUsage {
            prompt_tokens: response.usage.prompt_tokens,
            total_tokens: response.usage.total_tokens,
        },
    })
}
