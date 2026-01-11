//! 流式响应控制器模块
//! 提供SSE和WebSocket流式响应接口

use axum::{
    extract::ws::WebSocket,
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
};
use std::sync::Arc;

use futures::SinkExt;
use ulid::Ulid;

// 导入相关类型
use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::chat::ChatCompletionRequest;
use crate::domain::dto::ai_hub::content::{ChatCompletionMessage, ChatMessageContent};
use crate::domain::vo::ai_hub::streaming::{
    ChatCompletionChunk, ChatCompletionChunkChoice, ChatCompletionChunkDelta, QuotaWarning,
    WebSocketChatRequest, WebSocketMessageType,
};
use crate::service::ai_hub::rate_limit_service::RateLimitCheckResult;
use crate::service::{Content, TokenCountMeta, TokenCounter};

/// WebSocket聊天补全接口
///
/// 提供WebSocket双向通信支持
#[utoipa::path(
    get,
    path = "/api/v1/chat/completions/ws",
    responses(
        (status = 101, description = "WebSocket连接建立成功"),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器错误")
    ),
    tag = "streaming",
    security(
        ("api_key" = [])
    )
)]
#[axum::debug_handler]
pub async fn chat_completions_ws(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServiceContext>>,
) -> impl IntoResponse {
    // 用户认证
    let (user_id, api_key) = match authenticate_user(&headers, &state).await {
        Ok((id, key)) => (id, key),
        Err(e) => {
            return ws.on_upgrade(|mut socket| async move {
                let error_msg = WebSocketMessageType::Error(e.to_string());
                let _ = socket
                    .send(axum::extract::ws::Message::Text(
                        serde_json::to_string(&error_msg).unwrap_or_default().into(),
                    ))
                    .await;
                let _ = socket.close().await;
            });
        }
    };

    ws.on_upgrade(move |mut socket| async move {
        let request_id = Ulid::new().to_string();
        log::info!("[AI Hub] WebSocket connection established: {}", request_id);

        let model_router = state.model_router.clone();

        // 发送连接成功消息
        let connected_msg = WebSocketMessageType::Connected;
        let _ = socket
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&connected_msg)
                    .unwrap_or_default()
                    .into(),
            ))
            .await;

        // 处理消息
        while let Some(message) = socket.recv().await {
            match message {
                Ok(axum::extract::ws::Message::Text(text)) => {
                    if let Ok(req) = serde_json::from_str::<WebSocketChatRequest>(&text) {
                        // 处理聊天请求
                        handle_ws_chat_request(
                            &req,
                            &request_id,
                            &user_id,
                            &api_key,
                            &state,
                            &model_router,
                            &mut socket,
                        )
                        .await;
                    } else if let Ok(WebSocketMessageType::Pong) =
                        serde_json::from_str::<WebSocketMessageType>(&text)
                    {
                        // 收到pong，保持连接
                        log::debug!("[AI Hub] Received pong from {}", request_id);
                    }
                }
                Ok(axum::extract::ws::Message::Close(_)) => {
                    log::info!("[AI Hub] WebSocket connection closed: {}", request_id);
                    break;
                }
                _ => {}
            }
        }

        log::info!("[AI Hub] WebSocket connection ended: {}", request_id);
    })
}

