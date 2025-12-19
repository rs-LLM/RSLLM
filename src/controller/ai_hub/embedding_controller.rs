//! AI Hub 嵌入控制器模块
//!
//! 提供OpenAI兼容的嵌入生成API接口

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
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::vo::RespVO;
use crate::service::{TokenCounter, Content};
use crate::domain::vo::embeddings::{EmbeddingsResponse, Embeddings, Embedding};
use crate::domain::vo::usage::EmbeddingUsage;

/// 嵌入生成接口
///
/// 提供OpenAI兼容的嵌入生成功能
#[axum::debug_handler]
pub async fn embeddings(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(req): Json<EmbeddingsRequest>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    
    log::info!("[AI Hub] Embeddings request: {}", request_id);
    
    // 1. 用户认证
    let user_id = match authenticate_user(&headers, &state) {
        Ok(id) => id,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    log::info!("[AI Hub] User authenticated: {}", user_id);
    
    // 2. Token计算
    let (input_tokens, input_text_count) = match calculate_tokens(&req) {
        Ok(result) => result,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    log::info!("[AI Hub] Token calculation: input={}, model={}",
        input_tokens, req.model);
    
    // 3. 预消费和配额检查
    let billing_service = &state.billing_service;
    let (base_input_price, base_output_price) = get_pricing(&req.model);
    
    // 嵌入通常没有输出token，但为了统一处理，设置为0
    let fee = match billing_service.calculate_and_check(
        &user_id,
        &req.model,
        "unknown",
        input_tokens,
        0, // 嵌入没有输出token
        base_input_price,
        base_output_price,
        "embeddings",
    ).await {
        Ok(fee) => fee,
        Err(e) => return RespVO::from_error(e.to_string()),
    };
    
    log::info!("[AI Hub] Pre-consumption check passed: cost={:.2}", fee.total_cost);
    
    // 4. 调用AI服务（简化实现，返回成功响应）
    let response = create_mock_response(&req, input_text_count);
    
    // 5. 实际扣费和记录用量
    let duration_ms = start_time.elapsed().as_millis() as i64;
    let usage_log_id = match billing_service.deduct_quota_and_log(
        &fee,
        &request_id,
        duration_ms,
        "success",
        Some(serde_json::json!({
            "model": req.model,
            "input_count": input_text_count,
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
fn calculate_tokens(req: &EmbeddingsRequest) -> std::result::Result<(i64, usize), String> {
    let mut total_tokens = 0;
    let input_count;
    
    match &req.input {
        crate::domain::dto::embeddings::EmbeddingsInput::Single(text) => {
            let content = Content::Text(text.clone());
            let meta = TokenCounter::count_content_tokens(&content, &req.model, false)
                .map_err(|e| e.to_string())?;
            total_tokens += meta.input_tokens;
            input_count = 1;
        }
        crate::domain::dto::embeddings::EmbeddingsInput::Multiple(texts) => {
            for text in texts {
                let content = Content::Text(text.clone());
                let meta = TokenCounter::count_content_tokens(&content, &req.model, false)
                    .map_err(|e| e.to_string())?;
                total_tokens += meta.input_tokens;
            }
            input_count = texts.len();
        }
        crate::domain::dto::embeddings::EmbeddingsInput::SingleTokenIds(token_ids) => {
            // Token ID数组，每个ID算作1个token
            total_tokens += token_ids.len() as i64;
            input_count = 1;
        }
        crate::domain::dto::embeddings::EmbeddingsInput::MultipleTokenIds(token_ids_list) => {
            for token_ids in token_ids_list {
                total_tokens += token_ids.len() as i64;
            }
            input_count = token_ids_list.len();
        }
    }
    
    Ok((total_tokens, input_count))
}

/// 获取模型定价
fn get_pricing(model: &str) -> (f64, f64) {
    if model.contains("text-embedding-ada-002") {
        (0.0001, 0.0) // $0.0001/1K tokens, no output cost
    } else if model.contains("text-embedding-3-small") {
        (0.00002, 0.0) // $0.00002/1K tokens
    } else if model.contains("text-embedding-3-large") {
        (0.00013, 0.0) // $0.00013/1K tokens
    } else {
        (0.0001, 0.0) // 默认定价
    }
}

/// 创建模拟响应（用于测试）
fn create_mock_response(req: &EmbeddingsRequest, input_count: usize) -> EmbeddingsResponse {
    // 生成模拟的嵌入向量（简化处理）
    let mock_embedding = vec![0.1_f32; 1536]; // 假设1536维
    
    let data = (0..input_count).map(|i| Embeddings {
        object: "embedding".to_string(),
        embedding: Embedding::Float(mock_embedding.clone()),
        index: i,
    }).collect();
    
    EmbeddingsResponse {
        object: "list".to_string(),
        data,
        model: req.model.clone(),
        usage: EmbeddingUsage {
            prompt_tokens: input_count as u32 * 100, // 估算
            total_tokens: input_count as u32 * 100,
        },
    }
}