//! AI Hub 聊天控制器模块
//!
//! 提供OpenAI兼容的聊天补全API接口

use axum::{
    Extension, Json,
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
use crate::domain::dto::ai_hub::streaming::{
    ChatCompletionChunk, ChatCompletionChunkChoice, ChatCompletionChunkDelta,
};
use crate::domain::dto::ai_hub::validation::ChatCompletionRequestParams;
use crate::domain::dto::chat::ChatCompletionRequest;
use crate::domain::dto::content::{
    ChatCompletionMessage, ChatMessageContent, ChatMessageContentPart,
};
use crate::domain::dto::provider::{ProviderConfig as ProviderConfigDTO, ProviderType};
use crate::domain::dto::validation::Validator;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::vo::ai_hub::responses::OpenAIErrorResponse;
use crate::domain::vo::chat::ChatCompletion;
use crate::domain::vo::chat::ChatCompletionChoice;
use crate::domain::vo::usage::Usage;
use crate::router::model_router::ModelRoutingInfo;
use crate::service::BillingService;
use crate::service::CalculatedFee;
use crate::service::ai_hub::ProviderConfigService;
use crate::service::ai_hub::rate_limit_service::RateLimitCheckResult;
use crate::service::ai_hub::{AiRequestContext, resolve_request_identity};
use crate::service::{Content, TokenCountMeta, TokenCounter};

pub fn create_openai_error_response(
    status: axum::http::StatusCode,
    error_message: impl Into<String>,
) -> Response {
    let mut response =
        Json(OpenAIErrorResponse::new(status.as_u16(), error_message)).into_response();
    *response.status_mut() = status;
    response
}

/// 创建SSE错误响应
fn create_sse_error_response(error_message: String) -> Response {
    let error_event = Event::default()
        .json_data(OpenAIErrorResponse::new(500, error_message))
        .unwrap_or_else(|_| Event::default().data("Unknown error"));
    let stream = futures::stream::once(async move { Ok::<Event, Infallible>(error_event) });
    Sse::new(Box::pin(stream)).into_response()
}

/// 处理非流式响应
async fn handle_non_streaming_response(
    req: ChatCompletionRequest,
    user_id: String,
    _api_key: String,
    fee: CalculatedFee,
    _request_id: String,
    usage_log_id: String,
    provider_config: ProviderConfigDTO,
    model_info: ModelRoutingInfo,
    ip_address: Option<String>,
    user_agent: Option<String>,
    state: Arc<ServiceContext>,
) -> Response {
    let billing_service = &state.billing_service;
    let rate_limit_service = state.rate_limit_service.clone();

    let start_time = std::time::Instant::now();

    // 调用AI服务
    let response = match call_provider_with_fallback(
        &state,
        &req,
        &user_id,
        &provider_config,
        &model_info,
        &usage_log_id,
    )
    .await
    {
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

            return create_openai_error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("AI 服务调用失败: {}", e),
            );
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
                    match TokenCounter::count_content_tokens(
                        &Content::Text(content_str),
                        &req.model,
                        true,
                        Some(&model_info),
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
            }

            // 计算推理内容的 token
            if let Some(reasoning_content) = &choice.message.reasoning_content {
                if !reasoning_content.is_empty() {
                    match TokenCounter::count_content_tokens(
                        &Content::Text(reasoning_content.clone()),
                        &req.model,
                        true,
                        Some(&model_info),
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
        .settle_output_tokens(&user_id, output_tokens as i32)
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
    let upstream =
        crate::service::ai_hub::provider::upstream_trace::get_upstream_oauth(&usage_log_id);

    let extra = {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "prompt_tokens".to_string(),
            serde_json::Value::Number(serde_json::Number::from(fee.input_tokens)),
        );
        obj.insert(
            "completion_tokens".to_string(),
            serde_json::Value::Number(serde_json::Number::from(output_tokens)),
        );
        obj.insert(
            "total_tokens".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                fee.input_tokens.saturating_add(output_tokens),
            )),
        );
        if let Some(upstream) = &upstream {
            obj.insert(
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
        Some(serde_json::Value::Object(obj))
    };

    let meta = crate::service::ai_hub::UsageLogMeta::chat_completions(
        ip_address.clone(),
        user_agent.clone(),
    );

    let completed = billing_service
        .complete_usage_log(
            &fee,
            &usage_log_id,
            &meta,
            output_tokens,
            duration_ms,
            extra,
            upstream.clone(),
        )
        .await;

    if let Err(e) = completed {
        log::error!("[AI Hub] Failed to complete usage log: {}", e);
    } else {
        crate::service::ai_hub::provider::upstream_trace::remove_upstream_oauth(&usage_log_id);
        if let Some(upstream) = upstream {
            crate::service::ai_hub::provider::oauth::record_provider_token_usage(
                &upstream.provider_id,
                &upstream.provider_type,
                &upstream.account_key,
                fee.input_tokens,
                output_tokens,
            );
        }
    }

    // 返回响应（余额已在 chat_completions 中预扣减）
    Json(response).into_response()
}

/// 处理流式响应
async fn handle_streaming_response(
    req: ChatCompletionRequest,
    user_id: String,
    _api_key: String,
    fee: CalculatedFee,
    request_id: String,
    usage_log_id: String,
    provider_config: ProviderConfigDTO,
    model_info: ModelRoutingInfo,
    ip_address: Option<String>,
    user_agent: Option<String>,
    state: Arc<ServiceContext>,
) -> Response {
    let billing_service = state.billing_service.clone();
    let rate_limit_service = state.rate_limit_service.clone();

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
                delta: Some(
                    crate::domain::dto::ai_hub::streaming::ChatCompletionChunkDelta {
                        role: Some("assistant".to_string()),
                        content: None,
                        reasoning_content: None,
                        tool_calls: None,
                        extra_fields: serde_json::Value::default(),
                    },
                ),
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

        // 调用AI服务获取流式响应（支持降级）
        let mut stream = match call_provider_stream_with_fallback(
            &state,
            &req,
            &user_id,
            &provider_config,
            &model_info,
            &usage_log_id,
        )
        .await
        {
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

                // 发送一个符合OpenAI规范的错误完成块
                let error_chunk = ChatCompletionChunk {
                    id: request_id.clone(),
                    object: Some("chat.completion.chunk".to_string()),
                    created: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    model: req.model.clone(),
                    choices: vec![ChatCompletionChunkChoice {
                        index: 0,
                        delta: Some(ChatCompletionChunkDelta {
                            role: Some("assistant".to_string()),
                            content: Some(format!("Error: {}", e)),
                            reasoning_content: None,
                            tool_calls: None,
                            extra_fields: serde_json::Value::default(),
                        }),
                        finish_reason: Some("error".to_string()),
                        logprobs: None,
                        extra_fields: serde_json::Value::default(),
                    }],
                    system_fingerprint: None,
                    extra_fields: serde_json::Value::default(),
                };

                if let Ok(event) = Event::default().json_data(&error_chunk) {
                    let _ = tx.send(Ok(event)).await;
                }

                // 发送终止事件
                let done_event = Event::default().data("[DONE]");
                let _ = tx.send(Ok(done_event)).await;
                return;
            }
        };

        // 累计输出内容
        let mut accumulated_content = String::new();
        let mut accumulated_reasoning_content = String::new();
        let mut provider_usage: Option<serde_json::Value> = None;
        let mut provider_sent_finish_reason = false;

        // 处理流式响应
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(mut chunk) => {
                    // 替换 model 字段为用户请求的原始模型标识符
                    chunk.model = req.model.clone();
                    // 替换 ID 为请求 ID
                    chunk.id = request_id.clone();

                    // 检查并保存供应商返回的 usage 信息
                    if let Some(usage) = chunk.extra_fields.get("usage") {
                        provider_usage = Some(usage.clone());
                        log::info!("[AI Hub] Found provider usage: {:?}", usage);
                    }

                    // 检测供应商是否已经发送了 finish_reason
                    if let Some(choice) = chunk.choices.first() {
                        if choice.finish_reason.is_some() {
                            provider_sent_finish_reason = true;
                            log::info!(
                                "[AI Hub] Provider sent finish_reason: {:?}",
                                choice.finish_reason
                            );
                        }

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

                    // 过滤掉空 choices 的 chunk（避免客户端显示"Empty assistant response"）
                    // 只保留有内容、有 finish_reason 或有重要 extra_fields 的 chunk
                    let should_forward = !chunk.choices.is_empty()
                        || chunk.choices.iter().any(|c| c.finish_reason.is_some())
                        || !chunk.extra_fields.is_null()
                            && !chunk
                                .extra_fields
                                .as_object()
                                .map_or(true, |obj| obj.is_empty());

                    if should_forward {
                        // 转换为SSE事件
                        if let Ok(event) = Event::default().json_data(&chunk) {
                            let _ = tx.send(Ok(event)).await;
                        }
                    } else {
                        log::info!(
                            "[AI Hub] Skipping empty chunk: choices empty, no finish_reason, no extra_fields"
                        );
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
                match TokenCounter::count_content_tokens(
                    &Content::Text(accumulated_content),
                    &req.model,
                    true,
                    Some(&model_info),
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
            } else {
                log::warn!("[AI Hub] accumulated_content is empty");
            }

            // 计算推理内容的 token
            if !accumulated_reasoning_content.is_empty() {
                match TokenCounter::count_content_tokens(
                    &Content::Text(accumulated_reasoning_content),
                    &req.model,
                    true,
                    Some(&model_info),
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
                        log::warn!("[AI Hub] Failed to count reasoning content tokens: {}", e);
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
            .settle_output_tokens(&user_id, output_tokens as i32)
            .await
        {
            log::error!("[AI Hub] Failed to consume tokens: {}", e);
        } else {
            log::info!(
                "[AI Hub] Consumed {} output tokens for rate limit",
                output_tokens
            );
        }

        // 只有当供应商没有发送 finish_reason 时，才发送结束块
        if !provider_sent_finish_reason {
            log::info!("[AI Hub] Provider did not send finish_reason, creating end chunk");

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
                extra_fields: provider_usage
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| serde_json::Value::default()),
            };

            if let Ok(event) = Event::default().json_data(&end_chunk) {
                let _ = tx.send(Ok(event)).await;
            }
        } else {
            log::info!("[AI Hub] Provider already sent finish_reason, skipping end chunk");
        }

        // 发送 [DONE] 结束标记
        let done_event = Event::default().data("[DONE]");
        let _ = tx.send(Ok(done_event)).await;

        log::info!(
            "[AI Hub] Streaming response completed successfully (balance already pre-deducted, {} output tokens consumed)",
            output_tokens
        );

        // 创建完整的用量记录并扣减输出费用
        let duration_ms = start_time.elapsed().as_millis() as i64;

        let upstream =
            crate::service::ai_hub::provider::upstream_trace::get_upstream_oauth(&usage_log_id);

        // 克隆provider_usage以便后续使用
        let provider_usage_clone = provider_usage.clone();

        // 确定使用的 token 数量：优先使用供应商返回的，否则使用自己计算的
        let (final_input_tokens, final_output_tokens) = if let Some(usage) = provider_usage_clone {
            let prompt_tokens = usage
                .get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(fee.input_tokens);
            let completion_tokens = usage
                .get("completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(output_tokens);
            log::info!(
                "[AI Hub] Using provider token counts: prompt_tokens={}, completion_tokens={}",
                prompt_tokens,
                completion_tokens
            );
            (prompt_tokens, completion_tokens)
        } else {
            log::info!(
                "[AI Hub] Using calculated token counts: input_tokens={}, output_tokens={}",
                fee.input_tokens,
                output_tokens
            );
            (fee.input_tokens, output_tokens)
        };

        log::info!(
            "[AI Hub] Calling complete_usage_log (streaming): usage_log_id={}, user_id={}, model_id={}, input_tokens={}, output_tokens={}",
            usage_log_id,
            user_id,
            req.model,
            final_input_tokens,
            final_output_tokens
        );

        let provider_usage_with_upstream = if let Some(upstream) = &upstream {
            if let Some(mut map) = provider_usage.clone().and_then(|v| v.as_object().cloned()) {
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
                Some(serde_json::Value::Object(map))
            } else {
                Some(serde_json::json!({
                    "upstream_oauth": {
                        "provider_id": upstream.provider_id,
                        "provider_type": upstream.provider_type,
                        "account_key": upstream.account_key,
                        "account_id": upstream.account_id,
                        "email": upstream.email,
                    }
                }))
            }
        } else {
            provider_usage.clone()
        };

        let meta = crate::service::ai_hub::UsageLogMeta::chat_completions(
            ip_address.clone(),
            user_agent.clone(),
        );

        let completed = billing_service
            .complete_usage_log(
                &fee,
                &usage_log_id,
                &meta,
                final_output_tokens,
                duration_ms,
                provider_usage_with_upstream.clone(),
                upstream.clone(),
            )
            .await;

        if let Err(e) = completed {
            log::error!("[AI Hub] Failed to complete usage log: {}", e);
        } else {
            crate::service::ai_hub::provider::upstream_trace::remove_upstream_oauth(&usage_log_id);
            if let Some(upstream) = &upstream {
                crate::service::ai_hub::provider::oauth::record_provider_token_usage(
                    &upstream.provider_id,
                    &upstream.provider_type,
                    &upstream.account_key,
                    final_input_tokens,
                    final_output_tokens,
                );
            }
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
        (status = 400, description = "参数错误", body = OpenAIErrorResponse),
        (status = 401, description = "未授权", body = OpenAIErrorResponse),
        (status = 429, description = "请求过多", body = OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = OpenAIErrorResponse)
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
    request_context: Option<Extension<AiRequestContext>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    let request_id = Ulid::new().to_string();
    let start_time = std::time::Instant::now();

    log::info!("[AI Hub] Chat completion request: {}", request_id);

    // 1. 用户认证并获取API密钥
    let (user_id, api_key) = match authenticate_user(
        &headers,
        &state,
        request_context.as_ref().map(|context| &context.0),
    )
    .await
    {
        Ok((id, key)) => (id, key),
        Err(e) => {
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(e);
            } else {
                return create_openai_error_response(axum::http::StatusCode::UNAUTHORIZED, e);
            }
        }
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);

    // 2. 路由模型以获取模型配置信息
    let model_router = &state.model_router;
    let (provider_config, model_info) = match model_router.route(&req.model, true).await {
        Ok(result) => result,
        Err(e) => {
            let error_msg = format!("Failed to route model: {}", e);
            log::warn!("[AI Hub] {}", error_msg);
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return create_openai_error_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &error_msg,
                );
            }
        }
    };

    log::info!(
        "[AI Hub] Routed to provider: {}, model: {}",
        provider_config.provider_code,
        model_info.model_base.model_code
    );

    // 3. 计算动态内容长度限制
    let max_text_length = if let Some(max_tokens) = model_info.model_base.max_tokens_per_request {
        max_tokens as usize * 4
    } else {
        crate::domain::dto::validation::MAX_TEXT_LENGTH
    };

    log::info!(
        "[AI Hub] Using max_text_length: {} (model max_tokens_per_request: {:?})",
        max_text_length,
        model_info.model_base.max_tokens_per_request
    );

    // 4. 输入验证（使用动态长度限制）
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
        Ok(_) => log::info!("[AI Hub] Basic validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Basic validation failed: {}", e);
            let error_msg = format!("输入验证失败: {}", e);
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return create_openai_error_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    &error_msg,
                );
            }
        }
    }

    match Validator::validate_messages_with_limit(&req.messages, max_text_length) {
        Ok(_) => log::info!("[AI Hub] Message length validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Message length validation failed: {}", e);
            let error_msg = format!("输入验证失败: {}", e);
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return create_openai_error_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    &error_msg,
                );
            }
        }
    }

    // 5. Token计算
    let token_meta = match calculate_tokens(&req, &model_info).await {
        Ok(meta) => meta,
        Err(e) => {
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(e);
            } else {
                return create_openai_error_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &e,
                );
            }
        }
    };
    log::info!(
        "[AI Hub] Token calculation: input={}, model={}",
        token_meta.input_tokens,
        req.model
    );

    // 5.5. 检查速率限制（使用实际的token数量）
    let input_tokens_i32 = token_meta.input_tokens as i32;
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
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return create_openai_error_response(
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    &error_msg,
                );
            }
        }
        Err(e) => {
            log::error!("[AI Hub] Rate limit check failed: {}", e);
            let error_msg = format!("Rate limit check failed: {}", e);
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(error_msg);
            } else {
                return create_openai_error_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &error_msg,
                );
            }
        }
    }

    // 4. 获取动态定价并预消费和配额检查
    let billing_service = &state.billing_service;

    // 预估输出token（基于max_tokens或默认值）
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;

    // 获取动态定价
    let fee = match get_dynamic_pricing(
        &user_id,
        &req.model,
        token_meta.input_tokens,
        estimated_output_tokens,
        billing_service,
        &api_key,
        model_info.model_base.input_price,
        model_info.model_base.output_price,
    )
    .await
    {
        Ok(fee) => fee,
        Err(e) => {
            if req.stream.unwrap_or(false) {
                return create_sse_error_response(e);
            } else {
                return create_openai_error_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &e,
                );
            }
        }
    };

    log::info!(
        "[AI Hub] Pre-consumption check passed: cost={:.2}",
        fee.total_cost
    );

    // 4.5. 预扣减余额
    let _duration_ms = start_time.elapsed().as_millis() as i64;
    let (ip_address, user_agent) = crate::controller::ai_hub::extract_client_meta(&headers);

    let meta = crate::service::ai_hub::UsageLogMeta::chat_completions(
        ip_address.clone(),
        user_agent.clone(),
    );

    let _usage_log_id = match billing_service
        .pre_deduct_quota_and_log(
            &fee,
            &meta,
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
                return create_openai_error_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    &error_msg,
                );
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
            provider_config.clone(),
            model_info.clone(),
            ip_address.clone(),
            user_agent.clone(),
            state,
        )
        .await
    } else {
        // 非流式响应
        handle_non_streaming_response(
            req,
            user_id,
            api_key,
            fee,
            request_id,
            _usage_log_id,
            provider_config,
            model_info,
            ip_address,
            user_agent,
            state,
        )
        .await
    }
}

