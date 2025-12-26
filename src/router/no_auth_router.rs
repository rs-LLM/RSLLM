//! 无认证路由模块
//! 
//! 该模块定义不需要身份验证的API接口，包括：
//! - 健康检查接口
//! - 登录接口
//! - 认证检查接口
//! - 验证码接口
//! - 刷新Token接口
//! - 退出登录接口

use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

// 用途：导入控制器
// 说明：用于处理HTTP请求
use crate::controller::{
    img_controller,
    rbac_user_controller,
    sys_auth_controller,
    ai_hub::model_definition_controller,
};

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入ServiceContext
// 说明：用于需要状态的路由
use crate::context::ServiceContext;

/// 创建无认证路由
/// 
/// 返回不需要身份验证的路由配置
pub fn create_no_auth_router() -> Router<Arc<ServiceContext>> {
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
        // 用途：定义登录路由（兼容前端请求）
        // 说明：处理前端发送的 /admin/login 请求
        .route("/admin/login", post(rbac_user_controller::login))
        // 用途：定义登录路由（Vben前端专用）
        // 用途：处理登录请求，返回符合Vben期望的响应格式
                .route("/api/v1/auth/login", post(sys_auth_controller::login))
        // 用途：定义认证检查路由
        // 说明：检查用户认证状态
        .route("/admin/auth/check", post(sys_auth_controller::check))
        // 用途：定义刷新Token路由
        // 说明：延长Token有效期
        .route("/admin/auth/refresh", post(sys_auth_controller::refresh_token))
        // 用途：定义退出登录路由
        // 说明：清除用户登录状态
        .route("/admin/auth/logout", post(sys_auth_controller::logout))
        // 用途：定义验证码路由
        // 说明：生成和返回验证码图片
        .route("/admin/captcha", get(img_controller::captcha))
        // 用途：定义公开的模型列表路由
        // 说明：无需认证即可获取模型列表和价格信息
        .route("/api/v1/admin/models", get(model_definition_controller::list_models))
        // 用途：定义公开的模型详情路由
        // 说明：无需认证即可获取单个模型详情和价格信息
        .route("/api/v1/admin/models/{id}", get(model_definition_controller::get_model))
}