//! 静态资源路由模块
//!
//! 该模块定义静态文件服务，用于提供前端页面和静态资源访问

use crate::context::ServiceContext;
use axum::Router;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

/// 创建静态资源路由
///
/// 返回静态文件服务配置，支持SPA应用
pub fn create_resource_router() -> Router<Arc<ServiceContext>> {
    Router::new().fallback_service(
        // 用途：定义静态目录服务
        // 说明：提供dist目录下的静态文件访问
        ServeDir::new("dist/").not_found_service(
            // 用途：定义404页面
            // 说明：当找不到请求的静态文件时，返回index.html，支持SPA应用
            ServeFile::new("dist/index.html"),
        ),
    )
}
