//! 需要状态的路由模块
//! 
//! 该模块定义需要ServiceContext状态的API接口

use axum::{Router};
use std::sync::Arc;

use crate::context::ServiceContext;

/// 创建需要状态的路由
/// 
/// 返回需要ServiceContext状态的路由配置
pub fn create_state_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        // 用户查询路由（需要认证）
        .route("/api/v1/quota/overview", axum::routing::get(crate::controller::ai_hub::quota_controller::get_quota_overview))
        .route("/api/v1/billing/statistics", axum::routing::get(crate::controller::ai_hub::bill_controller::get_billing_statistics))
        // 统计分析路由
        .route("/api/v1/analytics/user-usage", axum::routing::get(crate::controller::ai_hub::analytics_controller::get_user_usage_stats))
        .route("/api/v1/analytics/system", axum::routing::get(crate::controller::ai_hub::analytics_controller::get_system_stats))
        .route("/api/v1/analytics/performance-trends", axum::routing::get(crate::controller::ai_hub::analytics_controller::get_performance_trends))
        // 流式响应路由
        .route("/api/v1/chat/completions/sse", axum::routing::post(crate::controller::ai_hub::streaming_controller::chat_completions_sse))
        .route("/api/v1/chat/completions/ws", axum::routing::get(crate::controller::ai_hub::streaming_controller::chat_completions_ws))
}