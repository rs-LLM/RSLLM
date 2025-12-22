//! AI Hub 聊天控制器模块
//!
//! 提供OpenAI兼容的聊天补全API接口

use axum::{
    extract::State,
    http::HeaderMap,
    Json,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

// 导入相关类型
use crate::context::ServiceContext;
use crate::domain::dto::chat::ChatCompletionRequest;
use crate::domain::vo::RespVO;
use crate::service::{TokenCounter, Content, TokenCountMeta};
use crate::domain::dto::content::{ChatMessageContent, ChatMessageContentPart, ChatCompletionMessage};
use crate::domain::vo::chat::{ChatCompletion, ChatCompletionChoice};
use crate::domain::vo::usage::Usage;
use crate::service::BillingService;
use crate::service::PriceRuleService;
use crate::service::CalculatedFee;
use crate::domain::dto::validation::Validator;

/// 聊天补全接口
///
/// 提供OpenAI兼容的聊天补全功能，支持流式和非流式响应
#[axum::debug_handler]
pub async fn chat_completions(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    log::info!("[AI Hub] Chat completion request: {}", request_id);
    
    // 1. 用户认证
    let user_id = match authenticate_user(&headers, &state) {
        Ok(id) => id,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);
    
    // 2. 输入验证
    match Validator::validate_chat_completion_request(
        &req.model,
        &req.messages,
        req.max_tokens,
        req.temperature,
        req.top_p,
        req.frequency_penalty,
        req.presence_penalty,
        req.n,
    ) {
        Ok(_) => log::info!("[AI Hub] Input validation passed"),
        Err(e) => {
            log::warn!("[AI Hub] Input validation failed: {}", e);
            return RespVO::from_error(format!("输入验证失败: {}", e));
        }
    }
    
    // 3. Token计算
    let token_meta = match calculate_tokens(&req) {
        Ok(meta) => meta,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    log::info!("[AI Hub] Token calculation: input={}, model={}",
        token_meta.input_tokens, req.model);
    
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
    ).await {
        Ok(fee) => fee,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    
    log::info!("[AI Hub] Pre-consumption check passed: cost={:.2}", fee.total_cost);
    
    // 5. 调用AI服务（真实实现）
    let response = match call_provider(&state, &req, &user_id).await {
        Ok(resp) => resp,
        Err(e) => {
            // 调用失败，回滚预消费
            log::error!("[AI Hub] Provider call failed, rolling back pre-consumption: {}", e);
            if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                log::error!("[AI Hub] Failed to rollback pre-consumption: {}", rollback_err);
            }
            return RespVO::from_error(format!("AI service call failed: {}", e));
        }
    };
    
    // 6. 实际扣费和记录用量
    let duration_ms = start_time.elapsed().as_millis() as i64;
    let usage_log_id = match billing_service.deduct_quota_and_log(
        &fee,
        &request_id,
        duration_ms,
        "success",
        Some(serde_json::json!({
            "model": req.model,
            "stream": req.stream.unwrap_or(false),
        })),
    ).await {
        Ok(id) => id,
        Err(e) => {
            // 扣费失败，回滚预消费
            log::error!("[AI Hub] Deduct quota failed, rolling back pre-consumption: {}", e);
            if let Err(rollback_err) = billing_service.rollback_pre_consumption(&fee).await {
                log::error!("[AI Hub] Failed to rollback pre-consumption: {}", rollback_err);
            }
            return RespVO::from_error(format!("Failed to deduct quota and log: {}", e));
        }
    };
    
    log::info!("[AI Hub] Usage logged: {}", usage_log_id);
    
    // 7. 返回响应
    RespVO::from(response)
}

/// 用户认证
fn authenticate_user(headers: &HeaderMap, _state: &Arc<ServiceContext>) -> std::result::Result<String, String> {
    // 从header获取token
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| "Missing or invalid authorization header".to_string())?;
    
    // 验证token并获取用户ID
    let jwt_token = crate::middleware::auth::checked_token(token)
        .map_err(|e| e.to_string())?;
    Ok(jwt_token.id)
}