/// 用户认证（仅支持API key）
pub async fn authenticate_user(
    headers: &HeaderMap,
    state: &Arc<ServiceContext>,
    request_context: Option<&AiRequestContext>,
) -> std::result::Result<(String, String), String> {
    let identity = resolve_request_identity(headers, state, request_context).await?;
    Ok((identity.user_id, identity.api_key))
}

pub fn extract_api_key(headers: &HeaderMap) -> std::result::Result<String, String> {
    crate::service::ai_hub::extract_api_key(headers)
}

/// 计算token数量
pub async fn calculate_tokens(
    req: &ChatCompletionRequest,
    model_info: &ModelRoutingInfo,
) -> std::result::Result<TokenCountMeta, String> {
    log::info!(
        "[AI Hub] calculate_tokens called: model={}, messages_count={}",
        req.model,
        req.messages.len()
    );

    let mut total_meta = TokenCountMeta::default();

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
                Some(model_info),
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
pub async fn get_dynamic_pricing(
    user_id: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    billing_service: &BillingService,
    api_key: &str,
    base_input_price: f64,
    base_output_price: f64,
) -> std::result::Result<CalculatedFee, String> {
    log::info!(
        "[AI Hub] get_dynamic_pricing called: user_id={}, model={}, input_tokens={}, output_tokens={}",
        user_id,
        model,
        input_tokens,
        output_tokens
    );

    log::info!(
        "[AI Hub] Using routed model pricing: {} - input: {:.6}, output: {:.6}",
        model,
        base_input_price,
        base_output_price
    );

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
        fee.input_price,
        fee.output_price
    );

    Ok(fee)
}

