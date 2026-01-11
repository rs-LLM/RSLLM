//! AI Hub 聊天控制器模块
//!
//! 提供OpenAI兼容的聊天补全API接口

use axum::{
    Json,
    extract::State,
    http::HeaderMap,
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
};
use futures::StreamExt;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use ulid::Ulid;

// 导入相关类型
use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::streaming::{ChatCompletionChunk, ChatCompletionChunkChoice};
use crate::domain::dto::ai_hub::validation::ChatCompletionRequestParams;
use crate::domain::dto::chat::ChatCompletionRequest;
use crate::domain::dto::content::{
    ChatCompletionMessage, ChatMessageContent, ChatMessageContentPart,
};
use crate::domain::dto::validation::Validator;
use crate::domain::table::ai_hub::model_base::ModelBase;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::vo::chat::ChatCompletion;
use crate::domain::vo::chat::ChatCompletionChoice;
use crate::domain::vo::response::ApiResponse;
use crate::domain::vo::usage::Usage;
use crate::service::BillingService;
use crate::service::CalculatedFee;
use crate::service::PriceRuleService;
use crate::service::ai_hub::rate_limit_service::RateLimitCheckResult;
use crate::service::{Content, TokenCountMeta, TokenCounter};

/// 创建SSE错误响应
fn create_sse_error_response(error_message: String) -> Response {
    let error_response = ApiResponse::<()>::error("500", &error_message);
    let error_event = Event::default()
        .json_data(&error_response)
        .unwrap_or_else(|_| Event::default().data("Unknown error"));
    let stream = futures::stream::once(async move { Ok::<Event, Infallible>(error_event) });
    Sse::new(Box::pin(stream)).into_response()
}

