// 用途：导入axum的Json提取器
// 说明：用于从HTTP请求体中提取JSON数据
use axum::Json;

// 用途：导入axum的Request提取器
// 说明：用于从HTTP请求中获取信息，如头信息
use axum::extract::Request;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

// 用途：导入全局上下文
// 说明：用于访问用户服务和配置信息
use crate::context::CONTEXT;

// 用途：导入用户相关的数据传输对象
// 说明：用于接收用户的登录、添加、编辑、分页查询等请求参数
use crate::domain::dto::{IdDTO, SignInDTO, UserAddDTO, UserEditDTO, UserRolePageDTO};

// 用途：导入用户相关的VO
// 说明：用于返回登录结果、JWT令牌和统一响应格式
use crate::domain::vo::{JWTToken, RespVO, SignInVO};

// 用途：导入自定义错误类型
// 说明：用于处理错误情况
use crate::error::Error;

// 用途：导入错误信息宏
// 说明：用于生成错误信息
use crate::error_info;

// 用途：导入令牌密钥常量
// 说明：用于从请求头中获取令牌
use crate::middleware::auth_axum::TOKEN_KEY;

/// 用途：用户登录
/// 说明：处理用户登录请求，验证用户身份并返回JWT令牌
pub async fn login(arg: Json<SignInDTO>) -> impl IntoResponse {
    // 用途：输出登录日志
    // 说明：记录登录请求信息，便于调试和监控
    log::info!("login:{:?}", arg.0);
    // 用途：调用用户服务进行登录验证
    // 说明：验证用户账号和密码，生成JWT令牌
    let vo = CONTEXT.sys_user_service.sign_in(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：获取用户信息
/// 说明：根据请求头中的JWT令牌获取当前登录用户的信息
pub async fn info(req: Request) -> impl IntoResponse {
    // 用途：从请求头中获取令牌
    // 说明：JWT令牌存储在请求头中，用于身份验证
    let token = req.headers().get(TOKEN_KEY);
    // 用途：匹配令牌是否存在
    // 说明：处理令牌存在和不存在的情况
    match token {
        Some(token) => {
            // 用途：将令牌转换为字符串
            // 说明：方便后续处理
            let token = token.to_str().unwrap_or("");
            // 用途：验证令牌
            // 说明：确保令牌的有效性，防止伪造
            let token = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            // 用途：检查令牌验证结果
            // 说明：如果令牌无效，返回错误信息
            if let Err(e) = &token {
                return RespVO::<SignInVO>::from_error(e.to_string());
            }
            // 用途：根据令牌获取用户信息
            // 说明：从数据库中查询用户的详细信息
            let user_data = CONTEXT
                .sys_user_service
                .get_user_info_by_token(&token.unwrap_or_default())
                .await;
            // 用途：将结果转换为响应VO
            // 说明：统一响应格式，包含状态码、消息和数据
            RespVO::from_result(user_data)
        }
        _ => {
            // 用途：返回令牌为空的错误
            // 说明：令牌是获取用户信息的必要条件
            RespVO::<SignInVO>::from_error(error_info!("access_token_empty"))
        }
    }
}

/// 用途：添加用户
/// 说明：处理用户的添加请求
pub async fn add(arg: Json<UserAddDTO>) -> impl IntoResponse {
    // 用途：调用用户服务添加用户
    // 说明：将新用户数据保存到数据库
    let vo = CONTEXT.sys_user_service.add(arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：分页查询用户
/// 说明：处理用户的分页查询请求
pub async fn page(arg: Json<UserRolePageDTO>) -> impl IntoResponse {
    // 用途：调用用户服务分页查询用户
    // 说明：从数据库中分页查询用户数据
    let vo = CONTEXT.sys_user_service.role_page(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：获取用户详情
/// 说明：根据用户ID获取用户的详细信息
pub async fn detail(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：调用用户服务获取用户详情
    // 说明：从数据库中查询指定ID的用户信息
    let vo = CONTEXT.sys_user_service.detail(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：更新用户
/// 说明：处理用户的更新请求
pub async fn update(arg: Json<UserEditDTO>) -> impl IntoResponse {
    // 用途：检查是否是禁用超级管理员
    // 说明：超级管理员账号不能被禁用，防止系统无法登录
    if let (Some(account), Some(state)) = (arg.0.account.as_ref(), arg.0.state.as_ref()) {
        if account == "00000000000" && *state == 0 {
            return RespVO::<u64>::from_result(Err(Error::from(error_info!(
                "cannot_disable_admin"
            ))));
        }
    }
    // 用途：调用用户服务更新用户
    // 说明：更新数据库中的用户数据
    let vo = CONTEXT.sys_user_service.edit(arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：删除用户
/// 说明：处理用户的删除请求
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：调用用户服务删除用户
    // 说明：从数据库中删除指定ID的用户
    let vo = CONTEXT
        .sys_user_service
        .remove(&arg.0.id.unwrap_or_default())
        .await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}
