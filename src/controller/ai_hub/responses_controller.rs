//! AI Hub Responses控制器模块
//!
//! 提供OpenAI兼容的Responses API接口

use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse, Response as HttpResponse,
        sse::{Event, Sse},
    },
};

use futures::StreamExt;
use std::{convert::Infallible, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;
use ulid::Ulid;

// 导入相关类型
use crate::context::ServiceContext;
use crate::controller::ai_hub::chat_controller;
use crate::domain::dto::chat::ChatCompletionRequest;
use crate::domain::dto::content::{
    ChatCompletionMessage, ChatMessageContent, ChatMessageContentPart,
};
use crate::domain::dto::responses::{ResponseCreateParams, ResponseInput};
use crate::domain::vo::ai_hub::responses::{
    OpenAIErrorResponse, Response as OpenAIResponse, ResponseOutput, ResponseStatus,
};
use crate::domain::vo::usage::Usage;

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn to_responses_sse_event(event_name: &'static str, payload: serde_json::Value) -> Event {
    // 说明：Responses SSE 的 data 应为 JSON 字符串（终止帧仍使用 data: [DONE]）。
    match serde_json::to_string(&payload) {
        Ok(json) => Event::default().event(event_name).data(json),
        Err(_) => Event::default().event("error").data(
            r#"{"type":"error","error":{"type":"server_error","message":"SSE serialization failed"}}"#,
        ),
    }
}

fn to_response_created_event(id: &str) -> Event {
    to_responses_sse_event(
        "response.created",
        serde_json::json!({
            "type": "response.created",
            "response": {
                "id": id,
                "object": "response",
                "created": now_unix_seconds(),
                "status": "in_progress"
            }
        }),
    )
}

fn to_response_output_text_delta_event(id: &str, delta: String) -> Option<Event> {
    if delta.is_empty() {
        return None;
    }

    Some(to_responses_sse_event(
        "response.output_text.delta",
        serde_json::json!({
            "type": "response.output_text.delta",
            "response_id": id,
            "delta": delta
        }),
    ))
}

fn to_response_completed_event(
    id: &str,
    output_text: String,
    input_tokens: i64,
    output_tokens: i64,
    finish_reason: Option<String>,
) -> Event {
    to_responses_sse_event(
        "response.completed",
        serde_json::json!({
            "type": "response.completed",
            "response": {
                "id": id,
                "object": "response",
                "created": now_unix_seconds(),
                "status": "completed",
                "output_text": if output_text.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(output_text) },
                "usage": {
                    "prompt_tokens": input_tokens,
                    "completion_tokens": output_tokens,
                    "total_tokens": input_tokens.saturating_add(output_tokens)
                },
            },
            "finish_reason": finish_reason
        }),
    )
}

fn to_response_error_event(id: &str, status: StatusCode, message: impl Into<String>) -> Event {
    to_responses_sse_event(
        "error",
        serde_json::json!({
            "type": "error",
            "response_id": id,
            "error": OpenAIErrorResponse::new(status.as_u16(), message).error
        }),
    )
}
use crate::service::BillingService;
use crate::service::CalculatedFee;
use crate::service::ai_hub::rate_limit_service::RateLimitCheckResult;
use crate::service::ai_hub::{AiRequestContext, resolve_request_identity};
use crate::service::{Content, TokenCountMeta, TokenCounter};

fn openai_error_response(status: StatusCode, message: impl Into<String>) -> HttpResponse {
    let mut response = Json(OpenAIErrorResponse::new(status.as_u16(), message)).into_response();
    *response.status_mut() = status;
    response
}

/// 创建 SSE 错误响应（OpenAI 风格：一个 data: {error...} 事件）
#[allow(dead_code)]
fn create_sse_error_response(status: StatusCode, message: impl Into<String>) -> HttpResponse {
    let error_event = Event::default()
        .json_data(OpenAIErrorResponse::new(status.as_u16(), message))
        .unwrap_or_else(|_| Event::default().data("Unknown error"));
    let stream = futures::stream::once(async move { Ok::<Event, Infallible>(error_event) });
    let mut response = Sse::new(Box::pin(stream)).into_response();
    *response.status_mut() = status;
    response
}