/// 处理非流式响应
async fn handle_non_streaming_response(
    req: ChatCompletionRequest,
    user_id: String,
    api_key: String,
    fee: CalculatedFee,
    _request_id: String,
    usage_log_id: String,
    state: Arc<ServiceContext>,
) -> Response {
    let billing_service = &state.billing_service;
    let rate_limit_service = state.rate_limit_service.clone();
    let model_router = state.model_router.clone();

    let start_time = std::time::Instant::now();

    // 调用AI服务
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
            return Json(ApiResponse::<()>::error(
                "500",
                &format!("AI service call failed: {}", e),
            ))
            .into_response();
        }
    };

    // 计算输出token数量（包括普通内容和推理内容）
    log::info!("[AI Hub] Calculating output tokens for non-streaming response");

    let output_tokens = {
        let mut total_output_tokens = 0;

        // 遍历所有choices，提取并计算token
        for choice in &response.choices {
            // 计算普通内容的 token
            if let Some(content) = &choice.message.content {
                let content_str = match content {
                    ChatMessageContent::String(s) => s.clone(),
                    ChatMessageContent::Array(parts) => parts
                        .iter()
                        .filter_map(|part| part.text.as_deref())
                        .collect::<Vec<&str>>()
                        .join(""),
                };

                if !content_str.is_empty() {
                    match model_router.route_to_model(&req.model).await {
                        Ok(model_def) => {
                            match TokenCounter::count_content_tokens(
                                &Content::Text(content_str),
                                &req.model,
                                true,
                                Some(&model_def),
                            ) {
                                Ok(meta) => {
                                    log::info!(
                                        "[AI Hub] Non-streaming content token count result: output_tokens={}, input_tokens={}, total_tokens={}",
                                        meta.output_tokens,
                                        meta.input_tokens,
                                        meta.total_tokens
                                    );
                                    total_output_tokens += meta.output_tokens;
                                }
                                Err(e) => {
                                    log::warn!(
                                        "[AI Hub] Failed to count non-streaming content tokens: {}",
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "[AI Hub] Failed to get model definition for non-streaming content: {}",
                                e
                            );
                        }
                    }
                }
            }

            // 计算推理内容的 token
            if let Some(reasoning_content) = &choice.message.reasoning_content {
                if !reasoning_content.is_empty() {
                    match model_router.route_to_model(&req.model).await {
                        Ok(model_def) => {
                            match TokenCounter::count_content_tokens(
                                &Content::Text(reasoning_content.clone()),
                                &req.model,
                                true,
                                Some(&model_def),
                            ) {
                                Ok(meta) => {
                                    log::info!(
                                        "[AI Hub] Non-streaming reasoning content token count result: output_tokens={}, input_tokens={}, total_tokens={}",
                                        meta.output_tokens,
                                        meta.input_tokens,
                                        meta.total_tokens
                                    );
                                    total_output_tokens += meta.output_tokens;
                                }
                                Err(e) => {
                                    log::warn!(
                                        "[AI Hub] Failed to count non-streaming reasoning content tokens: {}",
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "[AI Hub] Failed to get model definition for non-streaming reasoning content: {}",
                                e
                            );
                        }
                    }
                }
            }
        }

        log::info!(
            "[AI Hub] Total non-streaming output tokens: {} (content + reasoning)",
            total_output_tokens
        );
        total_output_tokens
    };

    // 消耗输出token到速率限制（每次请求都消耗 1 个 RPM 和 output_tokens 个 TPM）
    if let Err(e) = rate_limit_service
        .consume_tokens(&user_id, output_tokens as i32)
        .await
    {
        log::error!("[AI Hub] Failed to consume tokens: {}", e);
    } else {
        log::info!(
            "[AI Hub] Consumed {} output tokens for rate limit",
            output_tokens
        );
    }

    log::info!(
        "[AI Hub] AI service call successful, returning response (balance already pre-deducted, {} output tokens consumed)",
        output_tokens
    );

    let duration_ms = start_time.elapsed().as_millis() as i64;

    log::info!(
        "[AI Hub] Calling complete_usage_log: usage_log_id={}, user_id={}, model_id={}, input_tokens={}, output_tokens={}",
        usage_log_id,
        user_id,
        req.model,
        fee.input_tokens,
        output_tokens
    );

    // 创建完整的用量记录并扣减输出费用
    if let Err(e) = billing_service
        .complete_usage_log(
            &usage_log_id,
            &user_id,
            &req.model,
            &api_key,
            fee.input_tokens,
            output_tokens,
            fee.input_calculation.final_price,
            fee.output_calculation.final_price,
            duration_ms,
            None,
        )
        .await
    {
        log::error!("[AI Hub] Failed to complete usage log: {}", e);
    }

    // 返回响应（余额已在 chat_completions 中预扣减）
    Json(response).into_response()
}

/// 处理流式响应
async fn handle_streaming_response(
    req: ChatCompletionRequest,
    user_id: String,
    api_key: String,
    fee: CalculatedFee,
    request_id: String,
    usage_log_id: String,
    state: Arc<ServiceContext>,
) -> Response {
    let billing_service = state.billing_service.clone();
    let rate_limit_service = state.rate_limit_service.clone();
    let model_router = state.model_router.clone();

    // 创建SSE流
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

    tokio::spawn(async move {
        let start_time = std::time::Instant::now();

        // 发送初始块
        let init_chunk = ChatCompletionChunk {
            id: request_id.clone(),
            object: Some("chat.completion.chunk".to_string()),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            model: req.model.clone(),
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: Some(crate::domain::dto::ai_hub::streaming::ChatCompletionChunkDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    reasoning_content: None,
                    tool_calls: None,
                    extra_fields: serde_json::Value::default(),
                }),
                finish_reason: None,
                logprobs: None,
                extra_fields: serde_json::Value::default(),
            }],
            system_fingerprint: None,
            extra_fields: serde_json::Value::default(),
        };

        if let Ok(event) = Event::default().json_data(&init_chunk) {
            let _ = tx.send(Ok(event)).await;
        }

        // 调用AI服务获取流式响应
        let mut stream = match call_provider_stream(&state, &req, &user_id).await {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("[AI Hub] Provider call failed: {}", e);
                // 回滚预消费
                if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                    log::error!(
                        "[AI Hub] Failed to rollback pre-consumption: {}",
                        rollback_err
                    );
                }
                // 发送错误事件
                let error_event = Event::default().data(format!("Error: {}", e));
                let _ = tx.send(Ok(error_event)).await;
                return;
            }
        };

        // 累计输出内容
        let mut accumulated_content = String::new();
        let mut accumulated_reasoning_content = String::new();

        // 处理流式响应
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(mut chunk) => {
                    // 替换 model 字段为用户请求的原始模型标识符
                    chunk.model = req.model.clone();

                    // 累计输出内容和推理内容
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(delta) = &choice.delta {
                            // 累积推理内容
                            if let Some(reasoning) = &delta.reasoning_content {
                                accumulated_reasoning_content.push_str(reasoning);
                            }

                            // 累积普通内容
                            if let Some(content) = &delta.content {
                                accumulated_content.push_str(content);
                            }
                        }
                    }

                    // 转换为SSE事件
                    if let Ok(event) = Event::default().json_data(&chunk) {
                        let _ = tx.send(Ok(event)).await;
                    }
                }
                Err(e) => {
                    log::error!("[AI Hub] Stream error: {}", e);
                    // 发送错误事件
                    let error_event = Event::default().data(format!("Error: {}", e));
                    let _ = tx.send(Ok(error_event)).await;
                    break;
                }
            }
        }

        // 计算输出token数量（包括普通内容和推理内容）
        log::info!(
            "[AI Hub] Calculating output tokens: accumulated_content length={}, accumulated_reasoning_content length={}",
            accumulated_content.len(),
            accumulated_reasoning_content.len()
        );

        let output_tokens = {
            let mut total_output_tokens = 0;

            // 计算普通内容的 token
            if !accumulated_content.is_empty() {
                match model_router.route_to_model(&req.model).await {
                    Ok(model_def) => {
                        match TokenCounter::count_content_tokens(
                            &Content::Text(accumulated_content),
                            &req.model,
                            true,
                            Some(&model_def),
                        ) {
                            Ok(meta) => {
                                log::info!(
                                    "[AI Hub] Content token count result: output_tokens={}, input_tokens={}, total_tokens={}",
                                    meta.output_tokens,
                                    meta.input_tokens,
                                    meta.total_tokens
                                );
                                total_output_tokens += meta.output_tokens;
                            }
                            Err(e) => {
                                log::warn!("[AI Hub] Failed to count content tokens: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("[AI Hub] Failed to get model definition for content: {}", e);
                    }
                }
            } else {
                log::warn!("[AI Hub] accumulated_content is empty");
            }

            // 计算推理内容的 token
            if !accumulated_reasoning_content.is_empty() {
                match model_router.route_to_model(&req.model).await {
                    Ok(model_def) => {
                        match TokenCounter::count_content_tokens(
                            &Content::Text(accumulated_reasoning_content),
                            &req.model,
                            true,
                            Some(&model_def),
                        ) {
                            Ok(meta) => {
                                log::info!(
                                    "[AI Hub] Reasoning content token count result: output_tokens={}, input_tokens={}, total_tokens={}",
                                    meta.output_tokens,
                                    meta.input_tokens,
                                    meta.total_tokens
                                );
                                total_output_tokens += meta.output_tokens;
                            }
                            Err(e) => {
                                log::warn!(
                                    "[AI Hub] Failed to count reasoning content tokens: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "[AI Hub] Failed to get model definition for reasoning content: {}",
                            e
                        );
                    }
                }
            } else {
                log::warn!("[AI Hub] accumulated_reasoning_content is empty");
            }

            log::info!(
                "[AI Hub] Total output tokens: {} (content + reasoning)",
                total_output_tokens
            );
            total_output_tokens
        };

        // 消耗输出token到速率限制（每次请求都消耗 1 个 RPM 和 output_tokens 个 TPM）
        if let Err(e) = rate_limit_service
            .consume_tokens(&user_id, output_tokens as i32)
            .await
        {
            log::error!("[AI Hub] Failed to consume tokens: {}", e);
        } else {
            log::info!(
                "[AI Hub] Consumed {} output tokens for rate limit",
                output_tokens
            );
        }

        // 发送结束块
        let end_chunk = ChatCompletionChunk {
            id: request_id.clone(),
            object: Some("chat.completion.chunk".to_string()),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            model: req.model.clone(),
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: None,
                finish_reason: Some("stop".to_string()),
                logprobs: None,
                extra_fields: serde_json::Value::default(),
            }],
            system_fingerprint: None,
            extra_fields: serde_json::Value::default(),
        };

        if let Ok(event) = Event::default().json_data(&end_chunk) {
            let _ = tx.send(Ok(event)).await;
        }

        log::info!(
            "[AI Hub] Streaming response completed successfully (balance already pre-deducted, {} output tokens consumed)",
            output_tokens
        );

        // 创建完整的用量记录并扣减输出费用
        let duration_ms = start_time.elapsed().as_millis() as i64;

        log::info!(
            "[AI Hub] Calling complete_usage_log (streaming): usage_log_id={}, user_id={}, model_id={}, input_tokens={}, output_tokens={}",
            usage_log_id,
            user_id,
            req.model,
            fee.input_tokens,
            output_tokens
        );

        if let Err(e) = billing_service
            .complete_usage_log(
                &usage_log_id,
                &user_id,
                &req.model,
                &api_key,
                fee.input_tokens,
                output_tokens,
                fee.input_calculation.final_price,
                fee.output_calculation.final_price,
                duration_ms,
                None,
            )
            .await
        {
            log::error!("[AI Hub] Failed to complete usage log: {}", e);
        }
    });

    // 创建SSE流
    let stream = ReceiverStream::new(rx)
        .map(|result| result.unwrap_or_else(|_| Event::default().data("Stream error")))
        .map(Ok::<Event, Infallible>);

    Sse::new(Box::pin(stream)).into_response()
}

