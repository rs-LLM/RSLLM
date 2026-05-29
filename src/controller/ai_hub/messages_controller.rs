use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
};
use futures::StreamExt;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
};
use tokio_stream::wrappers::ReceiverStream;
use ulid::Ulid;

use crate::context::ServiceContext;
use crate::controller::ai_hub::chat_controller;
use crate::domain::dto::ai_hub::messages::ClaudeMessagesRequest;
use crate::domain::dto::ai_hub::streaming::ChatCompletionChunk;
use crate::domain::dto::ai_hub::validation::{ChatCompletionRequestParams, Validator};
use crate::domain::dto::chat::ChatCompletionRequest;
use crate::domain::dto::content::{ChatMessageContent, ChatMessageToolCall};
use crate::domain::vo::ai_hub::messages::{
    ClaudeContentBlock, ClaudeContentBlockDeltaEvent, ClaudeContentBlockStartEvent,
    ClaudeContentBlockStopEvent, ClaudeContentDelta, ClaudeErrorBody, ClaudeErrorEvent,
    ClaudeErrorResponse, ClaudeMessageDelta, ClaudeMessageDeltaEvent, ClaudeMessageResponse,
    ClaudeMessageStartEvent, ClaudeMessageStopEvent, ClaudeUsage,
};
use crate::router::model_router::ModelRoutingInfo;
use crate::service::ai_hub::AiRequestContext;
use crate::service::ai_hub::rate_limit_service::RateLimitCheckResult;

fn client_safe_error_message(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "Invalid Claude Messages request",
        StatusCode::UNAUTHORIZED => "Authentication failed",
        StatusCode::TOO_MANY_REQUESTS => "Rate limit exceeded",
        StatusCode::PAYMENT_REQUIRED => "Billing check failed",
        _ => "Claude Messages request failed",
    }
}

fn create_claude_error_response(status: StatusCode, message: impl Into<String>) -> Response {
    let _internal_message = message.into();
    let mut response = Json(ClaudeErrorResponse {
        error: ClaudeErrorBody {
            error_type: match status {
                StatusCode::BAD_REQUEST => "invalid_request_error".to_string(),
                StatusCode::UNAUTHORIZED => "authentication_error".to_string(),
                StatusCode::TOO_MANY_REQUESTS => "rate_limit_error".to_string(),
                StatusCode::PAYMENT_REQUIRED => "billing_error".to_string(),
                _ => "api_error".to_string(),
            },
            message: client_safe_error_message(status).to_string(),
        },
    })
    .into_response();
    *response.status_mut() = status;
    apply_anthropic_headers(response)
}

