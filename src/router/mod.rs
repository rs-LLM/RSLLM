//! 路由模块
//!
//! 该模块负责组织和导出所有路由配置，包括：
//! - 无认证路由
//! - 需认证路由  
//! - 静态资源路由

pub mod admin_router;
pub mod ai_hub_router;
pub mod api_key_router;
pub mod auth_router;
pub mod init_router;
pub mod no_auth_router;
pub mod resource_router;
pub mod state_router;
pub mod user_level_model_rate_limit_router;
pub mod user_level_router;

pub use admin_router::create_admin_router;
pub use ai_hub_router::create_ai_hub_router;
pub use api_key_router::create_api_key_and_rate_limit_router;
pub use auth_router::create_auth_router;
pub use init_router::create_init_router;
pub use no_auth_router::create_no_auth_router;
pub use resource_router::create_resource_router;
pub use state_router::create_state_router;
pub use user_level_model_rate_limit_router::create_user_level_model_rate_limit_router;
pub use user_level_router::create_user_level_router;

use crate::context::ServiceContext;
use axum::Router;
use std::sync::Arc;

/// 创建完整的应用路由
///
/// 合并所有路由配置，返回完整的应用路由
pub fn create_app_router(state: Arc<ServiceContext>) -> Router<()> {
    Router::new()
        // 添加统一前缀的路由
        .nest(
            "/api/v1",
            create_ai_hub_router(state.clone()).with_state(state.clone()),
        )
        // 流式响应接口，保持 /api/v1 前缀
        .route(
            "/api/v1/chat/completions/ws",
            axum::routing::get(
                crate::controller::ai_hub::streaming_controller::chat_completions_ws,
            ),
        )
        // 添加统一前缀的路由
        .nest(
            "/rsllm/api",
            create_resource_router().with_state(state.clone()),
        )
        .nest("/rsllm/api", create_init_router().with_state(state.clone()))
        .nest(
            "/rsllm/api",
            create_no_auth_router().with_state(state.clone()),
        )
        .nest("/rsllm/api", create_auth_router().with_state(state.clone()))
        .nest(
            "/rsllm/api",
            create_admin_router().with_state(state.clone()),
        )
        .nest(
            "/rsllm/api",
            create_state_router().with_state(state.clone()),
        )
        .nest(
            "/rsllm/api",
            create_api_key_and_rate_limit_router().with_state(state.clone()),
        )
        .nest(
            "/rsllm/api",
            create_user_level_router().with_state(state.clone()),
        )
        .nest(
            "/rsllm/api",
            create_user_level_model_rate_limit_router().with_state(state.clone()),
        )
        .with_state(state)
}