/// 聊天补全接口
///
/// 提供OpenAI兼容的聊天补全功能，通过 `stream` 参数控制响应模式：
/// - `stream=false`：返回完整的JSON响应（非流式）
/// - `stream=true`：返回SSE流式响应
#[utoipa::path(
    post,
    path = "/api/v1/chat/completions",
    request_body = ChatCompletionRequest,
    responses(
        (status = 200, description = "聊天补全成功", body = ChatCompletion),
        (status = 400, description = "参数错误", body = ApiResponse<ChatCompletion>),
        (status = 401, description = "未授权", body = ApiResponse<ChatCompletion>),
        (status = 500, description = "服务器错误", body = ApiResponse<ChatCompletion>)
    ),
    tag = "chat",
    security(
        ("api_key" = [])
    )
)]
#[axum::debug_handler]
pub async fn chat_completions(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    let request_id = Ulid::new().to_string();
    let start_time = std::time::Instant::now();

    log::info!("[AI Hub] Chat completion request: {}", request_id);

    // 1. 用户认证并获取API密钥
    let (user_id, api_key) = match authenticate_user(&headers, &state).await {
        Ok((id, key)) => (id, key),
        Err(e) => {
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(e);
            } else {
                return Json(ApiResponse::<()>::error("401", &e)).into_response();
            }
        }
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);

    // 2. 输入验证
    let params = ChatCompletionRequestParams {
        model: &req.model,
        messages: &req.messages,
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        top_p: req.top_p,
        frequency_penalty: req.frequency_penalty,
        presence_penalty: req.presence_penalty,
        n: req.n,
    };
    match Validator::validate_chat_completion_request(&params) {
        Ok(_) => log::info!("[AI Hub] Input validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Input validation failed: {}", e);
            let error_msg = format!("输入验证失败: {}", e);
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return Json(ApiResponse::<()>::error("400", &error_msg)).into_response();
            }
        }
    }

    // 3. Token计算
    let token_meta = match calculate_tokens(&req, &state.model_router).await {
        Ok(meta) => meta,
        Err(e) => {
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(e);
            } else {
                return Json(ApiResponse::<()>::error("500", &e)).into_response();
            }
        }
    };
    log::info!(
        "[AI Hub] Token calculation: input={}, model={}",
        token_meta.input_tokens,
        req.model
    );

    // 3.5. 检查速率限制（使用实际的token数量）
    let input_tokens_i32 = token_meta.input_tokens as i32;
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
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return Json(ApiResponse::<()>::error("429", &error_msg)).into_response();
            }
        }
        Err(e) => {
            log::error!("[AI Hub] Rate limit check failed: {}", e);
            let error_msg = format!("Rate limit check failed: {}", e);
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return Json(ApiResponse::<()>::error("500", &error_msg)).into_response();
            }
        }
    }

    // 4. 获取动态定价并预消费和配额检查
    let billing_service = &state.billing_service;
    let price_rule_service = &state.price_rule_service;

    // 预估输出token（基于max_tokens或默认值）
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;

    // 使用PriceRuleService获取动态定价
    let fee = match get_dynamic_pricing(
        &user_id,
        &req.model,
        token_meta.input_tokens,
        estimated_output_tokens,
        billing_service,
        price_rule_service,
        &state.model_router,
        &api_key,
    )
    .await
    {
        Ok(fee) => fee,
        Err(e) => {
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(e);
            } else {
                return Json(ApiResponse::<()>::error("500", &e)).into_response();
            }
        }
    };

    log::info!(
        "[AI Hub] Pre-consumption check passed: cost={:.2}",
        fee.total_cost
    );

    // 4.5. 预扣减余额
    let duration_ms = start_time.elapsed().as_millis() as i64;
    let _usage_log_id = match billing_service
        .pre_deduct_quota_and_log(
            &fee,
            duration_ms,
            "pending",
            Some(serde_json::json!({
                "model": req.model,
                "stream": req.stream.unwrap_or(false),
            })),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            let error_msg = format!("Failed to pre-deduct quota: {}", e);
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return Json(ApiResponse::<()>::error("500", &error_msg)).into_response();
            }
        }
    };

    log::info!("[AI Hub] Pre-deduct quota successful: {}", _usage_log_id);

    // 5. 根据 stream 参数选择响应模式
    if req.stream.unwrap_or(false) {
        // 流式响应
        handle_streaming_response(
            req,
            user_id,
            api_key,
            fee,
            request_id,
            _usage_log_id.clone(),
            state,
        )
        .await
    } else {
        // 非流式响应
        handle_non_streaming_response(req, user_id, api_key, fee, request_id, _usage_log_id, state)
            .await
    }
}