fn create_claude_sse_error_response(message: impl Into<String>) -> Response {
    let error_event = Event::default()
        .event("error")
        .json_data(ClaudeErrorEvent {
            event_type: "error".to_string(),
            error: ClaudeErrorBody {
                error_type: "api_error".to_string(),
                message: message.into(),
            },
        })
        .unwrap_or_else(|_| Event::default().event("error").data("Unknown error"));
    let stream = futures::stream::once(async move { Ok::<Event, Infallible>(error_event) });
    let mut response = Sse::new(Box::pin(stream)).into_response();
    *response.status_mut() = StatusCode::BAD_REQUEST;
    apply_anthropic_headers(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dto::content::{ChatCompletionMessage, ChatMessageFunction};
    use crate::domain::vo::ai_hub::chat::{ChatCompletion, ChatCompletionChoice};
    use crate::domain::vo::ai_hub::usage::Usage;
    use axum::body::to_bytes;

    #[test]
    fn validate_anthropic_headers_requires_header() {
        let headers = HeaderMap::new();
        let err = validate_anthropic_headers(&headers).unwrap_err();
        assert_eq!(err, "anthropic-version header is required");
    }

    #[test]
    fn completion_to_claude_message_includes_tool_use_blocks() {
        let completion = ChatCompletion {
            id: "msg_123".to_string(),
            object: Some("chat.completion".to_string()),
            created: Some(1),
            model: "claude-test".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatCompletionMessage {
                    role: "assistant".to_string(),
                    content: Some(ChatMessageContent::String("hello".to_string())),
                    name: None,
                    tool_calls: Some(vec![ChatMessageToolCall {
                        id: "toolu_123".to_string(),
                        function: ChatMessageFunction {
                            name: "get_weather".to_string(),
                            arguments: r#"{"city":"Paris"}"#.to_string(),
                        },
                        r#type: "function".to_string(),
                    }]),
                    tool_call_id: None,
                    refusal: None,
                    reasoning_content: None,
                    extra_fields: serde_json::Value::default(),
                },
                finish_reason: Some("tool_calls".to_string()),
                logprobs: None,
            }],
            usage: Usage {
                prompt_tokens: 12,
                completion_tokens: 7,
                total_tokens: 19,
            },
            system_fingerprint: None,
        };

        let response = completion_to_claude_message(&completion);

        assert_eq!(response.stop_reason.as_deref(), Some("tool_use"));
        assert_eq!(response.content.len(), 2);
        assert_eq!(response.content[0].block_type, "text");
        assert_eq!(response.content[0].text.as_deref(), Some("hello"));
        assert_eq!(response.content[1].block_type, "tool_use");
        assert_eq!(response.content[1].id.as_deref(), Some("toolu_123"));
        assert_eq!(response.content[1].name.as_deref(), Some("get_weather"));
        assert_eq!(
            response.content[1].input,
            Some(serde_json::json!({"city": "Paris"}))
        );
    }

    #[tokio::test]
    async fn process_stream_chunk_emits_text_then_tool_use_events_in_order() {
        use tokio::sync::mpsc;

        let chunk = ChatCompletionChunk {
            id: "chunk_1".to_string(),
            object: Some("chat.completion.chunk".to_string()),
            created: 1,
            model: "claude-test".to_string(),
            system_fingerprint: None,
            choices: vec![
                crate::domain::dto::ai_hub::streaming::ChatCompletionChunkChoice {
                    index: 0,
                    delta: Some(
                        crate::domain::dto::ai_hub::streaming::ChatCompletionChunkDelta {
                            role: Some("assistant".to_string()),
                            content: Some("hello".to_string()),
                            reasoning_content: None,
                            tool_calls: Some(vec![ChatMessageToolCall {
                                id: "toolu_123".to_string(),
                                function: ChatMessageFunction {
                                    name: "get_weather".to_string(),
                                    arguments: r#"{"city":"Paris"}"#.to_string(),
                                },
                                r#type: "function".to_string(),
                            }]),
                            extra_fields: serde_json::Value::default(),
                        },
                    ),
                    finish_reason: Some("tool_calls".to_string()),
                    logprobs: None,
                    extra_fields: serde_json::Value::default(),
                },
            ],
            extra_fields: serde_json::json!({
                "usage": {
                    "completion_tokens": 9
                }
            }),
        };

        let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(16);
        let mut output_tokens = 0u32;
        let mut final_stop_reason = None;
        let mut text_block_started = false;
        let mut emitted_tool_call_ids = HashSet::new();
        let mut tool_block_indexes = HashMap::new();
        let mut next_block_index = 1u32;

        process_stream_chunk(
            &tx,
            &chunk,
            &mut output_tokens,
            &mut final_stop_reason,
            &mut text_block_started,
            &mut emitted_tool_call_ids,
            &mut tool_block_indexes,
            &mut next_block_index,
        )
        .await;

        if text_block_started {
            let event = Event::default()
                .event("content_block_stop")
                .json_data(ClaudeContentBlockStopEvent {
                    event_type: "content_block_stop".to_string(),
                    index: 0,
                })
                .unwrap();
            tx.send(Ok(event)).await.unwrap();
        }

        drop(tx);

        let stream = ReceiverStream::new(rx)
            .map(|result| result.unwrap())
            .map(Ok::<Event, Infallible>);
        let response = Sse::new(Box::pin(stream)).into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sse = String::from_utf8(body.to_vec()).unwrap();

        let text_start = sse.find("event: content_block_start").unwrap();
        let text_delta = sse.find("\"type\":\"text_delta\"").unwrap();
        let tool_start = sse.find("\"type\":\"tool_use\"").unwrap();
        let tool_delta = sse.find("\"type\":\"input_json_delta\"").unwrap();
        let text_stop = sse.rfind("\"index\":0").unwrap();

        assert!(text_start < text_delta);
        assert!(text_delta < tool_start);
        assert!(tool_start < tool_delta);
        assert!(tool_delta < text_stop);
        assert!(sse.contains("\"index\":1"));
        assert!(sse.contains("\"partial_json\":\"{\\\"city\\\":\\\"Paris\\\"}\""));
        assert_eq!(output_tokens, 9);
        assert_eq!(final_stop_reason.as_deref(), Some("tool_use"));
    }
    #[tokio::test]
    async fn create_claude_error_response_sets_protocol_headers() {
        let response =
            create_claude_error_response(StatusCode::TOO_MANY_REQUESTS, "internal detail");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get("anthropic-version")
                .and_then(|v| v.to_str().ok()),
            Some("2023-06-01")
        );
        assert_eq!(
            response
                .headers()
                .get("x-accel-buffering")
                .and_then(|v| v.to_str().ok()),
            Some("no")
        );

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["type"], "rate_limit_error");
        assert_eq!(json["error"]["message"], "Rate limit exceeded");
    }
}