/// 处理WebSocket聊天请求
async fn handle_ws_chat_request(
    req: &WebSocketChatRequest,
    request_id: &str,
    user_id: &str,
    api_key: &str,
    state: &Arc<ServiceContext>,
    model_router: &crate::routers::model_router::ModelRouter,
    socket: &mut WebSocket,
) {
    let start_time = std::time::Instant::now();

    // 转换请求格式
    let chat_req = ChatCompletionRequest {
        model: req.model.clone(),
        messages: req
            .messages
            .iter()
            .map(|m| ChatCompletionMessage {
                role: m.role.clone(),
                content: Some(ChatMessageContent::String(m.content.clone())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                reasoning_content: None,
                extra_fields: serde_json::Value::default(),
            })
            .collect(),
        stream: req.stream,
        max_tokens: req.max_tokens.map(|v| v as u32),
        temperature: req.temperature.map(|v| v as f32),
        top_p: None, // WebSocketChatRequest 没有 top_p 字段
        frequency_penalty: Some(0.0),
        presence_penalty: Some(0.0),
        stop: None,
        response_format: None,
        tools: None,
        tool_choice: None,
        n: Some(1),
        max_completion_tokens: None,
        parallel_tool_calls: Some(false),
        logit_bias: None,
        user: None,
        logprobs: None,
        top_logprobs: None,
        reasoning: None,
        extra_body: None,
        extra_fields: serde_json::Value::default(),
    };

    // Token计算和配额检查
    let token_meta = match calculate_tokens(&chat_req, model_router).await {
        Ok(meta) => meta,
        Err(e) => {
            let error_msg = WebSocketMessageType::Error(e.to_string());
            let _ = socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&error_msg).unwrap_or_default().into(),
                ))
                .await;
            return;
        }
    };

    // 检查速率限制（使用实际的token数量）
    let input_tokens_i32 = token_meta.input_tokens as i32;
    let rate_limit_result = state
        .rate_limit_service
        .check_quota_with_tokens(user_id, input_tokens_i32)
        .await;
    match rate_limit_result {
        Ok(RateLimitCheckResult { allowed: true, .. }) => {
            log::info!("[AI Hub] Rate limit check passed for user {}", user_id);
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
            let error_msg = WebSocketMessageType::Error(error_msg);
            let _ = socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&error_msg).unwrap_or_default().into(),
                ))
                .await;
            return;
        }
        Err(e) => {
            log::error!("[AI Hub] Rate limit check failed: {}", e);
            let error_msg = WebSocketMessageType::Error(format!("Rate limit check failed: {}", e));
            let _ = socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&error_msg).unwrap_or_default().into(),
                ))
                .await;
            return;
        }
    }

    let billing_service = &state.billing_service;
    let (base_input_price, base_output_price) = get_pricing(&req.model, &state.model_router).await;
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;

    let fee = match billing_service
        .calculate_and_check(&crate::service::ai_hub::CalculateAndCheckParams {
            user_id,
            model_id: &req.model,
            api_key: &api_key,
            input_tokens: token_meta.input_tokens,
            output_tokens: estimated_output_tokens,
            base_input_price,
            base_output_price,
            request_type: "chat",
        })
        .await
    {
        Ok(fee) => fee,
        Err(e) => {
            let error_msg = WebSocketMessageType::Error(e.to_string());
            let _ = socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&error_msg).unwrap_or_default().into(),
                ))
                .await;
            return;
        }
    };

    // 检查配额警告
    if let Ok(Some(warning)) = state.quota_service.check_quota_warning(user_id).await {
        let warning_msg = WebSocketMessageType::QuotaWarning(QuotaWarning {
            user_id: user_id.to_string(),
            message: warning.warning_message,
        });
        let _ = socket
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&warning_msg)
                    .unwrap_or_default()
                    .into(),
            ))
            .await;
    }

    // 流式响应
    if req.stream.unwrap_or(false) {
        let response_content = "This is a simulated WebSocket streaming response.";
        let mut accumulated_content = String::new();

        for chunk in response_content.split_whitespace() {
            // 累计输出内容
            accumulated_content.push_str(chunk);
            accumulated_content.push(' ');

            // 移除延迟以提高性能，满足200ms响应时间要求
            // 实际AI服务调用时，延迟由外部服务决定
            // tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            let stream_chunk = ChatCompletionChunk {
                id: request_id.to_string(),
                object: Some("chat.completion.chunk".to_string()),
                created: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
                model: req.model.clone(),
                choices: vec![ChatCompletionChunkChoice {
                    index: 0,
                    delta: Some(ChatCompletionChunkDelta {
                        role: None,
                        content: Some(format!("{} ", chunk)),
                        reasoning_content: None,
                        tool_calls: None,
                        extra_fields: serde_json::Value::default(),
                    }),
                    finish_reason: None,
                    extra_fields: serde_json::Value::default(),
                }],
                system_fingerprint: None,
                extra_fields: serde_json::Value::default(),
            };

            let msg = WebSocketMessageType::ChatChunk(stream_chunk);
            let _ = socket
                .send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&msg).unwrap_or_default().into(),
                ))
                .await;
        }

        // 计算输出token数量
        let output_tokens = if !accumulated_content.is_empty() {
            match model_router.route_to_model(&req.model).await {
                Ok(model_def) => {
                    match TokenCounter::count_content_tokens(
                        &Content::Text(accumulated_content),
                        &req.model,
                        true,
                        Some(&model_def),
                    ) {
                        Ok(meta) => meta.output_tokens,
                        Err(e) => {
                            log::warn!("[AI Hub] Failed to count output tokens: {}", e);
                            0
                        }
                    }
                }
                Err(e) => {
                    log::warn!("[AI Hub] Failed to get model definition: {}", e);
                    0
                }
            }
        } else {
            0
        };

        // 消耗输出token到速率限制
        if output_tokens > 0 {
            if let Err(e) = state
                .rate_limit_service
                .consume_tokens(user_id, output_tokens as i32)
                .await
            {
                log::error!("[AI Hub] Failed to consume output tokens: {}", e);
            } else {
                log::info!(
                    "[AI Hub] Consumed {} output tokens for rate limit",
                    output_tokens
                );
            }
        }
    } else {
        // 非流式响应
        let response_content = "This is a simulated WebSocket response.";
        let complete_msg =
            WebSocketMessageType::ChatComplete(crate::domain::vo::streaming::ChatCompletion {
                id: request_id.to_string(),
                object: Some("chat.completion".to_string()),
                created: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                ),
                model: req.model.clone(),
                choices: vec![crate::domain::vo::streaming::ChatCompletionChoice {
                    index: 0,
                    message: crate::domain::vo::streaming::ChatCompletionMessage {
                        role: "assistant".to_string(),
                        content: Some(response_content.to_string()),
                        reasoning_content: None,
                        tool_calls: None,
                    },
                    finish_reason: Some("stop".to_string()),
                }],
                usage: crate::domain::vo::streaming::Usage {
                    prompt_tokens: token_meta.input_tokens as u32,
                    completion_tokens: 50,
                    total_tokens: (token_meta.input_tokens + 50) as u32,
                },
                system_fingerprint: None,
            });

        let _ = socket
            .send(axum::extract::ws::Message::Text(
                serde_json::to_string(&complete_msg)
                    .unwrap_or_default()
                    .into(),
            ))
            .await;
    }

    // 记录用量
    let duration_ms = start_time.elapsed().as_millis() as i64;
    let _ = billing_service
        .deduct_quota_and_log(
            &fee,
            &api_key,
            duration_ms,
            "success",
            Some(serde_json::json!({
                "model": req.model,
                "stream": req.stream.unwrap_or(false),
                "protocol": "websocket",
            })),
        )
        .await;
}