async fn handle_streaming_responses(
    state: Arc<ServiceContext>,
    user_id: String,
    _api_key: String,
    fee: CalculatedFee,
    usage_log_id: String,
    req: ResponseCreateParams,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> HttpResponse {
    let (provider_config, model_info) = match state.model_router.route(&req.model, true).await {
        Ok(v) => v,
        Err(e) => {
            log::error!("[AI Hub] Failed to route model (responses stream): {}", e);
            if let Err(rollback_err) = state.billing_service.rollback_pre_consumption(&fee).await {
                log::error!(
                    "[AI Hub] Failed to rollback pre-consumption (responses stream route fail): {}",
                    rollback_err
                );
            }
            let error = "AI 服务路由失败";

            let error_event =
                to_response_error_event(&usage_log_id, StatusCode::INTERNAL_SERVER_ERROR, error);
            let done_event = Event::default().data("[DONE]");

            let stream = futures::stream::iter([
                Ok::<Event, Infallible>(error_event),
                Ok::<Event, Infallible>(done_event),
            ]);

            let mut response = Sse::new(Box::pin(stream)).into_response();
            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            return response;
        }
    };

    let request_id = usage_log_id.clone();

    let billing_service = state.billing_service.clone();
    let rate_limit_service = state.rate_limit_service.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

    tokio::spawn(async move {
        let start_time = std::time::Instant::now();

        // 发送 response.created（严格 Responses SSE）
        if tx
            .send(Ok(to_response_created_event(&request_id)))
            .await
            .is_err()
        {
            log::info!("[AI Hub] Client disconnected before provider stream start (responses)");
            if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                log::error!(
                    "[AI Hub] Failed to rollback pre-consumption (responses client disconnect early): {}",
                    rollback_err
                );
            }
            return;
        }

        let chat_req = convert_to_chat_request(&req);

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
            Err(e) => {
                log::error!("[AI Hub] Provider stream startup failed (responses): {}", e);

                if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                    log::error!(
                        "[AI Hub] Failed to rollback pre-consumption (responses stream startup fail): {}",
                        rollback_err
                    );
                }

                let event = to_response_error_event(
                    &request_id,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Responses stream request failed",
                );
                let _ = tx.send(Ok(event)).await;

                let done_event = Event::default().data("[DONE]");
                let _ = tx.send(Ok(done_event)).await;
                return;
            }
        };

        let mut accumulated_content = String::new();
        let mut accumulated_reasoning_content = String::new();
        let mut provider_usage: Option<serde_json::Value> = None;
        let mut last_finish_reason: Option<String> = None;
        let mut client_disconnected = false;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(mut chunk) => {
                    chunk.model = req.model.clone();
                    chunk.id = request_id.clone();

                    if let Some(usage) = chunk.extra_fields.get("usage") {
                        provider_usage = Some(usage.clone());
                    }

                    if let Some(choice) = chunk.choices.first() {
                        if let Some(finish_reason) = &choice.finish_reason {
                            last_finish_reason = Some(finish_reason.clone());
                        }

                        if let Some(delta) = &choice.delta {
                            if let Some(reasoning) = &delta.reasoning_content {
                                accumulated_reasoning_content.push_str(reasoning);
                            }
                            if let Some(content) = &delta.content {
                                accumulated_content.push_str(content);

                                if let Some(event) = to_response_output_text_delta_event(
                                    &request_id,
                                    content.clone(),
                                ) {
                                    if tx.send(Ok(event)).await.is_err() {
                                        client_disconnected = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("[AI Hub] Provider stream chunk failed (responses): {}", e);

                    if !client_disconnected {
                        let event = to_response_error_event(
                            &request_id,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Responses stream interrupted",
                        );
                        let _ = tx.send(Ok(event)).await;

                        // 尝试向客户端发送终止标记，避免客户端挂起等待
                        let done_event = Event::default().data("[DONE]");
                        let _ = tx.send(Ok(done_event)).await;
                    }
                    break;
                }
            }

            if client_disconnected {
                break;
            }
        }

        // output_tokens：优先 provider usage，否则按累计内容估算
        let mut output_tokens_i64: i64 = 0;

        if let Some(usage) = provider_usage.clone() {
            output_tokens_i64 = usage
                .get("completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
        }

        if output_tokens_i64 == 0 {
            if !accumulated_content.is_empty() {
                if let Ok(meta) = TokenCounter::count_content_tokens(
                    &Content::Text(accumulated_content.clone()),
                    &req.model,
                    true,
                    Some(&model_info),
                ) {
                    output_tokens_i64 = output_tokens_i64.saturating_add(meta.output_tokens);
                }
            }

            if !accumulated_reasoning_content.is_empty() {
                if let Ok(meta) = TokenCounter::count_content_tokens(
                    &Content::Text(accumulated_reasoning_content),
                    &req.model,
                    true,
                    Some(&model_info),
                ) {
                    output_tokens_i64 = output_tokens_i64.saturating_add(meta.output_tokens);
                }
            }
        }

        if let Err(e) = rate_limit_service
            .settle_output_tokens(&user_id, output_tokens_i64 as i32)
            .await
        {
            log::error!(
                "[AI Hub] Failed to consume output tokens (responses stream): {}",
                e
            );
        }

        // 发送 response.completed（客户端断连时跳过发送，但仍然结算）
        if !client_disconnected {
            let _ = tx
                .send(Ok(to_response_completed_event(
                    &request_id,
                    accumulated_content.clone(),
                    fee.input_tokens,
                    output_tokens_i64,
                    last_finish_reason.clone().or(Some("stop".to_string())),
                )))
                .await;

            let done_event = Event::default().data("[DONE]");
            let _ = tx.send(Ok(done_event)).await;
        }

        let duration_ms = start_time.elapsed().as_millis() as i64;

        let upstream =
            crate::service::ai_hub::provider::upstream_trace::get_upstream_oauth(&usage_log_id);

        let extra = {
            let mut map = serde_json::Map::new();
            map.insert(
                "model".to_string(),
                serde_json::Value::String(req.model.clone()),
            );
            map.insert("stream".to_string(), serde_json::Value::Bool(true));
            map.insert(
                "api".to_string(),
                serde_json::Value::String("responses".to_string()),
            );
            map.insert(
                "request_path".to_string(),
                serde_json::Value::String("/api/v1/responses".to_string()),
            );
            map.insert(
                "request_type".to_string(),
                serde_json::Value::String("responses".to_string()),
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
            if let Some(usage) = provider_usage.clone() {
                map.insert("provider_usage".to_string(), usage);
            }
            Some(serde_json::Value::Object(map))
        };

        let meta =
            crate::service::ai_hub::UsageLogMeta::responses(ip_address.clone(), user_agent.clone());

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
            log::error!(
                "[AI Hub] Failed to complete usage log (responses stream): {}",
                e
            );
        } else {
            crate::service::ai_hub::provider::upstream_trace::remove_upstream_oauth(&usage_log_id);
            if let Some(upstream) = &upstream {
                crate::service::ai_hub::provider::oauth::record_provider_token_usage(
                    &upstream.provider_id,
                    &upstream.provider_type,
                    &upstream.account_key,
                    fee.input_tokens,
                    output_tokens_i64,
                );
            }
        }
    });

    let stream = ReceiverStream::new(rx)
        .map(|result| result.unwrap_or_else(|_| Event::default().data("Stream error")))
        .map(Ok::<Event, Infallible>);

    Sse::new(Box::pin(stream)).into_response()
}

/// Responses API接口
///
/// 提供OpenAI兼容的Responses API功能，支持流式和非流式响应
#[utoipa::path(
    post,
    path = "/api/v1/responses",
    request_body = ResponseCreateParams,
    responses(
        (status = 200, description = "响应创建成功", body = OpenAIResponse),
        (status = 400, description = "参数错误", body = OpenAIErrorResponse),
        (status = 401, description = "未授权", body = OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = OpenAIErrorResponse)
    ),
    tag = "responses",
    security(
        ("api_key" = [])
    )
)]
#[axum::debug_handler]
pub async fn create_response(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    Json(req): Json<ResponseCreateParams>,
) -> HttpResponse {
    let (ip_address, user_agent) = crate::controller::ai_hub::extract_client_meta(&headers);

    // 先生成 trace_key（后续 pre_deduct 会用它作为 usage_log_id）
    let trace_key = Ulid::new().to_string();
    log::info!("[AI Hub] Response creation request: {}", trace_key);

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
    match validate_response_request(&req) {
        Ok(_) => log::info!("[AI Hub] Input validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Input validation failed: {}", e);
            return openai_error_response(StatusCode::BAD_REQUEST, "输入参数不合法");
        }
    }

    // 3. Token计算
    let token_meta = match calculate_response_tokens(&req, &state.model_router).await {
        Ok(meta) => meta,
        Err(e) => return openai_error_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    log::info!(
        "[AI Hub] Token calculation: input={}, model={}",
        token_meta.input_tokens,
        req.model
    );

    // 4. 获取动态定价并预消费和配额检查
    let billing_service = &state.billing_service;

    // 4.5. 检查速率限制（使用实际的输入 token 数量）
    let input_tokens_i32 = token_meta.input_tokens as i32;
    let rate_limit_result = state
        .rate_limit_service
        .precheck_request_tokens(&user_id, input_tokens_i32)
        .await;
    match rate_limit_result {
        Ok(RateLimitCheckResult { allowed: true, .. }) => {}
        Ok(RateLimitCheckResult {
            allowed: false,
            warning,
            rpm_remaining,
            tpm_remaining,
        }) => {
            let message = if let Some(w) = warning {
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
            return openai_error_response(StatusCode::TOO_MANY_REQUESTS, message);
        }
        Err(e) => {
            log::error!("[AI Hub] Rate limit check failed: {}", e);
            return openai_error_response(StatusCode::INTERNAL_SERVER_ERROR, "限流服务异常");
        }
    }

    // 预估输出token（基于max_tokens或默认值）
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;

    // 获取动态定价
    let fee = match get_dynamic_pricing(
        &user_id,
        &req.model,
        token_meta.input_tokens,
        estimated_output_tokens,
        billing_service,
        &state.model_router,
        &api_key,
    )
    .await
    {
        Ok(fee) => fee,
        Err(e) => return openai_error_response(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    log::info!(
        "[AI Hub] Pre-consumption check passed: cost={:.2}",
        fee.total_cost
    );

    // 预扣减余额（只扣输入费用）并创建 pending usage log，trace_key 统一使用 trace_key
    let meta =
        crate::service::ai_hub::UsageLogMeta::responses(ip_address.clone(), user_agent.clone());
    let usage_log_id = match billing_service
        .pre_deduct_quota_and_log(
            &fee,
            &meta,
            Some(serde_json::json!({
                "model": req.model.clone(),
                "stream": req.stream.unwrap_or(false),
                "api": "responses",
                "request_path": "/api/v1/responses",
                "request_type": "responses",
                "trace_key": trace_key,
            })),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!(
                "[AI Hub] Failed to pre-deduct quota (responses): trace_key={}, user_id={}, error={}",
                trace_key,
                user_id,
                e
            );
            return openai_error_response(StatusCode::PAYMENT_REQUIRED, "计费检查失败");
        }
    };

    // 覆盖 usage_log_id 为预扣阶段生成的 log_id；trace_key 统一使用它
    let trace_key = usage_log_id.clone();
    let _ = trace_key;

    // 兼容：保留 request_id 变量供历史日志/调试使用，但 trace_key 统一使用 usage_log_id。
    let request_id = usage_log_id.clone();
    let _ = request_id;

    let start_time = std::time::Instant::now();

    let is_stream = req.stream.unwrap_or(false);

    if is_stream {
        return handle_streaming_responses(
            state,
            user_id,
            api_key,
            fee,
            usage_log_id,
            req,
            ip_address,
            user_agent,
        )
        .await;
    }

    // 5. 调用AI服务（非流式）
    let response =
        match call_provider_with_fallback_non_stream(&state, &req, &user_id, &usage_log_id).await {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("[AI Hub] Provider call failed: {}", e);
                if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                    log::error!(
                        "[AI Hub] Failed to rollback pre-consumption: {}",
                        rollback_err
                    );
                }
                return openai_error_response(StatusCode::INTERNAL_SERVER_ERROR, "AI 服务调用失败");
            }
        };

    // 6. 完成 usage log 并扣减输出费用
    let duration_ms = start_time.elapsed().as_millis() as i64;

    // 消耗输出 token 到速率限制（每次请求都消耗 1 个 RPM 和 output_tokens 个 TPM）
    if let Err(e) = state
        .rate_limit_service
        .settle_output_tokens(&user_id, response.usage.completion_tokens as i32)
        .await
    {
        log::error!("[AI Hub] Failed to consume output tokens: {}", e);
    }

    let upstream =
        crate::service::ai_hub::provider::upstream_trace::get_upstream_oauth(&usage_log_id);

    let extra = {
        let mut map = serde_json::Map::new();
        map.insert(
            "model".to_string(),
            serde_json::Value::String(req.model.clone()),
        );
        map.insert("stream".to_string(), serde_json::Value::Bool(false));
        map.insert(
            "api".to_string(),
            serde_json::Value::String("responses".to_string()),
        );
        map.insert(
            "request_path".to_string(),
            serde_json::Value::String("/api/v1/responses".to_string()),
        );
        map.insert(
            "request_type".to_string(),
            serde_json::Value::String("responses".to_string()),
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

    let completed = billing_service
        .complete_usage_log(
            &fee,
            &usage_log_id,
            &meta,
            response.usage.completion_tokens as i64,
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
            let input_tokens = i64::from(response.usage.prompt_tokens);
            let output_tokens = i64::from(response.usage.completion_tokens);
            crate::service::ai_hub::provider::oauth::record_provider_token_usage(
                &upstream.provider_id,
                &upstream.provider_type,
                &upstream.account_key,
                input_tokens,
                output_tokens,
            );
        }
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

/// 验证Responses API请求
fn validate_response_request(req: &ResponseCreateParams) -> std::result::Result<(), String> {
    // 验证推理配置
    if let Some(reasoning) = &req.reasoning {
        reasoning
            .validate()
            .map_err(|e| format!("Invalid reasoning config: {}", e))?;
    }

    // 验证模型名称
    if req.model.is_empty() {
        return Err("Model name cannot be empty".to_string());
    }

    // 验证输入内容
    match &req.input {
        ResponseInput::Text(text) => {
            if text.is_empty() {
                return Err("Input text cannot be empty".to_string());
            }
        }
        ResponseInput::Messages(messages) => {
            if messages.is_empty() {
                return Err("Input messages cannot be empty".to_string());
            }
        }
        ResponseInput::MultiModal(_) => {
            // 多模态输入验证
        }
    }

    // 验证温度参数
    if let Some(temp) = req.temperature
        && !(0.0..=2.0).contains(&temp)
    {
        return Err("Temperature must be between 0.0 and 2.0".to_string());
    }

    if let Some(top_p) = req.top_p
        && !(0.0..=1.0).contains(&top_p)
    {
        return Err("Top_p must be between 0.0 and 1.0".to_string());
    }

    if let Some(presence_penalty) = req.presence_penalty
        && !(-2.0..=2.0).contains(&presence_penalty)
    {
        return Err("Presence penalty must be between -2.0 and 2.0".to_string());
    }

    // 验证频率惩罚参数
    if let Some(frequency_penalty) = req.frequency_penalty
        && !(-2.0..=2.0).contains(&frequency_penalty)
    {
        return Err("Frequency penalty must be between -2.0 and 2.0".to_string());
    }

    Ok(())
}

/// 计算Responses API的token数量
async fn calculate_response_tokens(
    req: &ResponseCreateParams,
    model_router: &crate::router::model_router::ModelRouter,
) -> std::result::Result<TokenCountMeta, String> {
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

    // 计算输入中的token
    match &req.input {
        ResponseInput::Text(text) => {
            let meta = TokenCounter::count_content_tokens(
                &Content::Text(text.clone()),
                &req.model,
                false,
                Some(&model_def),
            )
            .map_err(|e| e.to_string())?;

            total_meta.input_tokens += meta.input_tokens;
            total_meta.output_tokens += meta.output_tokens;
        }
        ResponseInput::Messages(messages) => {
            for message in messages {
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
        }
        ResponseInput::MultiModal(_) => {
            // 多模态输入token计算（简化处理）
            total_meta.input_tokens += 100;
        }
    }

    // 计算instructions中的token
    if let Some(instructions) = &req.instructions {
        let meta = TokenCounter::count_content_tokens(
            &Content::Text(instructions.clone()),
            &req.model,
            false,
            Some(&model_def),
        )
        .map_err(|e| e.to_string())?;

        total_meta.input_tokens += meta.input_tokens;
        total_meta.output_tokens += meta.output_tokens;
    }

    Ok(total_meta)
}

/// 获取动态定价
async fn get_dynamic_pricing(
    user_id: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    billing_service: &BillingService,
    model_router: &crate::router::model_router::ModelRouter,
    api_key: &str,
) -> std::result::Result<CalculatedFee, String> {
    // 从数据库获取模型定价
    let (base_input_price, base_output_price, _price_unit, _patch_multiplier) = match model_router
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
                model_info.model_base.price_unit.clone(),
                model_info.model_base.patch_multiplier,
            )
        }
        Err(e) => {
            log::warn!(
                "[AI Hub] Failed to get model definition from database: {}, using default pricing",
                e
            );
            let (input, output) = if model.contains("gpt-4") {
                (0.03, 0.06)
            } else if model.contains("gpt-3.5") {
                (0.0015, 0.002)
            } else if model.contains("claude") {
                (0.01, 0.03)
            } else {
                (0.01, 0.02)
            };
            (input, output, None, None)
        }
    };

    // billing_service.calculate_and_check 内部会调用 price_rule_service
    let fee = billing_service
        .calculate_and_check(&crate::service::ai_hub::CalculateAndCheckParams {
            user_id,
            model_id: model,
            api_key,
            input_tokens,
            output_tokens,
            base_input_price,
            base_output_price,
            request_type: "responses",
        })
        .await
        .map_err(|e| format!("Billing calculation failed: {}", e))?;

    Ok(fee)
}

async fn call_provider_with_fallback_non_stream(
    state: &Arc<ServiceContext>,
    req: &ResponseCreateParams,
    user_id: &str,
    usage_log_id: &str,
) -> std::result::Result<OpenAIResponse, String> {
    let (provider_config, model_info) = state
        .model_router
        .route(&req.model, true)
        .await
        .map_err(|e| format!("Failed to route model: {}", e))?;

    let chat_req = convert_to_chat_request(req);

    let completion = chat_controller::call_provider_with_fallback(
        state,
        &chat_req,
        user_id,
        &provider_config,
        &model_info,
        usage_log_id,
    )
    .await?;

    convert_completion_to_response(completion, req)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn responses_stream_sse_emits_chunk_and_done() {
        let request_id = "resp_1";

        let created = to_response_created_event(request_id);
        let delta = to_response_output_text_delta_event(request_id, "hi".to_string()).unwrap();
        let completed = to_response_completed_event(
            request_id,
            "hi".to_string(),
            1,
            2,
            Some("stop".to_string()),
        );

        let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(16);
        tx.send(Ok(created)).await.unwrap();
        tx.send(Ok(delta)).await.unwrap();
        tx.send(Ok(completed)).await.unwrap();
        tx.send(Ok(Event::default().data("[DONE]"))).await.unwrap();
        drop(tx);

        let stream = ReceiverStream::new(rx)
            .map(|result| result.unwrap())
            .map(Ok::<Event, Infallible>);

        let response = Sse::new(Box::pin(stream)).into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let sse = String::from_utf8(body.to_vec()).unwrap();

        assert!(sse.contains("event: response.created"));
        assert!(sse.contains("\"type\":\"response.created\""));
        assert!(sse.contains("event: response.output_text.delta"));
        assert!(sse.contains("\"type\":\"response.output_text.delta\""));
        assert!(sse.contains("\"delta\":\"hi\""));
        assert!(sse.contains("event: response.completed"));
        assert!(sse.contains("\"type\":\"response.completed\""));
        assert!(sse.contains("\"status\":\"completed\""));
        assert!(sse.contains("data: [DONE]"));
    }
}

fn convert_completion_to_response(
    completion: crate::domain::vo::chat::ChatCompletion,
    req: &ResponseCreateParams,
) -> std::result::Result<OpenAIResponse, String> {
    // 提取输出文本
    let output_text = completion
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_ref())
        .and_then(|content| match content {
            crate::domain::dto::content::ChatMessageContent::String(s) => Some(s.clone()),
            crate::domain::dto::content::ChatMessageContent::Array(parts) => Some(
                parts
                    .iter()
                    .filter_map(|p| p.text.as_ref())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        });

    let output = if let Some(first_choice) = completion.choices.first() {
        Some(vec![ResponseOutput::Message(ChatCompletionMessage {
            role: first_choice.message.role.clone(),
            content: first_choice
                .message
                .content
                .as_ref()
                .map(|content| match content {
                    crate::domain::dto::content::ChatMessageContent::String(s) => {
                        ChatMessageContent::String(s.clone())
                    }
                    crate::domain::dto::content::ChatMessageContent::Array(parts) => {
                        ChatMessageContent::Array(
                            parts
                                .iter()
                                .map(|part| ChatMessageContentPart {
                                    r#type: part.r#type.clone(),
                                    text: part.text.clone(),
                                    image_url: part.image_url.clone(),
                                })
                                .collect(),
                        )
                    }
                }),
            name: first_choice.message.name.clone(),
            tool_calls: first_choice.message.tool_calls.clone(),
            tool_call_id: first_choice.message.tool_call_id.clone(),
            refusal: first_choice.message.refusal.clone(),
            reasoning_content: first_choice.message.reasoning_content.clone(),
            extra_fields: first_choice.message.extra_fields.clone(),
        })])
    } else {
        None
    };

    let input = match &req.input {
        ResponseInput::Text(text) => Some(vec![serde_json::json!({"type": "text", "text": text})]),
        ResponseInput::Messages(messages) => Some(
            messages
                .iter()
                .map(|msg| {
                    serde_json::json!({
                        "role": msg.role,
                        "content": msg.content
                    })
                })
                .collect(),
        ),
        ResponseInput::MultiModal(items) => Some(items.clone()),
    };

    Ok(OpenAIResponse {
        id: completion.id,
        object: "response".to_string(),
        created: completion.created.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        }),
        status: ResponseStatus::Completed,
        error: None,
        output,
        output_text,
        output_parsed: None,
        usage: Usage {
            prompt_tokens: completion.usage.prompt_tokens,
            completion_tokens: completion.usage.completion_tokens,
            total_tokens: completion.usage.total_tokens,
        },
        reasoning: None,
        instructions: req.instructions.clone(),
        input,
        parallel_tool_calls: req.parallel_tool_calls,
        tool: None,
        previous_response_id: req.previous_response_id.clone(),
        metadata: None,
    })
}