/// 用户认证（仅支持API key）
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
    req: &ChatCompletionRequest,
    model_router: &crate::routers::model_router::ModelRouter,
) -> std::result::Result<TokenCountMeta, String> {
    log::info!(
        "[AI Hub] calculate_tokens called: model={}, messages_count={}",
        req.model,
        req.messages.len()
    );

    let mut total_meta = TokenCountMeta::default();

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

    let is_stream = req.stream.unwrap_or(false);
    log::info!(
        "[AI Hub] Model definition retrieved: is_stream={}",
        is_stream
    );

    for (idx, message) in req.messages.iter().enumerate() {
        log::info!("[AI Hub] Processing message {}: role={}", idx, message.role);

        if let Some(content) = &message.content {
            let content_enum = match content {
                ChatMessageContent::String(s) => {
                    log::info!(
                        "[AI Hub] Message {} content type: String, length={}",
                        idx,
                        s.len()
                    );
                    Content::Text(s.clone())
                }
                ChatMessageContent::Array(parts) => {
                    log::info!(
                        "[AI Hub] Message {} content type: Array, parts_count={}",
                        idx,
                        parts.len()
                    );
                    let mut contents = Vec::new();
                    for (part_idx, part) in parts.iter().enumerate() {
                        if let Some(text) = &part.text {
                            log::info!(
                                "[AI Hub] Message {} part {}: text length={}",
                                idx,
                                part_idx,
                                text.len()
                            );
                            contents.push(Content::Text(text.clone()));
                        }
                    }
                    Content::Multi(contents)
                }
            };

            let meta = TokenCounter::count_content_tokens(
                &content_enum,
                &req.model,
                false,
                Some(&model_def),
            )
            .map_err(|e| e.to_string())?;

            log::info!(
                "[AI Hub] Message {} token count: input_tokens={}, output_tokens={}",
                idx,
                meta.input_tokens,
                meta.output_tokens
            );

            total_meta.input_tokens += meta.input_tokens;
            total_meta.output_tokens += meta.output_tokens;
        } else {
            log::warn!("[AI Hub] Message {} has no content", idx);
        }
    }

    log::info!(
        "[AI Hub] calculate_tokens completed: total_input_tokens={}, total_output_tokens={}",
        total_meta.input_tokens,
        total_meta.output_tokens
    );

    Ok(total_meta)
}