fn apply_anthropic_headers(mut response: Response) -> Response {
    response.headers_mut().insert(
        HeaderName::from_static("anthropic-version"),
        HeaderValue::from_static("2023-06-01"),
    );
    response.headers_mut().insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("no-store, no-cache"),
    );
    response.headers_mut().insert(
        HeaderName::from_static("pragma"),
        HeaderValue::from_static("no-cache"),
    );
    response.headers_mut().insert(
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    response
}

fn validate_anthropic_headers(headers: &HeaderMap) -> Result<(), String> {
    let version = headers
        .get("anthropic-version")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "anthropic-version header is required".to_string())?;

    if version.trim().is_empty() {
        return Err("anthropic-version header cannot be empty".to_string());
    }

    Ok(())
}

fn map_stop_reason(finish_reason: Option<&str>) -> Option<String> {
    match finish_reason {
        Some("stop") => Some("end_turn".to_string()),
        Some("length") => Some("max_tokens".to_string()),
        Some("tool_calls") => Some("tool_use".to_string()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

fn tool_call_to_claude_block(tool_call: &ChatMessageToolCall) -> ClaudeContentBlock {
    ClaudeContentBlock {
        block_type: "tool_use".to_string(),
        text: None,
        id: Some(tool_call.id.clone()),
        name: Some(tool_call.function.name.clone()),
        input: serde_json::from_str(&tool_call.function.arguments).ok(),
    }
}

fn content_to_claude_blocks(content: Option<&ChatMessageContent>) -> Vec<ClaudeContentBlock> {
    match content {
        Some(ChatMessageContent::String(text)) => vec![ClaudeContentBlock {
            block_type: "text".to_string(),
            text: Some(text.clone()),
            id: None,
            name: None,
            input: None,
        }],
        Some(ChatMessageContent::Array(parts)) => parts
            .iter()
            .filter(|part| part.r#type == "text")
            .map(|part| ClaudeContentBlock {
                block_type: "text".to_string(),
                text: Some(part.text.clone().unwrap_or_default()),
                id: None,
                name: None,
                input: None,
            })
            .collect(),
        None => Vec::new(),
    }
}

fn completion_to_claude_message(
    completion: &crate::domain::vo::chat::ChatCompletion,
) -> ClaudeMessageResponse {
    let first_choice = completion.choices.first();
    let mut content = first_choice
        .map(|choice| content_to_claude_blocks(choice.message.content.as_ref()))
        .unwrap_or_default();

    if let Some(tool_calls) = first_choice.and_then(|choice| choice.message.tool_calls.as_ref()) {
        content.extend(tool_calls.iter().map(tool_call_to_claude_block));
    }

    let stop_reason =
        first_choice.and_then(|choice| map_stop_reason(choice.finish_reason.as_deref()));

    ClaudeMessageResponse {
        id: completion.id.clone(),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        content,
        model: completion.model.clone(),
        stop_reason,
        stop_sequence: None,
        usage: ClaudeUsage {
            input_tokens: completion.usage.prompt_tokens,
            output_tokens: completion.usage.completion_tokens,
        },
    }
}

fn validate_claude_request(
    req: &ClaudeMessagesRequest,
    max_text_length: usize,
) -> Result<ChatCompletionRequest, String> {
    let chat_req = req.clone().into_chat_request()?;

    let params = ChatCompletionRequestParams {
        model: &chat_req.model,
        messages: &chat_req.messages,
        max_tokens: chat_req.max_tokens,
        temperature: chat_req.temperature,
        top_p: chat_req.top_p,
        frequency_penalty: chat_req.frequency_penalty,
        presence_penalty: chat_req.presence_penalty,
        n: chat_req.n,
    };

    Validator::validate_chat_completion_request(&params)
        .map_err(|e| format!("输入验证失败: {}", e))?;
    Validator::validate_messages_with_limit(&chat_req.messages, max_text_length)
        .map_err(|e| format!("输入验证失败: {}", e))?;

    Ok(chat_req)
}

async fn prepare_chat_request(
    headers: &HeaderMap,
    state: &Arc<ServiceContext>,
    request_context: Option<&AiRequestContext>,
    req: &ClaudeMessagesRequest,
) -> Result<
    (
        String,
        String,
        ChatCompletionRequest,
        crate::domain::dto::provider::ProviderConfig,
        ModelRoutingInfo,
        u32,
        crate::service::CalculatedFee,
    ),
    String,
> {
    validate_anthropic_headers(headers)?;

    let (user_id, api_key) =
        chat_controller::authenticate_user(headers, state, request_context).await?;

    let (provider_config, model_info) = state
        .model_router
        .route(&req.model, true)
        .await
        .map_err(|e| format!("Failed to route model: {}", e))?;

    let max_text_length = if let Some(max_tokens) = model_info.model_base.max_tokens_per_request {
        max_tokens as usize * 4
    } else {
        crate::domain::dto::validation::MAX_TEXT_LENGTH
    };

    let chat_req = validate_claude_request(req, max_text_length)?;

    let token_meta = chat_controller::calculate_tokens(&chat_req, &model_info)
        .await
        .map_err(|e| e.to_string())?;

    let rate_limit_result = state
        .rate_limit_service
        .precheck_request_tokens(&user_id, token_meta.input_tokens as i32)
        .await
        .map_err(|e| format!("Rate limit check failed: {}", e))?;

    match rate_limit_result {
        RateLimitCheckResult { allowed: true, .. } => {}
        RateLimitCheckResult {
            allowed: false,
            rpm_remaining,
            tpm_remaining,
            warning,
        } => {
            let message = if let Some(warning) = warning {
                format!(
                    "Rate limit exceeded: {}. RPM remaining: {}, TPM remaining: {}",
                    warning, rpm_remaining, tpm_remaining
                )
            } else {
                format!(
                    "Rate limit exceeded. RPM remaining: {}, TPM remaining: {}",
                    rpm_remaining, tpm_remaining
                )
            };
            return Err(message);
        }
    }

    let estimated_output_tokens = chat_req.max_tokens.unwrap_or(500) as i64;
    let fee = chat_controller::get_dynamic_pricing(
        &user_id,
        &chat_req.model,
        token_meta.input_tokens,
        estimated_output_tokens,
        &state.billing_service,
        &api_key,
        model_info.model_base.input_price,
        model_info.model_base.output_price,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok((
        user_id,
        api_key,
        chat_req,
        provider_config,
        model_info,
        token_meta.input_tokens as u32,
        fee,
    ))
}

async fn handle_non_streaming(
    state: Arc<ServiceContext>,
    user_id: String,
    _api_key: String,
    fee: crate::service::CalculatedFee,
    usage_log_id: String,
    chat_req: ChatCompletionRequest,
    provider_config: crate::domain::dto::provider::ProviderConfig,
    model_info: ModelRoutingInfo,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Response {
    let billing_service = state.billing_service.clone();
    let start_time = std::time::Instant::now();

    match chat_controller::call_provider_with_fallback(
        &state,
        &chat_req,
        &user_id,
        &provider_config,
        &model_info,
        &usage_log_id,
    )
    .await
    {
        Ok(completion) => {
            let upstream =
                crate::service::ai_hub::provider::upstream_trace::get_upstream_oauth(&usage_log_id);

            let input_tokens_i64 = fee.input_tokens;
            let output_tokens_i64 = i64::from(completion.usage.completion_tokens);

            let extra = {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "prompt_tokens".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(input_tokens_i64)),
                );
                obj.insert(
                    "completion_tokens".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(output_tokens_i64)),
                );
                obj.insert(
                    "total_tokens".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(
                        input_tokens_i64.saturating_add(output_tokens_i64),
                    )),
                );
                obj.insert(
                    "api".to_string(),
                    serde_json::Value::String("messages".to_string()),
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

            // tokens 聚合必须绑定 usage log 成功落库后再更新（避免落库失败但面板统计增长）。

            if let Err(e) = state
                .rate_limit_service
                .settle_output_tokens(&user_id, output_tokens_i64 as i32)
                .await
            {
                log::error!("[Claude Messages] Failed to consume output tokens: {}", e);
            }

            let duration_ms = start_time.elapsed().as_millis() as i64;
            let meta = crate::service::ai_hub::UsageLogMeta::messages(
                ip_address.clone(),
                user_agent.clone(),
            );

            let completed = billing_service
                .complete_usage_log(
                    &fee,
                    &usage_log_id,
                    &meta,
                    output_tokens_i64,
                    duration_ms,
                    extra,
                    upstream.clone(),
                )
                .await;

            if let Err(e) = completed {
                log::error!("[Claude Messages] Failed to complete usage log: {}", e);
            } else {
                crate::service::ai_hub::provider::upstream_trace::remove_upstream_oauth(
                    &usage_log_id,
                );
                if let Some(upstream) = &upstream {
                    crate::service::ai_hub::provider::oauth::record_provider_token_usage(
                        &upstream.provider_id,
                        &upstream.provider_type,
                        &upstream.account_key,
                        input_tokens_i64,
                        output_tokens_i64,
                    );
                }
            }

            apply_anthropic_headers(Json(completion_to_claude_message(&completion)).into_response())
        }
        Err(error) => {
            log::error!(
                "[Claude Messages] Non-stream provider call failed: {}",
                error
            );
            if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                log::error!(
                    "[Claude Messages] Failed to rollback pre-consumption: {}",
                    rollback_err
                );
            }
            create_claude_error_response(StatusCode::INTERNAL_SERVER_ERROR, "provider call failed")
        }
    }
}

async fn handle_streaming(
    state: Arc<ServiceContext>,
    user_id: String,
    _api_key: String,
    fee: crate::service::CalculatedFee,
    usage_log_id: String,
    chat_req: ChatCompletionRequest,
    provider_config: crate::domain::dto::provider::ProviderConfig,
    model_info: ModelRoutingInfo,
    input_tokens: u32,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Response {
    let message_id = Ulid::new().to_string();
    let model = chat_req.model.clone();
    let billing_service = state.billing_service.clone();
    let rate_limit_service = state.rate_limit_service.clone();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

    tokio::spawn(async move {
        let message_start = ClaudeMessageStartEvent {
            event_type: "message_start".to_string(),
            message: ClaudeMessageResponse {
                id: message_id.clone(),
                response_type: "message".to_string(),
                role: "assistant".to_string(),
                content: Vec::new(),
                model: model.clone(),
                stop_reason: None,
                stop_sequence: None,
                usage: ClaudeUsage {
                    input_tokens,
                    output_tokens: 0,
                },
            },
        };

        if let Ok(event) = Event::default()
            .event("message_start")
            .json_data(message_start)
        {
            let _ = tx.send(Ok(event)).await;
        }

        let start_time = std::time::Instant::now();

        let mut stream = match chat_controller::call_provider_stream_with_fallback(
            &state,
            &chat_req,
            &user_id,
            &provider_config,
            &model_info,
            &usage_log_id,
        )
        .await
        {
            Ok(stream) => stream,
            Err(error) => {
                log::error!(
                    "[Claude Messages] Provider stream startup failed: {}",
                    error
                );

                if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                    log::error!(
                        "[Claude Messages] Failed to rollback pre-consumption: {}",
                        rollback_err
                    );
                }

                let event = Event::default()
                    .event("error")
                    .json_data(ClaudeErrorEvent {
                        event_type: "error".to_string(),
                        error: ClaudeErrorBody {
                            error_type: "api_error".to_string(),
                            message: "Claude Messages request failed".to_string(),
                        },
                    })
                    .unwrap_or_else(|_| Event::default().event("error").data("Unknown error"));
                let _ = tx.send(Ok(event)).await;
                return;
            }
        };

        let mut output_tokens = 0u32;
        let mut final_stop_reason: Option<String> = None;
        let mut text_block_started = false;
        let mut emitted_tool_call_ids = HashSet::new();
        let mut tool_block_indexes = HashMap::new();
        let mut next_block_index = 1u32;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    process_stream_chunk(
                        &tx,
                        &chunk,
                        &mut output_tokens,
                        &mut final_stop_reason,
                        &mut text_block_started,
                        &mut emitted_tool_call_ids,
                        &mut tool_block_indexes,
                        &mut next_block_index,
                    )
                    .await;
                }
                Err(error) => {
                    log::error!("[Claude Messages] Provider stream chunk failed: {}", error);

                    if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await
                    {
                        log::error!(
                            "[Claude Messages] Failed to rollback pre-consumption: {}",
                            rollback_err
                        );
                    }

                    let event = Event::default()
                        .event("error")
                        .json_data(ClaudeErrorEvent {
                            event_type: "error".to_string(),
                            error: ClaudeErrorBody {
                                error_type: "api_error".to_string(),
                                message: "Claude Messages stream interrupted".to_string(),
                            },
                        })
                        .unwrap_or_else(|_| Event::default().event("error").data("Unknown error"));
                    let _ = tx.send(Ok(event)).await;
                    return;
                }
            }
        }

        if text_block_started {
            if let Ok(event) = Event::default().event("content_block_stop").json_data(
                ClaudeContentBlockStopEvent {
                    event_type: "content_block_stop".to_string(),
                    index: 0,
                },
            ) {
                let _ = tx.send(Ok(event)).await;
            }
        }

        if let Ok(event) =
            Event::default()
                .event("message_delta")
                .json_data(ClaudeMessageDeltaEvent {
                    event_type: "message_delta".to_string(),
                    delta: ClaudeMessageDelta {
                        stop_reason: final_stop_reason.or_else(|| Some("end_turn".to_string())),
                        stop_sequence: None,
                    },
                    usage: ClaudeUsage {
                        input_tokens,
                        output_tokens,
                    },
                })
        {
            let _ = tx.send(Ok(event)).await;
        }

        if let Ok(event) =
            Event::default()
                .event("message_stop")
                .json_data(ClaudeMessageStopEvent {
                    event_type: "message_stop".to_string(),
                })
        {
            let _ = tx.send(Ok(event)).await;
        }

        let upstream =
            crate::service::ai_hub::provider::upstream_trace::get_upstream_oauth(&usage_log_id);
        let input_tokens_i64 = fee.input_tokens;
        let output_tokens_i64 = i64::from(output_tokens);

        let extra = {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "prompt_tokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(input_tokens_i64)),
            );
            obj.insert(
                "completion_tokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(output_tokens_i64)),
            );
            obj.insert(
                "total_tokens".to_string(),
                serde_json::Value::Number(serde_json::Number::from(
                    input_tokens_i64.saturating_add(output_tokens_i64),
                )),
            );
            obj.insert(
                "api".to_string(),
                serde_json::Value::String("messages".to_string()),
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

        if let Err(e) = rate_limit_service
            .settle_output_tokens(&user_id, output_tokens_i64 as i32)
            .await
        {
            log::error!("[Claude Messages] Failed to consume output tokens: {}", e);
        }

        let duration_ms = start_time.elapsed().as_millis() as i64;
        let meta =
            crate::service::ai_hub::UsageLogMeta::messages(ip_address.clone(), user_agent.clone());

        let completed = billing_service
            .complete_usage_log(
                &fee,
                &usage_log_id,
                &meta,
                output_tokens_i64,
                duration_ms,
                extra,
                upstream.clone(),
            )
            .await;

        if let Err(e) = completed {
            log::error!("[Claude Messages] Failed to complete usage log: {}", e);
        } else {
            crate::service::ai_hub::provider::upstream_trace::remove_upstream_oauth(&usage_log_id);
            if let Some(upstream) = &upstream {
                crate::service::ai_hub::provider::oauth::record_provider_token_usage(
                    &upstream.provider_id,
                    &upstream.provider_type,
                    &upstream.account_key,
                    input_tokens_i64,
                    output_tokens_i64,
                );
            }
        }
    });

    let stream = ReceiverStream::new(rx)
        .map(|result| {
            result.unwrap_or_else(|_| Event::default().event("error").data("Stream error"))
        })
        .map(Ok::<Event, Infallible>);

    apply_anthropic_headers(Sse::new(Box::pin(stream)).into_response())
}

async fn process_stream_chunk(
    tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    chunk: &ChatCompletionChunk,
    output_tokens: &mut u32,
    final_stop_reason: &mut Option<String>,
    text_block_started: &mut bool,
    emitted_tool_call_ids: &mut HashSet<String>,
    tool_block_indexes: &mut HashMap<String, u32>,
    next_block_index: &mut u32,
) {
    if let Some(usage) = chunk.extra_fields.get("usage") {
        if let Some(completion_tokens) = usage
            .get("completion_tokens")
            .and_then(|value| value.as_u64())
        {
            *output_tokens = completion_tokens as u32;
        }
    }

    for choice in &chunk.choices {
        if let Some(delta) = &choice.delta {
            if let Some(content) = &delta.content {
                if !*text_block_started {
                    let block_start = ClaudeContentBlockStartEvent {
                        event_type: "content_block_start".to_string(),
                        index: 0,
                        content_block: ClaudeContentBlock {
                            block_type: "text".to_string(),
                            text: Some(String::new()),
                            id: None,
                            name: None,
                            input: None,
                        },
                    };

                    if let Ok(event) = Event::default()
                        .event("content_block_start")
                        .json_data(block_start)
                    {
                        let _ = tx.send(Ok(event)).await;
                    }
                    *text_block_started = true;
                }

                let event = Event::default().event("content_block_delta").json_data(
                    ClaudeContentBlockDeltaEvent {
                        event_type: "content_block_delta".to_string(),
                        index: 0,
                        delta: ClaudeContentDelta {
                            delta_type: "text_delta".to_string(),
                            text: Some(content.clone()),
                            partial_json: None,
                        },
                    },
                );
                if let Ok(event) = event {
                    let _ = tx.send(Ok(event)).await;
                }
            }

            if let Some(tool_calls) = &delta.tool_calls {
                for tool_call in tool_calls {
                    let tool_index = *tool_block_indexes
                        .entry(tool_call.id.clone())
                        .or_insert_with(|| {
                            let index = *next_block_index;
                            *next_block_index += 1;
                            index
                        });

                    if !emitted_tool_call_ids.insert(tool_call.id.clone()) {
                        continue;
                    }

                    let start_event = Event::default().event("content_block_start").json_data(
                        ClaudeContentBlockStartEvent {
                            event_type: "content_block_start".to_string(),
                            index: tool_index,
                            content_block: ClaudeContentBlock {
                                block_type: "tool_use".to_string(),
                                text: None,
                                id: Some(tool_call.id.clone()),
                                name: Some(tool_call.function.name.clone()),
                                input: Some(serde_json::json!({})),
                            },
                        },
                    );
                    if let Ok(event) = start_event {
                        let _ = tx.send(Ok(event)).await;
                    }

                    let delta_event = Event::default().event("content_block_delta").json_data(
                        ClaudeContentBlockDeltaEvent {
                            event_type: "content_block_delta".to_string(),
                            index: tool_index,
                            delta: ClaudeContentDelta {
                                delta_type: "input_json_delta".to_string(),
                                text: None,
                                partial_json: Some(tool_call.function.arguments.clone()),
                            },
                        },
                    );
                    if let Ok(event) = delta_event {
                        let _ = tx.send(Ok(event)).await;
                    }

                    let stop_event = Event::default().event("content_block_stop").json_data(
                        ClaudeContentBlockStopEvent {
                            event_type: "content_block_stop".to_string(),
                            index: tool_index,
                        },
                    );
                    if let Ok(event) = stop_event {
                        let _ = tx.send(Ok(event)).await;
                    }
                }
            }
        }

        if choice.finish_reason.is_some() {
            *final_stop_reason = map_stop_reason(choice.finish_reason.as_deref());
        }
    }
}

pub async fn create_messages(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    Json(req): Json<ClaudeMessagesRequest>,
) -> Response {
    let is_stream = req.stream.unwrap_or(false);
    let (ip_address, user_agent) = crate::controller::ai_hub::extract_client_meta(&headers);

    let prepared = match prepare_chat_request(
        &headers,
        &state,
        request_context.as_ref().map(|context| &context.0),
        &req,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(error) => {
            return if is_stream {
                create_claude_sse_error_response(error)
            } else if error.contains("authorization") || error.contains("API key") {
                create_claude_error_response(StatusCode::UNAUTHORIZED, error)
            } else if error.contains("Rate limit") {
                create_claude_error_response(StatusCode::TOO_MANY_REQUESTS, error)
            } else {
                create_claude_error_response(StatusCode::BAD_REQUEST, error)
            };
        }
    };

    let (user_id, api_key, chat_req, provider_config, model_info, input_tokens, fee) = prepared;

    let meta =
        crate::service::ai_hub::UsageLogMeta::messages(ip_address.clone(), user_agent.clone());

    let usage_log_id = match state
        .billing_service
        .pre_deduct_quota_and_log(
            &fee,
            &meta,
            Some(serde_json::json!({
                "model": chat_req.model,
                "stream": is_stream,
                "api": "messages",
                "request_path": "/api/v1/messages",
                "request_type": "messages",
            })),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            let error = format!("Failed to pre-deduct quota: {}", e);
            return if is_stream {
                create_claude_sse_error_response(error)
            } else {
                create_claude_error_response(StatusCode::PAYMENT_REQUIRED, error)
            };
        }
    };

    if is_stream {
        handle_streaming(
            state,
            user_id,
            api_key,
            fee,
            usage_log_id,
            chat_req,
            provider_config,
            model_info,
            input_tokens,
            ip_address,
            user_agent,
        )
        .await
    } else {
        handle_non_streaming(
            state,
            user_id,
            api_key,
            fee,
            usage_log_id,
            chat_req,
            provider_config,
            model_info,
            ip_address,
            user_agent,
        )
        .await
    }
}