/// 调用AI Provider获取真实响应
#[allow(dead_code)]
async fn call_provider(
    state: &Arc<ServiceContext>,
    req: &ResponseCreateParams,
    _user_id: &str,
    trace_key: &str,
) -> std::result::Result<OpenAIResponse, String> {
    // 旧实现保留以避免大范围改动；主路径已切换到 call_provider_with_fallback_non_stream

    let model_router = &state.model_router;

    let (provider_config, _model_info) = model_router
        .route(&req.model, true)
        .await
        .map_err(|e| format!("Failed to route model: {}", e))?;

    let provider_registry = state.provider_registry.read().await;

    let provider = provider_registry
        .get_provider(&provider_config.provider_code)
        .ok_or_else(|| {
            format!(
                "Provider not found for provider_code: {}",
                provider_config.provider_code
            )
        })?;

    let chat_req = convert_to_chat_request(req);

    let model_config = serde_json::json!({
        "rsllm": {
            "trace_key": trace_key,
            "provider_id": provider_config.id.as_str(),
            "provider_type": provider_config.provider_type.to_string(),
        }
    });

    let response = provider
        .chat_completions(chat_req, &model_config)
        .await
        .map_err(|e| format!("Provider error: {}", e))?;

    convert_to_response(response, req)
}

/// 转换为ChatCompletionRequest
fn convert_to_chat_request(req: &ResponseCreateParams) -> ChatCompletionRequest {
    use crate::domain::dto::content as content_dto;

    // 直接使用tools，因为类型已经相同
    let tools = req.tools.clone();

    // 直接使用tool_choice，因为类型已经相同
    let tool_choice = req.tool_choice.clone();

    // 直接使用reasoning，因为类型已经相同
    let reasoning = req.reasoning.clone();

    // 构建消息列表
    let mut messages = Vec::new();

    // 添加instructions作为系统消息
    if let Some(instructions) = &req.instructions {
        messages.push(ChatCompletionMessage {
            role: "system".to_string(),
            content: Some(content_dto::ChatMessageContent::String(
                instructions.clone(),
            )),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
            reasoning_content: None,
            extra_fields: serde_json::Value::default(),
        });
    }

    // 添加conversation历史（如果存在）
    if let Some(conversation) = &req.conversation {
        messages.extend(conversation.iter().map(|msg| ChatCompletionMessage {
            role: msg.role.clone(),
            content: msg.content.as_ref().map(|content| {
                match content {
                    ChatMessageContent::String(s) => {
                        crate::domain::dto::content::ChatMessageContent::String(s.clone())
                    }
                    ChatMessageContent::Array(parts) => {
                        crate::domain::dto::content::ChatMessageContent::Array(
                            parts
                                .iter()
                                .map(|part| crate::domain::dto::content::ChatMessageContentPart {
                                    r#type: part.r#type.clone(),
                                    text: part.text.clone(),
                                    image_url: part.image_url.clone(),
                                })
                                .collect(),
                        )
                    }
                }
            }),
            name: msg.name.clone(),
            tool_calls: msg.tool_calls.clone(),
            tool_call_id: msg.tool_call_id.clone(),
            refusal: msg.refusal.clone(),
            reasoning_content: None,
            extra_fields: serde_json::Value::default(),
        }));
    }

    // 添加input作为用户消息
    match &req.input {
        ResponseInput::Text(text) => {
            messages.push(ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(crate::domain::dto::content::ChatMessageContent::String(
                    text.clone(),
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                reasoning_content: None,
                extra_fields: serde_json::Value::default(),
            });
        }
        ResponseInput::Messages(input_messages) => {
            messages.extend(input_messages.iter().map(|msg| ChatCompletionMessage {
                role: msg.role.clone(),
                content: msg.content.as_ref().map(|content| {
                    match content {
                        ChatMessageContent::String(s) => {
                            crate::domain::dto::content::ChatMessageContent::String(s.clone())
                        }
                        ChatMessageContent::Array(parts) => {
                            crate::domain::dto::content::ChatMessageContent::Array(
                                parts
                                    .iter()
                                    .map(|part| {
                                        crate::domain::dto::content::ChatMessageContentPart {
                                            r#type: part.r#type.clone(),
                                            text: part.text.clone(),
                                            image_url: part.image_url.clone(),
                                        }
                                    })
                                    .collect(),
                            )
                        }
                    }
                }),
                name: msg.name.clone(),
                tool_calls: msg.tool_calls.clone(),
                tool_call_id: msg.tool_call_id.clone(),
                refusal: msg.refusal.clone(),
                reasoning_content: None,
                extra_fields: serde_json::Value::default(),
            }));
        }
        ResponseInput::MultiModal(_) => {
            // 多模态输入处理（简化）
        }
    }

    ChatCompletionRequest {
        model: req.model.clone(),
        messages,
        stream: req.stream,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        top_p: req.top_p,
        frequency_penalty: req.frequency_penalty,
        presence_penalty: req.presence_penalty,
        stop: req.stop.clone(),
        response_format: req.response_format.clone(),
        tools,
        tool_choice,
        user: req.user.clone(),
        logprobs: None,
        top_logprobs: None,
        n: None,
        max_completion_tokens: req.max_completion_tokens,
        parallel_tool_calls: req.parallel_tool_calls,
        logit_bias: req.logit_bias.clone(),
        reasoning,
        extra_body: None,
        extra_fields: serde_json::Value::default(),
    }
}

/// 转换为Response类型
fn convert_to_response(
    response: crate::domain::dto::chat::ChatCompletionResponse,
    req: &ResponseCreateParams,
) -> std::result::Result<OpenAIResponse, String> {
    match response {
        crate::domain::dto::chat::ChatCompletionResponse::NonStream(completion) => {
            // 提取输出文本
            let output_text = completion
                .choices
                .first()
                .and_then(|choice| choice.message.content.as_ref())
                .and_then(|content| match content {
                    crate::domain::dto::content::ChatMessageContent::String(s) => Some(s.clone()),
                    crate::domain::dto::content::ChatMessageContent::Array(parts) => parts
                        .iter()
                        .filter_map(|p| p.text.as_ref())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n")
                        .into(),
                });

            // 构建输出内容
            let output =
                if let Some(first_choice) = completion.choices.first() {
                    Some(vec![ResponseOutput::Message(ChatCompletionMessage {
                        role: first_choice.message.role.clone(),
                        content: first_choice.message.content.as_ref().map(
                            |content| match content {
                                crate::domain::dto::content::ChatMessageContent::String(s) => {
                                    ChatMessageContent::String(s.clone())
                                }
                                crate::domain::dto::content::ChatMessageContent::Array(parts) => {
                                    ChatMessageContent::Array(
                                        parts
                                            .iter()
                                            .map(|part| ChatMessageContentPart {
                                                r#type: part.r#type.clone(),
                                                text: part.text.clone(),
                                                image_url: part.image_url.clone(),
                                            })
                                            .collect(),
                                    )
                                }
                            },
                        ),
                        name: first_choice.message.name.clone(),
                        tool_calls: first_choice.message.tool_calls.clone(),
                        tool_call_id: first_choice.message.tool_call_id.clone(),
                        refusal: first_choice.message.refusal.clone(),
                        reasoning_content: first_choice.message.reasoning_content.clone(),
                        extra_fields: first_choice.message.extra_fields.clone(),
                    })])
                } else {
                    None
                };

            // 构建输入内容（用于响应中返回）
            let input = match &req.input {
                ResponseInput::Text(text) => {
                    Some(vec![serde_json::json!({"type": "text", "text": text})])
                }
                ResponseInput::Messages(messages) => Some(
                    messages
                        .iter()
                        .map(|msg| {
                            serde_json::json!({
                                "role": msg.role,
                                "content": msg.content
                            })
                        })
                        .collect(),
                ),
                ResponseInput::MultiModal(items) => Some(items.clone()),
            };

            Ok(OpenAIResponse {
                id: completion.id,
                object: "response".to_string(),
                created: completion.created.unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                }),
                status: ResponseStatus::Completed,
                error: None,
                output,
                output_text,
                output_parsed: None,
                usage: Usage {
                    prompt_tokens: completion.usage.prompt_tokens,
                    completion_tokens: completion.usage.completion_tokens,
                    total_tokens: completion.usage.total_tokens,
                },
                reasoning: None,
                instructions: req.instructions.clone(),
                input,
                parallel_tool_calls: req.parallel_tool_calls,
                tool: None,
                previous_response_id: req.previous_response_id.clone(),
                metadata: None,
            })
        }
        crate::domain::dto::chat::ChatCompletionResponse::Stream(_) => {
            Err("Stream response not supported in non-stream mode".to_string())
        }
    }
}
