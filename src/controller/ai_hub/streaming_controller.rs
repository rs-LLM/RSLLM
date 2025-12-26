//! 流式响应控制器模块
//! 提供SSE和WebSocket流式响应接口

use axum::{
    extract::{State, WebSocketUpgrade},
    http::HeaderMap,
    response::{IntoResponse, sse::{Event, Sse}},
    Json,
    extract::ws::WebSocket,
};
use std::sync::Arc;
use std::convert::Infallible;
use std::pin::Pin;

use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;
use futures::SinkExt;
use futures::Stream;
use futures::StreamExt;

// 导入相关类型
use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::chat::ChatCompletionRequest;
use crate::domain::vo::response::ApiResponse;
use crate::service::{TokenCounter, Content, TokenCountMeta};
use crate::domain::dto::ai_hub::content::{ChatMessageContent, ChatCompletionMessage};
use crate::domain::vo::ai_hub::streaming::{ChatCompletionChunk, ChatCompletionChunkChoice, ChatCompletionChunkDelta, WebSocketMessageType, WebSocketChatRequest, QuotaWarning};

/// 创建错误事件流
fn create_error_stream(error_message: String) -> Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> {
    let error_response = ApiResponse::<()>::error("500", &error_message);
    let error_event = Event::default().json_data(&error_response).unwrap_or_else(|_| Event::default().data("Unknown error"));
    let stream = futures::stream::once(async move { Ok::<Event, Infallible>(error_event) });
    Box::pin(stream)
}