/// 检测错误是否为429 Too Many Requests
fn is_rate_limit_error(error: &str) -> bool {
    error.contains("429")
        || error.contains("Too Many Requests")
        || error.contains("1302")
        || error.contains("并发数过高")
}

fn is_unauthorized_error(error: &str) -> bool {
    error.contains("401") || error.contains("Unauthorized")
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

/// 调用AI Provider获取真实流式响应（支持降级）
pub async fn call_provider_stream_with_fallback(
    state: &Arc<ServiceContext>,
    req: &ChatCompletionRequest,
    _user_id: &str,
    provider_config: &ProviderConfigDTO,
    model_info: &ModelRoutingInfo,
    trace_key: &str,
) -> std::result::Result<
    futures::stream::BoxStream<
        'static,
        Result<crate::domain::dto::ai_hub::streaming::ChatCompletionChunk, reqwest::Error>,
    >,
    String,
> {
    let model_id = model_info
        .model_base
        .id
        .clone()
        .ok_or_else(|| "Model ID not found".to_string())?;
    let provider_id = provider_config.id.as_str();

    let mappings = get_all_mappings(&model_id, provider_id).await?;

    if mappings.is_empty() {
        return Err("No mappings found for this model".to_string());
    }

    let mut last_error = String::new();

    for (index, mapping) in mappings.iter().enumerate() {
        let mut retried_after_unauthorized = false;
        log::info!(
            "[AI Hub] Trying mapping {}/{}: provider_model_name={}, priority={:?}",
            index + 1,
            mappings.len(),
            mapping.provider_model_name,
            mapping.priority
        );

        if provider_config.provider_type == ProviderType::Codex {
            if let Some(provider_config_service) =
                state.provider_config_service.read().await.as_ref()
            {
                if let Err(e) = provider_config_service
                    .get_provider_oauth_status(provider_id)
                    .await
                {
                    log::warn!(
                        "[AI Hub] Codex pre-refresh status check failed for provider {}: {}",
                        provider_id,
                        e
                    );
                }
            }
        }

        let provider_req = convert_to_provider_request(req, &mapping.provider_model_name);

        loop {
            let provider = {
                let provider_registry = state.provider_registry.read().await;
                match provider_registry.get_provider(&provider_config.provider_code) {
                    Some(p) => p,
                    None => {
                        last_error = format!(
                            "Provider not found for provider_code: {}",
                            provider_config.provider_code
                        );
                        log::warn!("[AI Hub] {}", last_error);
                        break;
                    }
                }
            };

            let model_config = serde_json::json!({
                "rsllm": {
                    "trace_key": trace_key,
                    "provider_id": provider_id,
                    "provider_type": provider_config.provider_type.to_string(),
                }
            });

            match provider
                .chat_completions(provider_req.clone(), &model_config)
                .await
            {
                Ok(response) => {
                    log::info!(
                        "[AI Hub] Successfully called provider with mapping {}/{}",
                        index + 1,
                        mappings.len()
                    );
                    match response {
                        crate::domain::dto::chat::ChatCompletionResponse::Stream(stream) => {
                            return Ok(stream);
                        }
                        crate::domain::dto::chat::ChatCompletionResponse::NonStream(_) => {
                            last_error =
                                "Expected stream response but got non-stream response".to_string();
                            log::warn!("[AI Hub] {}", last_error);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("Provider error: {}", e);
                    log::error!(
                        "[AI Hub] Mapping {}/{} failed: {}",
                        index + 1,
                        mappings.len(),
                        last_error
                    );

                    if provider_config.provider_type == ProviderType::Codex
                        && !retried_after_unauthorized
                        && is_unauthorized_error(&last_error)
                    {
                        retried_after_unauthorized = true;
                        if let Some(provider_config_service) =
                            state.provider_config_service.read().await.as_ref()
                        {
                            if let Err(refresh_err) = provider_config_service
                                .get_provider_oauth_status(provider_id)
                                .await
                            {
                                log::warn!(
                                    "[AI Hub] Codex oauth status re-check failed for provider {}: {}",
                                    provider_id,
                                    refresh_err
                                );
                            } else {
                                log::info!(
                                    "[AI Hub] Codex oauth status re-checked, retrying mapping {}/{}",
                                    index + 1,
                                    mappings.len()
                                );
                            }
                        }
                        continue;
                    }

                    if is_rate_limit_error(&last_error) {
                        log::info!("[AI Hub] Rate limit error detected, trying next mapping...");
                        break;
                    } else {
                        return Err(last_error);
                    }
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

/// 调用AI Provider获取真实响应（支持降级）
pub async fn call_provider_with_fallback(
    state: &Arc<ServiceContext>,
    req: &ChatCompletionRequest,
    _user_id: &str,
    provider_config: &ProviderConfigDTO,
    model_info: &ModelRoutingInfo,
    trace_key: &str,
) -> std::result::Result<ChatCompletion, String> {
    let model_id = model_info
        .model_base
        .id
        .clone()
        .ok_or_else(|| "Model ID not found".to_string())?;
    let provider_id = provider_config.id.as_str();

    let mappings = get_all_mappings(&model_id, provider_id).await?;

    if mappings.is_empty() {
        return Err("No mappings found for this model".to_string());
    }

    let mut last_error = String::new();

    for (index, mapping) in mappings.iter().enumerate() {
        let mut retried_after_unauthorized = false;
        log::info!(
            "[AI Hub] Trying mapping {}/{}: provider_model_name={}, priority={:?}",
            index + 1,
            mappings.len(),
            mapping.provider_model_name,
            mapping.priority
        );

        if provider_config.provider_type == ProviderType::Codex {
            if let Some(provider_config_service) =
                state.provider_config_service.read().await.as_ref()
            {
                if let Err(e) = provider_config_service
                    .get_provider_oauth_status(provider_id)
                    .await
                {
                    log::warn!(
                        "[AI Hub] Codex pre-refresh status check failed for provider {}: {}",
                        provider_id,
                        e
                    );
                }
            }
        }

        let provider_req = convert_to_provider_request(req, &mapping.provider_model_name);

        loop {
            let provider = {
                let provider_registry = state.provider_registry.read().await;
                match provider_registry.get_provider(&provider_config.provider_code) {
                    Some(p) => p,
                    None => {
                        last_error = format!(
                            "Provider not found for provider_code: {}",
                            provider_config.provider_code
                        );
                        log::warn!("[AI Hub] {}", last_error);
                        break;
                    }
                }
            };

            let model_config = serde_json::json!({
                "rsllm": {
                    "trace_key": trace_key,
                    "provider_id": provider_id,
                    "provider_type": provider_config.provider_type.to_string(),
                }
            });

            match provider
                .chat_completions(provider_req.clone(), &model_config)
                .await
            {
                Ok(response) => {
                    log::info!(
                        "[AI Hub] Successfully called provider with mapping {}/{}",
                        index + 1,
                        mappings.len()
                    );
                    match response {
                        crate::domain::dto::chat::ChatCompletionResponse::NonStream(completion) => {
                            return convert_from_provider_response(
                                crate::domain::dto::chat::ChatCompletionResponse::NonStream(
                                    completion,
                                ),
                                &req.model,
                            );
                        }
                        crate::domain::dto::chat::ChatCompletionResponse::Stream(_) => {
                            last_error =
                                "Expected non-stream response but got stream response".to_string();
                            log::warn!("[AI Hub] {}", last_error);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    last_error = format!("Provider error: {}", e);
                    log::error!(
                        "[AI Hub] Mapping {}/{} failed: {}",
                        index + 1,
                        mappings.len(),
                        last_error
                    );

                    if provider_config.provider_type == ProviderType::Codex
                        && !retried_after_unauthorized
                        && is_unauthorized_error(&last_error)
                    {
                        retried_after_unauthorized = true;
                        if let Some(provider_config_service) =
                            state.provider_config_service.read().await.as_ref()
                        {
                            if let Err(refresh_err) = provider_config_service
                                .get_provider_oauth_status(provider_id)
                                .await
                            {
                                log::warn!(
                                    "[AI Hub] Codex oauth status re-check failed for provider {}: {}",
                                    provider_id,
                                    refresh_err
                                );
                            } else {
                                log::info!(
                                    "[AI Hub] Codex oauth status re-checked, retrying mapping {}/{}",
                                    index + 1,
                                    mappings.len()
                                );
                            }
                        }
                        continue;
                    }

                    if is_rate_limit_error(&last_error) {
                        log::info!("[AI Hub] Rate limit error detected, trying next mapping...");
                        break;
                    } else {
                        return Err(last_error);
                    }
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
