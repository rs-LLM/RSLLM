//! AI Hub Responses控制器模块
//!
//! 提供OpenAI兼容的Responses API接口

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use std::sync::Arc;
use ulid::Ulid;

// 导入相关类型
use crate::context::ServiceContext;
use crate::domain::dto::content::{
    ChatCompletionMessage, ChatMessageContent, ChatMessageContentPart,
};
use crate::domain::dto::responses::{ResponseCreateParams, ResponseInput};
use crate::domain::vo::ai_hub::responses::{Response, ResponseOutput, ResponseStatus};
use crate::domain::vo::response::ApiResponse;
use crate::domain::vo::usage::Usage;
use crate::service::BillingService;
use crate::service::CalculatedFee;
use crate::service::PriceRuleService;
use crate::service::{Content, TokenCountMeta, TokenCounter};

/// Responses API接口
///
/// 提供OpenAI兼容的Responses API功能，支持流式和非流式响应
#[utoipa::path(
    post,
    path = "/api/v1/responses",
    request_body = ResponseCreateParams,
    responses(
        (status = 200, description = "响应创建成功", body = ApiResponse<Response>),
        (status = 400, description = "参数错误", body = ApiResponse<Response>),
        (status = 401, description = "未授权", body = ApiResponse<Response>),
        (status = 500, description = "服务器错误", body = ApiResponse<Response>)
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
    Json(req): Json<ResponseCreateParams>,
) -> impl IntoResponse {
    let request_id = Ulid::new().to_string();
    let start_time = std::time::Instant::now();

    log::info!("[AI Hub] Response creation request: {}", request_id);

    // 1. 用户认证
    let (user_id, api_key) = match authenticate_user(&headers, &state).await {
        Ok((id, key)) => (id, key),
        Err(e) => return Json(ApiResponse::error("401", &e)),
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);

    // 2. 输入验证
    match validate_response_request(&req) {
        Ok(_) => log::info!("[AI Hub] Input validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Input validation failed: {}", e);
            return Json(ApiResponse::error("400", &format!("输入验证失败: {}", e)));
        }
    }

    // 3. Token计算
    let token_meta = match calculate_response_tokens(&req, &state.model_router).await {
        Ok(meta) => meta,
        Err(e) => return Json(ApiResponse::error("500", &e)),
    };
    log::info!(
        "[AI Hub] Token calculation: input={}, model={}",
        token_meta.input_tokens,
        req.model
    );

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
        Err(e) => return Json(ApiResponse::error("500", &e)),
    };

    log::info!(
        "[AI Hub] Pre-consumption check passed: cost={:.2}",
        fee.total_cost
    );

    // 5. 调用AI服务（真实实现）
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
                "stream": req.stream.unwrap_or(false),
            })),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            // 扣费失败，回滚预消费
            log::error!(
                "[AI Hub] Deduct quota failed, rolling back pre-consumption: {}",
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
    model_router: &crate::routers::model_router::ModelRouter,
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
                req.stream.unwrap_or(false),
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
                        req.stream.unwrap_or(false),
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
            req.stream.unwrap_or(false),
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
    _price_rule_service: &PriceRuleService,
    model_router: &crate::routers::model_router::ModelRouter,
    api_key: &str,
) -> std::result::Result<CalculatedFee, String> {
    // 从数据库获取模型定价
    let (base_input_price, base_output_price) = match model_router.route_to_model(model).await {
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
            )
        }
        Err(e) => {
            log::warn!(
                "[AI Hub] Failed to get model definition from database: {}, using default pricing",
                e
            );
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

/// 调用AI Provider获取真实响应
async fn call_provider(
    state: &Arc<ServiceContext>,
    req: &ResponseCreateParams,
    _user_id: &str,
) -> std::result::Result<Response, String> {
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

    // 2. 从ProviderRegistry获取Provider实例
    let provider_registry = state.provider_registry.read().await;

    let provider = provider_registry
        .get_provider(&provider_config.provider_code)
        .ok_or_else(|| {
            format!(
                "Provider not found for provider_code: {}",
                provider_config.provider_code
            )
        })?;

    // 3. 转换为ChatCompletionRequest（Responses API与Chat Completions API兼容）
    let chat_req = convert_to_chat_request(req);

    // 4. 调用Provider的chat_completions方法
    let response = provider
        .chat_completions(chat_req, &serde_json::json!({}))
        .await
        .map_err(|e| format!("Provider error: {}", e))?;

    // 5. 转换为Response类型
    convert_to_response(response, req)
}

/// 转换为ChatCompletionRequest
fn convert_to_chat_request(
    req: &ResponseCreateParams,
) -> crate::domain::dto::chat::ChatCompletionRequest {
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

    crate::domain::dto::chat::ChatCompletionRequest {
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
) -> std::result::Result<Response, String> {
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

            Ok(Response {
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
