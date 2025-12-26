// 用途：导入全局上下文
// 说明：用于访问系统认证服务
use crate::context::CONTEXT;

// 用途：导入系统认证DTO
// 说明：用于接收认证检查请求参数
use crate::domain::dto::basic::auth::SysAuthDTO;

// 用途：导入登录DTO
// 说明：用于接收登录请求参数
use crate::domain::dto::basic::sign_in::SignInDTO;

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入统一API响应
// 说明：用于OpenAPI文档生成
use crate::domain::vo::response::ApiResponse;

// 用途：导入JWT令牌VO
// 说明：用于返回认证结果
use crate::domain::vo::JWTToken;

// 用途：导入登录VO
// 说明：用于返回登录结果
use crate::domain::vo::basic::LoginVO;

// 用途：导入刷新Token响应VO
// 说明：用于返回刷新Token结果
// use crate::domain::vo::basic::RefreshTokenVO;

// 用途：导入axum的Json提取器
// 说明：用于从HTTP请求体中提取JSON数据
use axum::Json;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

// 用途：导入axum的Request提取器
// 说明：用于从HTTP请求中获取信息，如头信息
use axum::extract::Request;

// 用途：导入令牌密钥常量
// 说明：用于从请求头中获取令牌
use crate::middleware::auth_axum::TOKEN_KEY;

// 用途：导入JwtAuth提取器
// 说明：用于从请求中提取JWT令牌
use crate::middleware::auth_axum::JwtAuth;

/// 用途：用户登录
/// 说明：验证用户身份并返回访问令牌
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = SignInDTO,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<LoginVO>),
        (status = 401, description = "登录失败", body = ApiResponse<LoginVO>)
    ),
    tag = "auth"
)]
pub async fn login(arg: Json<SignInDTO>) -> impl IntoResponse {
    // 用途：调用系统用户服务进行登录验证
    // 说明：验证用户账号和密码，返回用户信息和令牌
    let result = CONTEXT.sys_user_service.sign_in(&arg.0).await;
    // 用途：处理登录结果
    // 说明：成功时返回访问令牌，失败时返回401状态码
    use axum::http::StatusCode;
    match result {
        Ok(sign_in_vo) => {
            // 用途：构建登录响应
            // 说明：返回符合Vben前端期望的响应格式
            let vo = LoginVO {
                access_token: sign_in_vo.access_token,
            };
            (StatusCode::OK, axum::Json(RespVO::from(vo)))
        }
        Err(e) => {
            // 用途：返回登录失败错误
            // 说明：登录失败时返回401状态码
            let resp_vo = RespVO::<LoginVO>::from_error(e.to_string());
            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
        }
    }
}

/// 用途：检查令牌和路径是否有效且可访问
/// 说明：验证用户是否有权限访问指定路径
#[utoipa::path(
    post,
    path = "/api/v1/auth/check",
    request_body = SysAuthDTO,
    responses(
        (status = 200, description = "认证检查成功", body = ApiResponse<bool>),
        (status = 401, description = "认证失败", body = ApiResponse<bool>)
    ),
    tag = "auth"
)]
pub async fn check(arg: Json<SysAuthDTO>) -> impl IntoResponse {
    // 用途：调用系统认证服务检查权限
    // 说明：验证用户令牌和请求路径的权限关系
    let result = CONTEXT.sys_auth_service.check_auth(arg.0).await;
    // 用途：处理认证检查结果
    // 说明：成功时返回200状态码，失败时返回401状态码
    use axum::http::StatusCode;
    match result {
        Ok(_) => (StatusCode::OK, axum::Json(RespVO::from(true))),
        Err(e) => {
            // 用途：返回401状态码
            // 说明：认证失败时返回401状态码
            let resp_vo = RespVO::<bool>::from_error(e.to_string());
            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
        }
    }
}

