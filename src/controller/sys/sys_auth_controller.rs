// 用途：导入全局上下文
// 说明：用于访问系统认证服务
use crate::context::CONTEXT;

// 用途：导入系统认证DTO
// 说明：用于接收认证检查请求参数
use crate::domain::dto::basic::auth::SysAuthDTO;

// 用途：导入登录DTO
// 说明：用于接收登录请求参数
use crate::domain::dto::basic::sign_in::SignInDTO;

// 用途：导入统一API响应
// 说明：用于OpenAPI文档生成
use crate::domain::vo::response::ApiResponse;

// 用途：导入JWT令牌视图对象
// 说明：用于返回认证结果
use crate::domain::vo::JWTToken;

// 用途：导入登录视图对象
// 说明：用于返回登录结果
use crate::domain::vo::basic::LoginVO;

// 用途：导入2FA控制器
// 说明：用于统一处理登录二次验证响应
use crate::controller::sys::twofa_controller;

// 用途：导入2FA服务
// 说明：用于登录时判定是否需要二次验证
use crate::service::sys::{SignInResult, TwoFaService};

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
    path = "/auth/login",
    request_body = SignInDTO,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<LoginVO>),
        (status = 401, description = "需要2FA挑战或登录失败", body = ApiResponse<crate::controller::sys::twofa_controller::NeedTwoFaResponse>)
    ),
    tag = "auth"
)]
pub async fn login(arg: Json<SignInDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;

    let service = TwoFaService;
    match service.sign_in_or_create_challenge(&arg.0).await {
        Ok(SignInResult::Success(sign_in_vo)) => {
            let vo = LoginVO {
                access_token: sign_in_vo.access_token,
            };
            (StatusCode::OK, axum::Json(ApiResponse::success(vo))).into_response()
        }
        Ok(SignInResult::NeedTwoFa {
            challenge_id,
            expires_in,
        }) => twofa_controller::sign_in_result_to_response(SignInResult::NeedTwoFa {
            challenge_id,
            expires_in,
        })
        .into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            axum::Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        )
            .into_response(),
    }
}

/// 用途：检查令牌和路径是否有效且可访问
/// 说明：验证用户是否有权限访问指定路径
#[utoipa::path(
    post,
    path = "/auth/check",
    request_body = SysAuthDTO,
    responses(
        (status = 200, description = "认证检查成功", body = ApiResponse<bool>),
        (status = 401, description = "认证失败", body = ApiResponse<bool>)
    ),
    tag = "auth"
)]
pub async fn check(arg: Json<SysAuthDTO>) -> impl IntoResponse {
    let result = CONTEXT.sys_auth_service.check_auth(arg.0).await;
    use axum::http::StatusCode;
    match result {
        Ok(_) => (StatusCode::OK, axum::Json(ApiResponse::success(true))),
        Err(e) => {
            let error_msg = e.to_string();
            (
                StatusCode::UNAUTHORIZED,
                axum::Json(ApiResponse::error("-1", &error_msg)),
            )
        }
    }
}

/// 用途：刷新令牌信息
/// 说明：验证当前access_token并延长有效期，返回符合Vben前端期望的格式
#[utoipa::path(
    post,
    path = "/auth/refresh",
    responses(
        (status = 200, description = "刷新令牌信息成功", body = ApiResponse<String>),
        (status = 401, description = "访问令牌无效或已过期", body = ApiResponse<String>)
    ),
    tag = "auth"
)]
pub async fn refresh_token(req: Request) -> impl IntoResponse {
    let token = req.headers().get(TOKEN_KEY);
    use axum::http::StatusCode;
    match token {
        Some(token) => {
            let token = token.to_str().unwrap_or("");
            let token = token.trim_start_matches("Bearer ");
            let jwt = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            match jwt {
                Ok(jwt_token) => {
                    let new_token =
                        jwt_token.refresh(&CONTEXT.config.jwt_secret, CONTEXT.config.jwt_exp);
                    match new_token {
                        Ok(access_token) => (
                            StatusCode::OK,
                            axum::Json(ApiResponse::success(access_token)),
                        ),
                        Err(e) => {
                            let error_msg = e.to_string();
                            (
                                StatusCode::UNAUTHORIZED,
                                axum::Json(ApiResponse::error("-1", &error_msg)),
                            )
                        }
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(ApiResponse::error("-1", &error_msg)),
                    )
                }
            }
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            axum::Json(ApiResponse::error("-1", "access_token_empty")),
        ),
    }
}

/// 用途：退出登录
/// 说明：清除用户登录状态
#[utoipa::path(
    post,
    path = "/auth/logout",
    responses(
        (status = 200, description = "退出登录成功", body = ApiResponse<String>),
        (status = 401, description = "访问令牌无效", body = ApiResponse<String>)
    ),
    tag = "auth"
)]
pub async fn logout(req: Request) -> impl IntoResponse {
    let token = req.headers().get(TOKEN_KEY);
    use axum::http::StatusCode;
    match token {
        Some(token) => {
            let token = token.to_str().unwrap_or("");
            let token = token.trim_start_matches("Bearer ");
            let jwt = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            match jwt {
                Ok(_) => (
                    StatusCode::OK,
                    axum::Json(ApiResponse::success("logout_success".to_string())),
                ),
                Err(e) => {
                    let error_msg = e.to_string();
                    (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(ApiResponse::error("-1", &error_msg)),
                    )
                }
            }
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            axum::Json(ApiResponse::error("-1", "access_token_empty")),
        ),
    }
}

/// 用途：获取用户权限码
/// 说明：从JWT令牌中提取用户权限码，返回符合Vben前端期望的格式
#[utoipa::path(
    get,
    path = "/auth/codes",
    responses(
        (status = 200, description = "获取权限码成功", body = ApiResponse<Vec<String>>),
        (status = 401, description = "访问令牌无效", body = ApiResponse<Vec<String>>)
    ),
    tag = "auth"
)]
pub async fn get_codes(jwt_auth: JwtAuth) -> impl IntoResponse {
    use axum::http::StatusCode;
    match CONTEXT
        .sys_user_service
        .load_level_permission(jwt_auth.id.as_str())
        .await
    {
        Ok(permissions) => (
            StatusCode::OK,
            axum::Json(ApiResponse::success(permissions)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}
