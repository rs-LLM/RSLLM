// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入响应转换特性和Response类型
// 说明：用于为RespVO实现IntoResponse trait
use axum::response::{IntoResponse, Response};

// 用途：导入StatusCode
// 说明：用于设置HTTP状态码
use axum::http::StatusCode;

// 用途：导入序列化特性
// 说明：用于约束RespVO的泛型参数
use serde::Serialize;

// 用途：导入反序列化特性
// 说明：用于约束RespVO的泛型参数
use serde::de::DeserializeOwned;

// 用途：导出初始化向导控制器模块
// 说明：允许其他模块访问初始化向导相关的控制器功能
pub mod init_controller;

// 用途：导出图片控制器模块
// 说明：允许其他模块访问图片相关的控制器功能
pub mod img_controller;

// 用途：导出权限模块
// 说明：允许其他模块访问权限功能
pub mod rbac;
pub use rbac::*;

// 用途：导出系统模块
// 说明：允许其他模块访问系统相关的控制器功能
pub mod sys;
pub use sys::*;

// 用途：导出AI Hub模块
// 说明：允许其他模块访问AI Hub相关的控制器功能
pub mod ai_hub;
pub use ai_hub::*;

/// 用途：为RespVO实现IntoResponse特性
/// 说明：允许RespVO直接作为axum响应返回，简化控制器代码
impl<T: Serialize + DeserializeOwned> IntoResponse for RespVO<T> {
    /// 用途：将RespVO转换为HTTP响应
    /// 说明：实现axum的响应转换机制，根据错误码返回正确的HTTP状态码
    fn into_response(self) -> Response {
        // 用途：确定HTTP状态码
        // 说明：根据RespVO的code字段和msg字段判断返回的状态码
        let status_code = match (&self.code, &self.msg) {
            // 用途：成功响应
            // 说明：code为"0"或None时返回200
            (Some(code), _) if code == "0" => StatusCode::OK,
            (None, _) => StatusCode::OK,
            
            // 用途：认证错误
            // 说明：access_token_empty、token_invalid等返回401
            (_, Some(msg)) if msg.contains("access_token_empty") => StatusCode::UNAUTHORIZED,
            (_, Some(msg)) if msg.contains("token_invalid") => StatusCode::UNAUTHORIZED,
            (_, Some(msg)) if msg.contains("token_expired") => StatusCode::UNAUTHORIZED,
            (_, Some(msg)) if msg.contains("unauthorized") => StatusCode::UNAUTHORIZED,
            
            // 用途：参数错误
            // 说明：参数缺失或格式错误返回400
            (_, Some(msg)) if msg.contains("parameter") || msg.contains("invalid") => StatusCode::BAD_REQUEST,
            
            // 用途：资源不存在
            // 说明：资源未找到返回404
            (_, Some(msg)) if msg.contains("not_found") || msg.contains("not found") => StatusCode::NOT_FOUND,
            
            // 用途：其他错误
            // 说明：默认返回200，错误信息在响应体中
            _ => StatusCode::OK,
        };
        
        // 用途：返回带状态码的JSON响应
        // 说明：将RespVO包装为axum::Json并设置正确的状态码
        (status_code, axum::Json(self)).into_response()
    }
}