/// 用途：刷新Token
/// 说明：验证当前access_token并延长有效期，返回符合Vben前端期望的格式
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    responses(
        (status = 200, description = "Token刷新成功", body = ApiResponse<String>),
        (status = 401, description = "Token无效或已过期", body = ApiResponse<String>)
    ),
    tag = "auth"
)]
pub async fn refresh_token(req: Request) -> impl IntoResponse {
    // 用途：从请求头中获取令牌
    // 说明：JWT令牌存储在请求头中，用于身份验证
    let token = req.headers().get(TOKEN_KEY);
    // 用途：匹配令牌是否存在
    // 说明：处理令牌存在和不存在的情况
    use axum::http::StatusCode;
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
            let jwt = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            // 用途：检查令牌验证结果
            // 说明：如果令牌无效，返回401状态码
            match jwt {
                Ok(jwt_token) => {
                    // 用途：延长令牌有效期
                    // 说明：生成新的令牌，延长有效期
                    let new_token = jwt_token.refresh(&CONTEXT.config.jwt_secret, CONTEXT.config.jwt_exp);
                    // 用途：检查令牌生成结果
                    // 说明：如果生成失败，返回错误信息
                    match new_token {
                        Ok(access_token) => {
                            // 用途：返回新的access_token
                            // 说明：符合Vben前端期望的响应格式，data字段为字符串
                            (StatusCode::OK, axum::Json(RespVO::from(access_token)))
                        }
                        Err(e) => {
                            // 用途：返回令牌生成失败错误
                            // 说明：令牌生成过程中出现错误
                            let resp_vo = RespVO::<String>::from_error(e.to_string());
                            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
                        }
                    }
                }
                Err(e) => {
                    // 用途：返回令牌验证失败错误
                    // 说明：令牌无效或已过期，返回401状态码
                    let resp_vo = RespVO::<String>::from_error(e.to_string());
                    (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
                }
            }
        }
        _ => {
            // 用途：返回令牌为空的错误
            // 说明：令牌是刷新Token的必要条件，返回401状态码
            let resp_vo = RespVO::<String>::from_error("access_token_empty".to_string());
            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
        }
    }
}

/// 用途：退出登录
/// 说明：清除用户登录状态
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "退出登录成功", body = ApiResponse<String>),
        (status = 401, description = "Token无效", body = ApiResponse<String>)
    ),
    tag = "auth"
)]
pub async fn logout(req: Request) -> impl IntoResponse {
    // 用途：从请求头中获取令牌
    // 说明：JWT令牌存储在请求头中，用于身份验证
    let token = req.headers().get(TOKEN_KEY);
    // 用途：匹配令牌是否存在
    // 说明：处理令牌存在和不存在的情况
    use axum::http::StatusCode;
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
            let jwt = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            // 用途：检查令牌验证结果
            // 说明：如果令牌无效，返回401状态码
            match jwt {
                Ok(_) => {
                    // 用途：清除用户登录状态
                    // 说明：如果使用Redis缓存，清除Token缓存
                    // 当前使用JWT无状态认证，直接返回成功
                    // 用途：返回响应
                    // 说明：统一响应格式，包含状态码、消息和数据
                    (StatusCode::OK, axum::Json(RespVO::from("logout_success".to_string())))
                }
                Err(e) => {
                    // 用途：返回令牌验证失败错误
                    // 说明：令牌无效或已过期，返回401状态码
                    let resp_vo = RespVO::<String>::from_error(e.to_string());
                    (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
                }
            }
        }
        _ => {
            // 用途：返回令牌为空的错误
            // 说明：令牌是退出登录的必要条件，返回401状态码
            let resp_vo = RespVO::<String>::from_error("access_token_empty".to_string());
            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
        }
    }
}

/// 用途：获取用户权限码
/// 说明：从JWT令牌中提取用户权限码，返回符合Vben前端期望的格式
#[utoipa::path(
    get,
    path = "/api/v1/auth/codes",
    responses(
        (status = 200, description = "获取权限码成功", body = ApiResponse<Vec<String>>),
        (status = 401, description = "Token无效", body = ApiResponse<Vec<String>>)
    ),
    tag = "auth"
)]
pub async fn get_codes(jwt_auth: JwtAuth) -> impl IntoResponse {
    // 用途：从JWT令牌中提取权限码
    // 说明：JwtAuth提取器已验证令牌有效性，直接使用
    let permissions = jwt_auth.permissions.clone();
    // 用途：返回权限码列表
    // 说明：符合Vben前端期望的响应格式，data字段为字符串数组
    use axum::http::StatusCode;
    (StatusCode::OK, axum::Json(RespVO::from(permissions)))
}
