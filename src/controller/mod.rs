// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入响应转换特性和Response类型
// 说明：用于为RespVO实现IntoResponse trait
use axum::response::{IntoResponse, Response};

// 用途：导入序列化特性
// 说明：用于约束RespVO的泛型参数
use serde::Serialize;

// 用途：导入反序列化特性
// 说明：用于约束RespVO的泛型参数
use serde::de::DeserializeOwned;

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
    /// 说明：实现axum的响应转换机制
    fn into_response(self) -> Response {
        // 用途：将RespVO包装为axum::Json
        // 说明：axum可以直接处理Json类型的响应
        axum::Json(self).into_response()
    }
}
