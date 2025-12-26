//! AI Hub 补全控制器模块
//!
//! 提供OpenAI兼容的文本补全API接口

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
use crate::domain::dto::completion::CompletionRequest;
use crate::domain::vo::response::ApiResponse;
use crate::service::{TokenCounter, Content, TokenCountMeta};
use crate::domain::vo::completion::{CompletionResponse, CompletionChoice};
use crate::domain::vo::usage::Usage;
use crate::domain::dto::validation::Validator;

/// 文本补全接口
///
/// 提供OpenAI兼容的文本补全功能
#[utoipa::path(
    post,
    path = "/api/v1/completions",
    request_body = CompletionRequest,
    responses(
        (status = 200, description = "文本补全成功", body = ApiResponse<CompletionResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<CompletionResponse>),
        (status = 401, description = "未授权", body = ApiResponse<CompletionResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<CompletionResponse>)
    ),
    tag = "completion",
    security(
        ("api_key" = [])
    )
)]
#[axum::debug_handler]
pub async fn completions(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(req): Json<CompletionRequest>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    log::info!("[AI Hub] Completion request: {}", request_id);
    
    // 1. 用户认证
    let user_id = match authenticate_user(&headers, &state) {
        Ok(id) => id,
        Err(e) => return Json(ApiResponse::error("401", &e.to_string())),
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);
    
    // 2. 输入验证
    match Validator::validate_completion_request(
        &req.model,
        &req.prompt,
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
            return Json(ApiResponse::error("400", &format!("输入验证失败: {}", e)));
        }
    }
    
    // 3. Token计算
    let token_meta = match calculate_tokens(&req) {
        Ok(meta) => meta,
        Err(e) => return Json(ApiResponse::error("500", &e.to_string())),
    };
    log::info!("[AI Hub] Token calculation: input={}, model={}",
        token_meta.input_tokens, req.model);
    
    // 4. 预消费和配额检查
    let billing_service = &state.billing_service;
    let (base_input_price, base_output_price) = get_pricing(&req.model);
    
    // 预估输出token
    let estimated_output_tokens = req.max_tokens.unwrap_or(500) as i64;
    
    let fee = match billing_service.calculate_and_check(
        &user_id,
        &req.model,
        "unknown",
        token_meta.input_tokens,
        estimated_output_tokens,
        base_input_price,
        base_output_price,
        "completion",
    ).await {
        Ok(fee) => fee,
        Err(e) => return Json(ApiResponse::error("400", &e.to_string())),
    };
    
    log::info!("[AI Hub] Pre-consumption check passed: cost={:.2}", fee.total_cost);
    
    // 5. 调用AI服务（简化实现，返回成功响应）
    let response = create_mock_response(&req, &token_meta);
    
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
        Err(e) => return Json(ApiResponse::error("500", &format!("Failed to deduct quota and log: {}", e))),
    };
    
    log::info!("[AI Hub] Usage logged: {}", usage_log_id);
    
    // 7. 返回响应
    Json(ApiResponse::success(response))
}

/// 用户认证
fn authenticate_user(headers: &HeaderMap, _state: &Arc<ServiceContext>) -> std::result::Result<String, String> {
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
fn calculate_tokens(req: &CompletionRequest) -> std::result::Result<TokenCountMeta, String> {
    // 计算提示文本的token
    let content = Content::Text(req.prompt.clone());
    let meta = TokenCounter::count_content_tokens(
        &content,
        &req.model,
        req.stream.unwrap_or(false),
    ).map_err(|e| e.to_string())?;
    
    Ok(meta)
}

/// 获取模型定价
fn get_pricing(model: &str) -> (f64, f64) {
    if model.contains("gpt-3.5-turbo-instruct") {
        (0.0015, 0.002)
    } else {
        (0.01, 0.02) // 默认定价
    }
}

/// 创建模拟响应（用于测试）
fn create_mock_response(req: &CompletionRequest, token_meta: &TokenCountMeta) -> CompletionResponse {
    CompletionResponse {
        id: format!("cmpl-{}", Uuid::new_v4().to_string()),
        object: "text_completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        model: req.model.clone(),
        choices: vec![CompletionChoice {
            text: "This is a mock completion response.".to_string(),
            index: 0,
            logprobs: None,
            finish_reason: Some("stop".to_string()),
        }],
        usage: Usage {
            prompt_tokens: token_meta.input_tokens as u32,
            completion_tokens: 50,
            total_tokens: (token_meta.input_tokens + 50) as u32,
        },
    }
}