/// 用户认证
async fn authenticate_user(
    headers: &HeaderMap,
    state: &Arc<ServiceContext>,
) -> Result<(String, String), String> {
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
) -> Result<TokenCountMeta, String> {
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

    for message in &req.messages {
        if let Some(content) = &message.content {
            let content_enum = match content {
                ChatMessageContent::String(s) => Content::Text(s.clone()),
                ChatMessageContent::Array(parts) => {
                    let mut contents = Vec::new();
                    for part in parts {
                        if let Some(text) = &part.text {
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

            total_meta.input_tokens += meta.input_tokens;
            total_meta.output_tokens += meta.output_tokens;
        }
    }

    Ok(total_meta)
}

/// 获取模型定价
async fn get_pricing(
    model: &str,
    model_router: &crate::routers::model_router::ModelRouter,
) -> (f64, f64) {
    match model_router.route_to_model(model).await {
        Ok(model_def) => {
            log::info!(
                "[AI Hub] Retrieved model pricing from database: {} - input: {:.6}, output: {:.6}",
                model,
                model_def.model_base.input_price,
                model_def.model_base.output_price
            );
            (
                model_def.model_base.input_price,
                model_def.model_base.output_price,
            )
        }
        Err(e) => {
            log::warn!(
                "[AI Hub] Failed to get model definition from database: {}, using default pricing",
                e
            );
            (1.0, 1.0)
        }
    }
}