/// 计算token数量
fn calculate_tokens(req: &ChatCompletionRequest) -> std::result::Result<TokenCountMeta, String> {
    let mut total_meta = TokenCountMeta::default();
    
    // 计算消息中的token
    for message in &req.messages {
        if let Some(content) = &message.content {
            // 将ChatMessageContent转换为Content枚举
            let content_enum = match content {
                ChatMessageContent::String(s) => {
                    Content::Text(s.clone())
                }
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

/// 获取动态定价
async fn get_dynamic_pricing(
    user_id: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    billing_service: &BillingService,
    _price_rule_service: &PriceRuleService, // 暂时不需要，billing_service内部已处理
) -> std::result::Result<CalculatedFee, String> {
    // 根据模型名称确定基础价格（分/1000 tokens）
    // 这些基础价格将作为PriceRuleService的输入，应用动态规则
    let (base_input_price, base_output_price) = if model.contains("gpt-4") {
        (0.03, 0.06)
    } else if model.contains("gpt-3.5") {
        (0.0015, 0.002)
    } else if model.contains("claude") {
        (0.01, 0.03)
    } else {
        (0.01, 0.02)
    };
    
    // billing_service.calculate_and_check 内部会调用 price_rule_service
    // 应用动态定价规则，所以我们直接调用它
    let fee = billing_service.calculate_and_check(
        user_id,
        model,
        "unknown", // provider_id将在实际调用时确定
        input_tokens,
        output_tokens,
        base_input_price,
        base_output_price,
        "chat",
    ).await.map_err(|e| format!("Billing calculation failed: {}", e))?;
    
    Ok(fee)
}

/// 调用AI Provider获取真实响应
async fn call_provider(
    state: &Arc<ServiceContext>,
    req: &ChatCompletionRequest,
    _user_id: &str,
) -> std::result::Result<ChatCompletion, String> {
    // 从ProviderRegistry获取Provider
    let provider_registry = state.provider_registry.read().await;
    
    // 根据模型名称查找对应的Provider
    // 这里简化处理，实际应该根据模型配置映射到具体的provider_id
    let provider_id = &req.model; // 假设model就是provider_id
    
    let provider = provider_registry.get_provider(provider_id)
        .ok_or_else(|| format!("Provider not found for model: {}", req.model))?;
    
    // 转换请求类型
    let provider_req = convert_to_provider_request(req);
    
    // 调用Provider的chat_completions方法
    let response = provider.chat_completions(provider_req, &serde_json::json!({}))
        .await
        .map_err(|e| format!("Provider error: {}", e))?;
    
    // 转换响应类型
    convert_from_provider_response(response)
}

/// 转换为Provider请求类型
fn convert_to_provider_request(req: &ChatCompletionRequest) -> crate::domain::dto::chat::ChatCompletionRequest {
    crate::domain::dto::chat::ChatCompletionRequest {
        model: req.model.clone(),
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
    }
}

/// 转换从Provider响应类型
fn convert_from_provider_response(response: crate::domain::dto::chat::ChatCompletionResponse) -> std::result::Result<ChatCompletion, String> {
    match response {
        crate::domain::dto::chat::ChatCompletionResponse::NonStream(completion) => {
            Ok(ChatCompletion {
                id: completion.id,
                object: completion.object,
                created: completion.created,
                model: completion.model,
                choices: completion.choices.into_iter().map(|choice| {
                    ChatCompletionChoice {
                        index: choice.index,
                        message: ChatCompletionMessage {
                            role: choice.message.role,
                            content: choice.message.content.map(|content| {
                                match content {
                                    crate::domain::dto::content::ChatMessageContent::String(s) => {
                                        ChatMessageContent::String(s)
                                    }
                                    crate::domain::dto::content::ChatMessageContent::Array(parts) => {
                                        ChatMessageContent::Array(parts.into_iter().map(|part| {
                                            ChatMessageContentPart {
                                                r#type: part.r#type,
                                                text: part.text,
                                                image_url: part.image_url,
                                            }
                                        }).collect())
                                    }
                                }
                            }),
                            name: choice.message.name,
                            tool_calls: choice.message.tool_calls,
                            tool_call_id: choice.message.tool_call_id,
                            refusal: choice.message.refusal,
                        },
                        finish_reason: choice.finish_reason,
                        logprobs: choice.logprobs,
                    }
                }).collect(),
                usage: Usage {
                    prompt_tokens: completion.usage.prompt_tokens as u32,
                    completion_tokens: completion.usage.completion_tokens as u32,
                    total_tokens: completion.usage.total_tokens as u32,
                },
                system_fingerprint: completion.system_fingerprint,
            })
        }
        crate::domain::dto::chat::ChatCompletionResponse::Stream(_) => {
            Err("Stream response not supported in non-stream mode".to_string())
        }
    }
}