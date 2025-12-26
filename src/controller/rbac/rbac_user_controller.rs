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

// 用途：导入PageWrapper
// 说明：用于分页响应的OpenAPI文档生成
use crate::domain::vo::response::PageWrapper;

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
#[utoipa::path(
    post,
    path = "/api/v1/user/login",
    request_body = SignInDTO,
    responses(
        (status = 200, description = "登录成功", body = RespVO<SignInVO>),
        (status = 401, description = "登录失败", body = RespVO<SignInVO>)
    ),
    tag = "user"
)]
pub async fn login(arg: Json<SignInDTO>) -> impl IntoResponse {
    // 用途：输出登录日志
    // 说明：记录登录请求信息，便于调试和监控
    log::info!("login:{:?}", arg.0);
    // 用途：调用用户服务进行登录验证
    // 说明：验证用户账号和密码，生成JWT令牌
    let result = CONTEXT.sys_user_service.sign_in(&arg.0).await;
    // 用途：处理登录结果
    // 说明：成功时返回200状态码，失败时返回401状态码
    use axum::http::StatusCode;
    match result {
        Ok(vo) => (StatusCode::OK, axum::Json(RespVO::from(vo))),
        Err(e) => {
            // 用途：返回401状态码
            // 说明：认证失败时返回401状态码
            let resp_vo = RespVO::<SignInVO>::from_error(e.to_string());
            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
        }
    }
}

/// 用途：获取用户信息
/// 说明：根据请求头中的JWT令牌获取当前登录用户的信息
#[utoipa::path(
    get,
    path = "/api/v1/user/info",
    responses(
        (status = 200, description = "获取成功", body = RespVO<SignInVO>),
        (status = 401, description = "认证失败", body = RespVO<SignInVO>)
    ),
    tag = "user"
)]
pub async fn info(req: Request) -> impl IntoResponse {
    use axum::http::StatusCode;
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
            // 用途：去除Bearer前缀
            // 说明：前端发送的令牌包含Bearer前缀，需要去除后才能验证
            let token = token.trim_start_matches("Bearer ");
            // 用途：验证令牌
            // 说明：确保令牌的有效性，防止伪造
            let token = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            // 用途：检查令牌验证结果
            // 说明：如果令牌无效，返回错误信息
            if let Err(e) = &token {
                let resp_vo = RespVO::<SignInVO>::from_error(e.to_string());
                return (StatusCode::UNAUTHORIZED, axum::Json(resp_vo));
            }
            // 用途：根据令牌获取用户信息
            // 说明：从数据库中查询用户的详细信息
            let user_data = CONTEXT
                .sys_user_service
                .get_user_info_by_token(&token.unwrap_or_default())
                .await;
            // 用途：将结果转换为响应VO
            // 说明：统一响应格式，包含状态码、消息和数据
            match user_data {
                Ok(data) => (StatusCode::OK, axum::Json(RespVO::from(data))),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(RespVO::<SignInVO>::from_error(e.to_string()))),
            }
        }
        _ => {
            let resp_vo = RespVO::<SignInVO>::from_error("缺少访问令牌，请先登录".to_string());
            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
        }
    }
}

/// 用途：添加用户
/// 说明：处理用户的添加请求
#[utoipa::path(
    post,
    path = "/api/v1/user/add",
    request_body = UserAddDTO,
    responses(
        (status = 200, description = "添加成功", body = RespVO<u64>),
        (status = 400, description = "添加失败", body = RespVO<u64>)
    ),
    tag = "user"
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/user/page",
    request_body = UserRolePageDTO,
    responses(
        (status = 200, description = "查询成功", body = RespVO<PageWrapper<crate::domain::vo::SysUserVO>>),
        (status = 400, description = "查询失败", body = RespVO<PageWrapper<crate::domain::vo::SysUserVO>>)
    ),
    tag = "user"
)]
pub async fn page(arg: Json<UserRolePageDTO>) -> impl IntoResponse {
    // 用途：调用用户服务分页查询用户
    // 说明：从数据库中分页查询用户数据
    let vo = CONTEXT.sys_user_service.role_page(&arg.0).await;
    // 用途：将结果转换为PageWrapper
    // 说明：将rbatis Page转换为PageWrapper以支持OpenAPI文档生成
    let wrapper_data: Result<PageWrapper<crate::domain::vo::SysUserVO>, _> = vo.map(|p| PageWrapper {
                page: p.page_no,
                page_size: p.page_size,
                total: p.total,
                records: p.records,
            });
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(wrapper_data)
}

/// 用途：获取用户详情
/// 说明：根据用户ID获取用户的详细信息
#[utoipa::path(
    post,
    path = "/api/v1/user/detail",
    request_body = IdDTO,
    responses(
        (status = 200, description = "查询成功", body = RespVO<crate::domain::vo::SysUserVO>),
        (status = 400, description = "查询失败", body = RespVO<crate::domain::vo::SysUserVO>)
    ),
    tag = "user"
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/user/update",
    request_body = UserEditDTO,
    responses(
        (status = 200, description = "更新成功", body = RespVO<u64>),
        (status = 400, description = "更新失败", body = RespVO<u64>)
    ),
    tag = "user"
)]
pub async fn update(arg: Json<UserEditDTO>) -> impl IntoResponse {
    // 用途：检查是否是禁用超级管理员
    // 说明：超级管理员账号不能被禁用，防止系统无法登录
    if let (Some(user_id), Some(state)) = (arg.0.id.as_ref(), arg.0.state.as_ref()) {
        if *state == 0 {
            // 用途：检查用户是否是超级管理员
            // 说明：如果是超级管理员，则不允许禁用
            if let Ok(is_admin) = CONTEXT.sys_user_service.is_super_admin(user_id).await {
                if is_admin {
                    return RespVO::<u64>::from_result(Err(Error::from(error_info!(
                        "cannot_disable_admin"
                    ))));
                }
            }
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
#[utoipa::path(
    post,
    path = "/api/v1/user/remove",
    request_body = IdDTO,
    responses(
        (status = 200, description = "删除成功", body = RespVO<u64>),
        (status = 400, description = "删除失败", body = RespVO<u64>)
    ),
    tag = "user"
)]
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
