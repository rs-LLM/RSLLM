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
use crate::domain::dto::content::{ChatMessageContent, ChatCompletionMessage};
use crate::domain::vo::chat::{ChatCompletion, ChatCompletionChoice};
use crate::domain::vo::usage::Usage;

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
    
    // 2. Token计算
    let token_meta = match calculate_tokens(&req) {
        Ok(meta) => meta,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    log::info!("[AI Hub] Token calculation: input={}, model={}",
        token_meta.input_tokens, req.model);
    
    // 3. 预消费和配额检查
    let billing_service = &state.billing_service;
    let (base_input_price, base_output_price) = get_pricing(&req.model);
    
    // 预估输出token（基于max_tokens或默认值）
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;
    
    let fee = match billing_service.calculate_and_check(
        &user_id,
        &req.model,
        "unknown", // provider_id将在实际调用时确定
        token_meta.input_tokens,
        estimated_output_tokens,
        base_input_price,
        base_output_price,
        "chat",
    ).await {
        Ok(fee) => fee,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    
    log::info!("[AI Hub] Pre-consumption check passed: cost={:.2}", fee.total_cost);
    
    // 4. 调用AI服务（简化实现，返回成功响应）
    // 实际应该调用provider服务
    let response = create_mock_response(&req, &token_meta);
    
    // 5. 实际扣费和记录用量
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
        Err(e) => return RespVO::from_error(format!("Failed to deduct quota and log: {}", e)),
    };
    
    log::info!("[AI Hub] Usage logged: {}", usage_log_id);
    
    // 6. 返回响应
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

/// 创建模拟响应（用于测试）
fn create_mock_response(req: &ChatCompletionRequest, token_meta: &TokenCountMeta) -> ChatCompletion {
    ChatCompletion {
        id: format!("chatcmpl-{}", Uuid::new_v4().to_string()),
        object: Some("chat.completion".to_string()),
        created: Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()),
        model: req.model.clone(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatCompletionMessage {
                role: "assistant".to_string(),
                content: Some(ChatMessageContent::String(
                    "This is a mock response for testing.".to_string()
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: Usage {
            prompt_tokens: token_meta.input_tokens as u32,
            completion_tokens: 50,
            total_tokens: (token_meta.input_tokens + 50) as u32,
        },
        system_fingerprint: None,
    }
}