/// SSE流式聊天补全接口
///
/// 提供Server-Sent Events流式响应
pub async fn chat_completions_sse(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Sse<Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>> {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    log::info!("[AI Hub] SSE chat completion request: {}", request_id);
    
    // 1. 用户认证
    let user_id = match authenticate_user(&headers, &state) {
        Ok(id) => id,
        Err(e) => {
            return Sse::new(Box::pin(create_error_stream(e.to_string())));
        }
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);
    
    // 2. Token计算
    let token_meta = match calculate_tokens(&req) {
        Ok(meta) => meta,
        Err(e) => {
            return Sse::new(Box::pin(create_error_stream(e.to_string())));
        }
    };
    
    // 3. 预消费和配额检查
    let billing_service = &state.billing_service;
    let (base_input_price, base_output_price) = get_pricing(&req.model);
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;
    
    let fee = match billing_service.calculate_and_check(
        &user_id,
        &req.model,
        "unknown",
        token_meta.input_tokens,
        estimated_output_tokens,
        base_input_price,
        base_output_price,
        "chat",
    ).await {
        Ok(fee) => fee,
        Err(e) => {
            return Sse::new(Box::pin(create_error_stream(e.to_string())));
        }
    };
    
    log::info!("[AI Hub] Pre-consumption check passed: cost={:.2}", fee.total_cost);
    
    // 克隆需要在tokio::spawn中使用的服务
    let billing_service = state.billing_service.clone();
    
    // 4. 创建SSE流
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);
    
    tokio::spawn(async move {
        // 发送初始块
        let init_chunk = ChatCompletionChunk {
            id: request_id.clone(),
            object: Some("chat.completion.chunk".to_string()),
            created: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()),
            model: req.model.clone(),
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatCompletionChunkDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            system_fingerprint: None,
        };
        
        if let Ok(event) = Event::default().json_data(&init_chunk) {
            let _ = tx.send(Ok(event)).await;
        }
        
        // 模拟流式响应（实际应该调用AI服务）
        let response_content = "This is a simulated streaming response for testing purposes.";
        for chunk in response_content.split_whitespace() {
            // 移除延迟以提高性能，满足200ms响应时间要求
            // 实际AI服务调用时，延迟由外部服务决定
            // tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            let stream_chunk = ChatCompletionChunk {
                id: request_id.clone(),
                object: Some("chat.completion.chunk".to_string()),
                created: Some(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()),
                model: req.model.clone(),
                choices: vec![ChatCompletionChunkChoice {
                    index: 0,
                    delta: ChatCompletionChunkDelta {
                        role: None,
                        content: Some(format!("{} ", chunk)),
                        tool_calls: None,
                    },
                    finish_reason: None,
                }],
                system_fingerprint: None,
            };
            
            if let Ok(event) = Event::default().json_data(&stream_chunk) {
                let _ = tx.send(Ok(event)).await;
            }
        }
        
        // 发送结束块
        let end_chunk = ChatCompletionChunk {
            id: request_id.clone(),
            object: Some("chat.completion.chunk".to_string()),
            created: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()),
            model: req.model.clone(),
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatCompletionChunkDelta {
                    role: None,
                    content: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            system_fingerprint: None,
        };
        
        if let Ok(event) = Event::default().json_data(&end_chunk) {
            let _ = tx.send(Ok(event)).await;
        }
        
        // 实际扣费和记录用量
        let duration_ms = start_time.elapsed().as_millis() as i64;
        let usage_log_id = match billing_service.deduct_quota_and_log(
            &fee,
            &request_id,
            duration_ms,
            "success",
            Some(serde_json::json!({
                "model": req.model,
                "stream": true,
            })),
        ).await {
            Ok(id) => id,
            Err(e) => {
                log::error!("[AI Hub] Failed to deduct quota and log: {}", e);
                return;
            }
        };
        
        log::info!("[AI Hub] Usage logged: {}", usage_log_id);
    });
    
    // 创建SSE流 - 使用具体的Stream类型而不是trait对象
    let stream = ReceiverStream::new(rx)
        .map(|result| result.unwrap_or_else(|_| Event::default().data("Stream error")))
        .map(|event| Ok::<Event, Infallible>(event));
    
    Sse::new(Box::pin(stream))
}

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
    let user_id = match authenticate_user(&headers, &state) {
        Ok(id) => id,
        Err(e) => {
            return ws.on_upgrade(|mut socket| async move {
                let error_msg = WebSocketMessageType::Error(e.to_string());
                let _ = socket.send(axum::extract::ws::Message::Text(
                    serde_json::to_string(&error_msg).unwrap_or_default().into()
                )).await;
                let _ = socket.close().await;
            });
        }
    };
    
    ws.on_upgrade(|mut socket| async move {
        let request_id = Uuid::new_v4().to_string();
        log::info!("[AI Hub] WebSocket connection established: {}", request_id);
        
        // 发送连接成功消息
        let connected_msg = WebSocketMessageType::Connected;
        let _ = socket.send(axum::extract::ws::Message::Text(
            serde_json::to_string(&connected_msg).unwrap_or_default().into()
        )).await;
        
        // 处理消息
        while let Some(message) = socket.recv().await {
            match message {
                Ok(axum::extract::ws::Message::Text(text)) => {
                    if let Ok(req) = serde_json::from_str::<WebSocketChatRequest>(&text) {
                        // 处理聊天请求
                        handle_ws_chat_request(&req, &request_id, &user_id, &state, &mut socket).await;
                    } else if let Ok(WebSocketMessageType::Pong) = serde_json::from_str::<WebSocketMessageType>(&text) {
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
    state: &Arc<ServiceContext>,
    socket: &mut WebSocket,
) {
    let start_time = std::time::Instant::now();
    
    // 转换请求格式
    let chat_req = ChatCompletionRequest {
        model: req.model.clone(),
        messages: req.messages.iter().map(|m| ChatCompletionMessage {
            role: m.role.clone(),
            content: Some(ChatMessageContent::String(m.content.clone())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
        }).collect(),
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
    };
    
    // Token计算和配额检查
    let token_meta = match calculate_tokens(&chat_req) {
        Ok(meta) => meta,
        Err(e) => {
            let error_msg = WebSocketMessageType::Error(e.to_string());
            let _ = socket.send(axum::extract::ws::Message::Text(
                serde_json::to_string(&error_msg).unwrap_or_default().into()
            )).await;
            return;
        }
    };
    
    let billing_service = &state.billing_service;
    let (base_input_price, base_output_price) = get_pricing(&req.model);
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;
    
    let fee = match billing_service.calculate_and_check(
        user_id,
        &req.model,
        "unknown",
        token_meta.input_tokens,
        estimated_output_tokens,
        base_input_price,
        base_output_price,
        "chat",
    ).await {
        Ok(fee) => fee,
        Err(e) => {
            let error_msg = WebSocketMessageType::Error(e.to_string());
            let _ = socket.send(axum::extract::ws::Message::Text(
                serde_json::to_string(&error_msg).unwrap_or_default().into()
            )).await;
            return;
        }
    };
    
    // 检查配额警告
    if let Ok(Some(warning)) = state.quota_service.check_quota_warning(user_id).await {
        let warning_msg = WebSocketMessageType::QuotaWarning(QuotaWarning {
            user_id: user_id.to_string(),
            remaining_quota: warning.remaining_quota,
            usage_rate: warning.current_usage_rate,
            message: warning.warning_message,
        });
        let _ = socket.send(axum::extract::ws::Message::Text(
            serde_json::to_string(&warning_msg).unwrap_or_default().into()
        )).await;
    }
    
    // 流式响应
    if req.stream.unwrap_or(false) {
        let response_content = "This is a simulated WebSocket streaming response.";
        for chunk in response_content.split_whitespace() {
            // 移除延迟以提高性能，满足200ms响应时间要求
            // 实际AI服务调用时，延迟由外部服务决定
            // tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            
            let stream_chunk = ChatCompletionChunk {
                id: request_id.to_string(),
                object: Some("chat.completion.chunk".to_string()),
                created: Some(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()),
                model: req.model.clone(),
                choices: vec![ChatCompletionChunkChoice {
                    index: 0,
                    delta: ChatCompletionChunkDelta {
                        role: None,
                        content: Some(format!("{} ", chunk)),
                        tool_calls: None,
                    },
                    finish_reason: None,
                }],
                system_fingerprint: None,
            };
            
            let msg = WebSocketMessageType::ChatChunk(stream_chunk);
            let _ = socket.send(axum::extract::ws::Message::Text(
                serde_json::to_string(&msg).unwrap_or_default().into()
            )).await;
        }
    } else {
        // 非流式响应
        let response_content = "This is a simulated WebSocket response.";
        let complete_msg = WebSocketMessageType::ChatComplete(crate::domain::vo::streaming::ChatCompletion {
            id: request_id.to_string(),
            object: Some("chat.completion".to_string()),
            created: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()),
            model: req.model.clone(),
            choices: vec![crate::domain::vo::streaming::ChatCompletionChoice {
                index: 0,
                message: crate::domain::vo::streaming::ChatCompletionMessage {
                    role: "assistant".to_string(),
                    content: Some(response_content.to_string()),
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
        
        let _ = socket.send(axum::extract::ws::Message::Text(
            serde_json::to_string(&complete_msg).unwrap_or_default().into()
        )).await;
    }
    
    // 记录用量
    let duration_ms = start_time.elapsed().as_millis() as i64;
    let _ = billing_service.deduct_quota_and_log(
        &fee,
        request_id,
        duration_ms,
        "success",
        Some(serde_json::json!({
            "model": req.model,
            "stream": req.stream.unwrap_or(false),
            "protocol": "websocket",
        })),
    ).await;
}

/// 用户认证
fn authenticate_user(headers: &HeaderMap, _state: &Arc<ServiceContext>) -> Result<String, String> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| "Missing or invalid authorization header".to_string())?;
    
    let jwt_token = crate::middleware::auth::checked_token(token)
        .map_err(|e| e.to_string())?;
    Ok(jwt_token.id)
}

/// 计算token数量
fn calculate_tokens(req: &ChatCompletionRequest) -> Result<TokenCountMeta, String> {
    let mut total_meta = TokenCountMeta::default();
    
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
                req.stream.unwrap_or(false),
            ).map_err(|e| e.to_string())?;
            
            total_meta.input_tokens += meta.input_tokens;
            total_meta.output_tokens += meta.output_tokens;
        }
    }
    
    Ok(total_meta)
}

/// 获取模型定价
fn get_pricing(model: &str) -> (f64, f64) {
    if model.contains("gpt-4") {
        (0.03, 0.06)
    } else if model.contains("gpt-3.5") {
        (0.0015, 0.002)
    } else if model.contains("claude") {
        (0.01, 0.03)
    } else {
        (0.01, 0.02)
    }
}