/// 获取动态定价
async fn get_dynamic_pricing(
    user_id: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    billing_service: &BillingService,
    _price_rule_service: &PriceRuleService,
    model_router: &crate::routers::model_router::ModelRouter,
    api_key: &str,
) -> std::result::Result<CalculatedFee, String> {
    log::info!(
        "[AI Hub] get_dynamic_pricing called: user_id={}, model={}, input_tokens={}, output_tokens={}",
        user_id,
        model,
        input_tokens,
        output_tokens
    );

    let (base_input_price, base_output_price, _provider_id) = match model_router
        .route_to_model(model)
        .await
    {
        Ok(model_info) => {
            log::info!(
                "[AI Hub] Retrieved model pricing from database: {} - input: {:.6}, output: {:.6}",
                model,
                model_info.model_base.input_price,
                model_info.model_base.output_price
            );
            (
                model_info.model_base.input_price,
                model_info.model_base.output_price,
                model_info.provider.id.clone().unwrap_or_default(),
            )
        }
        Err(e) => {
            log::warn!(
                "[AI Hub] Failed to get model definition from database: {}, using default pricing",
                e
            );
            (1.0, 1.0, "unknown".to_string())
        }
    };

    log::info!(
        "[AI Hub] Calling calculate_and_check: base_input_price={:.6}, base_output_price={:.6}",
        base_input_price,
        base_output_price
    );

    let fee = billing_service
        .calculate_and_check(&crate::service::ai_hub::CalculateAndCheckParams {
            user_id,
            model_id: model,
            api_key,
            input_tokens,
            output_tokens,
            base_input_price,
            base_output_price,
            request_type: "chat",
        })
        .await
        .map_err(|e| format!("Billing calculation failed: {}", e))?;

    log::info!(
        "[AI Hub] calculate_and_check result: input_cost={:.2}, output_cost={:.2}, total_cost={:.2}, final_input_price={:.6}, final_output_price={:.6}",
        fee.input_cost,
        fee.output_cost,
        fee.total_cost,
        fee.input_calculation.final_price,
        fee.output_calculation.final_price
    );

    Ok(fee)
}

