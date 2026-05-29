//! AI Hub控制器模块
//!
//! 提供OpenAI兼容的API控制器

use axum::http::HeaderMap;

pub(crate) fn extract_client_meta(headers: &HeaderMap) -> (Option<String>, Option<String>) {
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(std::string::ToString::to_string);

    let x_forwarded_for = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let x_real_ip = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    (x_forwarded_for.or(x_real_ip), user_agent)
}

pub mod admin_stats_controller;
pub mod analytics_controller;
pub mod api_key_controller;
pub mod balance_controller;
pub mod chat_controller;
pub mod checkin_controller;
pub mod dashboard_billing_controller;
pub mod embedding_controller;
pub mod messages_controller;
pub mod model_controller;
pub mod model_provider_mapping_controller;
pub mod provider;
pub mod provider_config_controller;
pub mod quota_controller;
pub mod rate_limit_controller;
pub mod redeem_code_controller;
pub mod responses_controller;
pub mod streaming_controller;
pub mod subscription_controller;
pub mod transaction_controller;
pub mod usage_log_controller;
pub mod user_level_controller;
pub mod user_level_model_rate_limit_controller;
pub mod user_stats_controller;
