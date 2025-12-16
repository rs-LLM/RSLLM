// 用途：导入全局上下文
// 说明：用于访问系统认证服务
use crate::context::CONTEXT;

// 用途：导入系统认证DTO
// 说明：用于接收认证检查请求参数
use crate::domain::dto::auth::SysAuthDTO;

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入axum的Json提取器
// 说明：用于从HTTP请求体中提取JSON数据
use axum::Json;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

/// 用途：检查令牌和路径是否有效且可访问
/// 说明：验证用户是否有权限访问指定路径
pub async fn check(arg: Json<SysAuthDTO>) -> impl IntoResponse {
    // 用途：调用系统认证服务检查权限
    // 说明：验证用户令牌和请求路径的权限关系
    let r = CONTEXT.sys_auth_service.check_auth(arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(r)
}
