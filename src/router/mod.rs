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

// 用途：重新导出各路由模块
// 说明：便于外部统一导入
pub use no_auth_router::create_no_auth_router;
pub use auth_router::create_auth_router;
pub use resource_router::create_resource_router;
pub use ai_hub_router::create_ai_hub_router;

use axum::Router;
use std::sync::Arc;
use crate::context::ServiceContext;

/// 创建完整的应用路由
///
/// 合并所有路由配置，返回完整的应用路由
pub fn create_app_router(state: Arc<ServiceContext>) -> Router {
    Router::new()
        .merge(create_resource_router())  // 合并静态资源路由
        .merge(create_no_auth_router())   // 合并无认证路由
        .merge(create_auth_router())      // 合并需认证路由
        .merge(create_ai_hub_router().with_state(state))  // 合并AI Hub路由
}