/// 调用AI Provider获取真实流式响应
async fn call_provider_stream(
    state: &Arc<ServiceContext>,
    req: &ChatCompletionRequest,
    _user_id: &str,
) -> std::result::Result<
    futures::stream::BoxStream<
        'static,
        Result<crate::domain::dto::ai_hub::streaming::ChatCompletionChunk, reqwest::Error>,
    >,
    String,
> {
    // 1. 使用ModelRouter解析模型标识符并路由到对应的provider和model
    let model_router = &state.model_router;

    // 解析模型标识符格式: "provider_code/model_code"
    let (provider_config, model_info) = model_router
        .route(&req.model, true)
        .await
        .map_err(|e| format!("Failed to route model: {}", e))?;

    log::info!(
        "[AI Hub] Routed to provider: {}, model: {}",
        provider_config.provider_code,
        model_info.model_base.model_code
    );

    // 2. 查询供应商模型名称
    let provider_model_name = {
        let rb = crate::pool!();

        // 通过 model_code 查询 model_base 获取 model_id
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

        // 通过 model_id 和 provider_id 查询 model_provider_mapping 获取 provider_model_name
        let mapping =
            ModelProviderMapping::select_by_model_and_provider(rb, &model_id, provider_id)
                .await
                .map_err(|e| format!("Failed to query model_provider_mapping: {}", e))?
                .ok_or_else(|| {
                    format!(
                        "Model provider mapping not found for model_id: {}, provider_id: {}",
                        model_id, provider_id
                    )
                })?;

        mapping.provider_model_name
    };

    log::info!("[AI Hub] Provider model name: {}", provider_model_name);

    // 3. 从ProviderRegistry获取Provider实例
    let provider_registry = state.provider_registry.read().await;

    let provider = provider_registry
        .get_provider(&provider_config.provider_code)
        .ok_or_else(|| {
            format!(
                "Provider not found for provider_code: {}",
                provider_config.provider_code
            )
        })?;

    // 4. 转换请求类型，使用供应商模型名称
    let provider_req = convert_to_provider_request(req, &provider_model_name);

    // 5. 调用Provider的chat_completions方法
    let response = provider
        .chat_completions(provider_req, &serde_json::json!({}))
        .await
        .map_err(|e| format!("Provider error: {}", e))?;

    // 6. 返回流式响应
    match response {
        crate::domain::dto::chat::ChatCompletionResponse::Stream(stream) => Ok(stream),
        crate::domain::dto::chat::ChatCompletionResponse::NonStream(_) => {
            Err("Expected stream response but got non-stream response".to_string())
        }
    }
}

