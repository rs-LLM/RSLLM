//! AI Hub 嵌入控制器模块
//!
//! 提供OpenAI兼容的嵌入生成API接口

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use std::sync::Arc;
use ulid::Ulid;

// 导入相关类型
use crate::context::ServiceContext;
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::dto::validation::Validator;
use crate::domain::table::ai_hub::model_base::ModelBase;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::vo::embeddings::{Embedding, Embeddings, EmbeddingsResponse};
use crate::domain::vo::response::ApiResponse;
use crate::domain::vo::usage::EmbeddingUsage;
use crate::service::ai_hub::rate_limit_service::RateLimitCheckResult;
use crate::service::{Content, TokenCounter};

/// 检测错误是否为429 Too Many Requests
fn is_rate_limit_error(error: &str) -> bool {
    error.contains("429") || 
    error.contains("Too Many Requests") ||
    error.contains("1302") ||
    error.contains("并发数过高")
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
        (status = 200, description = "嵌入生成成功", body = ApiResponse<EmbeddingsResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<EmbeddingsResponse>),
        (status = 401, description = "未授权", body = ApiResponse<EmbeddingsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<EmbeddingsResponse>)
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
    Json(req): Json<EmbeddingsRequest>,
) -> impl IntoResponse {
    let request_id = Ulid::new().to_string();
    let start_time = std::time::Instant::now();

    log::info!("[AI Hub] Embeddings request: {}", request_id);

    // 1. 用户认证
    let (user_id, api_key) = match authenticate_user(&headers, &state).await {
        Ok((id, key)) => (id, key),
        Err(e) => return Json(ApiResponse::error("401", &e.to_string())),
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);

    // 2. 输入验证
    match Validator::validate_embeddings_request(&req.model, &req.input) {
        Ok(_) => log::info!("[AI Hub] Input validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Input validation failed: {}", e);
            return Json(ApiResponse::error("400", &format!("输入验证失败: {}", e)));
        }
    }

    // 3. Token计算
    let (input_tokens, input_text_count) = match calculate_tokens(&req, &state.model_router).await {
        Ok(result) => result,
        Err(e) => return Json(ApiResponse::error("500", &e.to_string())),
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
        .check_quota_with_tokens(&user_id, input_tokens_i32)
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
            return Json(ApiResponse::error("429", &error_msg));
        }
        Err(e) => {
            log::error!("[AI Hub] Rate limit check failed: {}", e);
            return Json(ApiResponse::error(
                "500",
                &format!("Rate limit check failed: {}", e),
            ));
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
        Err(e) => return Json(ApiResponse::error("400", &e.to_string())),
    };

    log::info!(
        "[AI Hub] Pre-consumption check passed: cost={:.2}",
        fee.total_cost
    );

    // 5. 调用AI服务
    let response = match call_provider(&state, &req, &user_id).await {
        Ok(resp) => resp,
        Err(e) => {
            // 调用失败，回滚预消费
            log::error!(
                "[AI Hub] Provider call failed, rolling back pre-consumption: {}",
                e
            );
            if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                log::error!(
                    "[AI Hub] Failed to rollback pre-consumption: {}",
                    rollback_err
                );
            }
            return Json(ApiResponse::error(
                "500",
                &format!("AI service call failed: {}", e),
            ));
        }
    };

    // 6. 实际扣费和记录用量
    let duration_ms = start_time.elapsed().as_millis() as i64;
    let usage_log_id = match billing_service
        .deduct_quota_and_log(
            &fee,
            &api_key,
            duration_ms,
            "success",
            Some(serde_json::json!({
                "model": req.model,
                "input_count": input_text_count,
            })),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return Json(ApiResponse::error(
                "500",
                &format!("Failed to deduct quota and log: {}", e),
            ));
        }
    };

    log::info!("[AI Hub] Usage logged: {}", usage_log_id);

    // 7. 返回响应
    Json(ApiResponse::success(response))
}

/// 用户认证
async fn authenticate_user(
    headers: &HeaderMap,
    state: &Arc<ServiceContext>,
) -> std::result::Result<(String, String), String> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| "Missing or invalid authorization header".to_string())?;

    let validation_result = state
        .api_key_service
        .validate_api_key(token)
        .await
        .map_err(|e| format!("API key validation failed: {}", e))?;

    if !validation_result.valid {
        return Err(validation_result
            .error
            .unwrap_or_else(|| "Invalid API key".to_string()));
    }

    let user_id = validation_result
        .user_id
        .ok_or_else(|| "User ID not found".to_string())?;

    Ok((user_id, token.to_string()))
}

/// 计算token数量
async fn calculate_tokens(
    req: &EmbeddingsRequest,
    model_router: &crate::routers::model_router::ModelRouter,
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
    model_router: &crate::routers::model_router::ModelRouter,
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

    let model_id = model_base.id.ok_or_else(|| "Model ID not found".to_string())?;
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

        match provider
            .embeddings(provider_req, &serde_json::json!({}))
            .await
        {
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
                log::error!("[AI Hub] Mapping {}/{} failed: {}", index + 1, mappings.len(), last_error);

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
) -> std::result::Result<EmbeddingsResponse, String> {
    call_provider_with_fallback(state, req, _user_id).await
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
