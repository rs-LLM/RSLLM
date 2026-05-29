//! 静态资源路由模块
//!
//! 该模块定义静态文件服务，用于提供前端页面和静态资源访问

use crate::context::ServiceContext;
use axum::{
    Router,
    body::Body,
    extract::Request,
    response::{IntoResponse, Response},
};
use include_dir::{Dir, include_dir};
use std::sync::Arc;

// 嵌入 dist 目录到二进制文件中
static DIST_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/dist");

/// 创建静态资源路由
///
/// 返回静态文件服务配置，支持SPA应用
pub fn create_resource_router() -> Router<Arc<ServiceContext>> {
    Router::new().fallback_service(axum::routing::get(serve_static))
}

/// 服务静态文件
async fn serve_static(req: Request) -> Response {
    let path = req.uri().path();
    let path = path.trim_start_matches('/');

    if path.is_empty() || path == "/" {
        return serve_file("index.html");
    }

    if let Some(file) = DIST_DIR.get_file(path) {
        let contents = file.contents();
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Response::builder()
            .header("content-type", mime.essence_str())
            .body(Body::from(contents.to_vec()))
            .unwrap()
            .into_response()
    } else {
        Response::builder()
            .status(404)
            .body(Body::empty())
            .unwrap()
            .into_response()
    }
}

/// 服务单个文件
fn serve_file(path: &str) -> Response {
    if let Some(file) = DIST_DIR.get_file(path) {
        let contents = file.contents();
        Response::builder()
            .header("content-type", "text/html")
            .body(Body::from(contents.to_vec()))
            .unwrap()
            .into_response()
    } else {
        Response::builder()
            .status(404)
            .body(Body::empty())
            .unwrap()
            .into_response()
    }
}