/// 调用AI Provider获取真实响应
async fn call_provider(
    state: &Arc<ServiceContext>,
    req: &ChatCompletionRequest,
    _user_id: &str,
) -> std::result::Result<ChatCompletion, String> {
    // 1. 使用ModelRouter解析模型标识符并路由到对应的provider和model
    let model_router = &state.model_router;

    // 解析模型标识符格式: "provider_code/model_code"
    let (provider_config, model_info) = model_router
        .route(&req.model, true)
        .await
        .map_err(|e| format!("Failed to route model: {}", e))?;

    log::info!(
        "[AI Hub] Routed to provider: {}, model: {}",
        provider_config.provider_code,
        model_info.model_base.model_code
    );

    // 2. 查询供应商模型名称
    let provider_model_name = {
        let rb = crate::pool!();

        // 通过 model_code 查询 model_base 获取 model_id
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

        // 通过 model_id 和 provider_id 查询 model_provider_mapping 获取 provider_model_name
        let mapping =
            ModelProviderMapping::select_by_model_and_provider(rb, &model_id, provider_id)
                .await
                .map_err(|e| format!("Failed to query model_provider_mapping: {}", e))?
                .ok_or_else(|| {
                    format!(
                        "Model provider mapping not found for model_id: {}, provider_id: {}",
                        model_id, provider_id
                    )
                })?;

        mapping.provider_model_name
    };

    log::info!("[AI Hub] Provider model name: {}", provider_model_name);

    // 3. 从ProviderRegistry获取Provider实例
    let provider_registry = state.provider_registry.read().await;

    let provider = provider_registry
        .get_provider(&provider_config.provider_code)
        .ok_or_else(|| {
            format!(
                "Provider not found for provider_code: {}",
                provider_config.provider_code
            )
        })?;

    // 4. 转换请求类型，使用供应商模型名称
    let provider_req = convert_to_provider_request(req, &provider_model_name);

    // 5. 调用Provider的chat_completions方法
    let response = provider
        .chat_completions(provider_req, &serde_json::json!({}))
        .await
        .map_err(|e| format!("Provider error: {}", e))?;

    // 6. 转换响应类型，使用原始 model 标识符替换供应商返回的 model 字段
    convert_from_provider_response(response, &req.model)
}

