//! 路由模块
//! 
//! 该模块负责组织和导出所有路由配置，包括：
//! - 无认证路由
//! - 需认证路由  
//! - 静态资源路由

pub mod no_auth_router;
pub mod auth_router;
pub mod resource_router;
pub mod ai_hub_router;
pub mod admin_router;
pub mod state_router;
pub mod init_router;

// 用途：重新导出各路由模块
// 说明：便于外部统一导入
pub use no_auth_router::create_no_auth_router;
pub use auth_router::create_auth_router;
pub use resource_router::create_resource_router;
pub use ai_hub_router::create_ai_hub_router;
pub use admin_router::create_admin_router;
pub use state_router::create_state_router;
pub use init_router::create_init_router;

use axum::Router;
use std::sync::Arc;
use crate::context::ServiceContext;

/// 创建完整的应用路由
///
/// 合并所有路由配置，返回完整的应用路由
pub fn create_app_router(state: Arc<ServiceContext>) -> Router {
    // 创建需要状态的路由
    Router::new()
        .merge(create_resource_router())  // 合并静态资源路由
        .merge(create_init_router())    // 合并初始化向导路由（无认证）
        .merge(create_no_auth_router()) // 合并无认证路由（需要State）
        .merge(create_auth_router())     // 合并需认证路由
        .merge(create_ai_hub_router())  // 合并AI Hub路由
        .merge(create_admin_router())   // 合并管理路由
        .merge(create_state_router())   // 合并需要状态的路由
        .with_state(state)
}