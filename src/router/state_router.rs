//! 需要状态的路由模块
//!
//! 该模块定义需要ServiceContext状态的API接口

use axum::Router;
use std::sync::Arc;

use crate::context::ServiceContext;

/// 创建需要状态的路由
///
/// 返回需要ServiceContext状态的路由配置
pub fn create_state_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        // 用户查询路由（需要认证）
        .route(
            "/quota/overview",
            axum::routing::get(crate::controller::ai_hub::quota_controller::get_quota_overview),
        )
        // 统计分析路由
        .route(
            "/analytics/user-usage",
            axum::routing::get(
                crate::controller::ai_hub::analytics_controller::get_user_usage_stats,
            ),
        )
        .route(
            "/analytics/system",
            axum::routing::get(crate::controller::ai_hub::analytics_controller::get_system_stats),
        )
        .route(
            "/analytics/performance-trends",
            axum::routing::get(
                crate::controller::ai_hub::analytics_controller::get_performance_trends,
            ),
        )
}