/// 转换为Provider请求类型
fn convert_to_provider_request(
    req: &ChatCompletionRequest,
    provider_model_name: &str,
) -> crate::domain::dto::chat::ChatCompletionRequest {
    crate::domain::dto::chat::ChatCompletionRequest {
        model: provider_model_name.to_string(),
        messages: req.messages.clone(),
        stream: req.stream,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        top_p: req.top_p,
        frequency_penalty: req.frequency_penalty,
        presence_penalty: req.presence_penalty,
        stop: req.stop.clone(),
        response_format: req.response_format.clone(),
        tools: req.tools.clone(),
        tool_choice: req.tool_choice.clone(),
        user: req.user.clone(),
        logprobs: req.logprobs,
        top_logprobs: req.top_logprobs,
        n: req.n,
        max_completion_tokens: req.max_completion_tokens,
        parallel_tool_calls: req.parallel_tool_calls,
        logit_bias: req.logit_bias.clone(),
        reasoning: req.reasoning.clone(),
        extra_body: req.extra_body.clone(),
        extra_fields: req.extra_fields.clone(),
    }
}

/// 转换从Provider响应类型
fn convert_from_provider_response(
    response: crate::domain::dto::chat::ChatCompletionResponse,
    original_model: &str,
) -> std::result::Result<ChatCompletion, String> {
    match response {
        crate::domain::dto::chat::ChatCompletionResponse::NonStream(completion) => {
            Ok(ChatCompletion {
                id: completion.id,
                object: completion.object,
                created: completion.created,
                model: original_model.to_string(),
                choices: completion
                    .choices
                    .into_iter()
                    .map(|choice| ChatCompletionChoice {
                        index: choice.index,
                        message: ChatCompletionMessage {
                            role: choice.message.role,
                            content: choice.message.content.map(|content| match content {
                                crate::domain::dto::content::ChatMessageContent::String(s) => {
                                    ChatMessageContent::String(s)
                                }
                                crate::domain::dto::content::ChatMessageContent::Array(parts) => {
                                    ChatMessageContent::Array(
                                        parts
                                            .into_iter()
                                            .map(|part| ChatMessageContentPart {
                                                r#type: part.r#type,
                                                text: part.text,
                                                image_url: part.image_url,
                                            })
                                            .collect(),
                                    )
                                }
                            }),
                            name: choice.message.name,
                            tool_calls: choice.message.tool_calls,
                            tool_call_id: choice.message.tool_call_id,
                            refusal: choice.message.refusal,
                            reasoning_content: choice.message.reasoning_content,
                            extra_fields: choice.message.extra_fields,
                        },
                        finish_reason: choice.finish_reason,
                        logprobs: choice.logprobs,
                    })
                    .collect(),
                usage: Usage {
                    prompt_tokens: completion.usage.prompt_tokens,
                    completion_tokens: completion.usage.completion_tokens,
                    total_tokens: completion.usage.total_tokens,
                },
                system_fingerprint: completion.system_fingerprint,
            })
        }
        crate::domain::dto::chat::ChatCompletionResponse::Stream(_) => {
            Err("Stream response not supported in non-stream mode".to_string())
        }
    }
}
