//! 无认证路由模块
//! 
//! 该模块定义不需要身份验证的API接口，包括：
//! - 健康检查接口
//! - 登录接口
//! - 认证检查接口
//! - 验证码接口

use axum::Router;
use axum::routing::{get, post};

// 用途：导入控制器
// 说明：用于处理HTTP请求
use crate::controller::{
    img_controller,
    rbac_user_controller,
    sys_auth_controller,
};

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

/// 创建无认证路由
/// 
/// 返回不需要身份验证的路由配置
pub fn create_no_auth_router() -> Router {
    Router::new()
        // 用途：定义根路径路由
        // 说明：提供简单的健康检查接口
        .route(
            "/admin/",
            get(|| async { RespVO::from("hello".to_string()) }),
        )
        // 用途：定义登录路由
        // 说明：处理用户登录请求
        .route("/admin/sys_login", post(rbac_user_controller::login))
        // 用途：定义认证检查路由
        // 说明：检查用户认证状态
        .route("/admin/auth/check", post(sys_auth_controller::check))
        // 用途：定义验证码路由
        // 说明：生成和返回验证码图片
        .route("/admin/captcha", get(img_controller::captcha))
}