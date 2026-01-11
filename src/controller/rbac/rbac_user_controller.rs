use crate::context::CONTEXT;
use crate::domain::dto::{IdDTO, SignInDTO, UserAddDTO, UserEditDTO, UserRolePageDTO};
use crate::domain::vo::response::{ApiResponse, PageWrapper};
use crate::domain::vo::{JWTToken, SignInVO};
use crate::middleware::auth_axum::TOKEN_KEY;
use axum::Json;
use axum::extract::Request;
use axum::response::IntoResponse;

#[utoipa::path(
    post,
    path = "/user/login",
    request_body = SignInDTO,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<SignInVO>),
        (status = 401, description = "登录失败", body = ApiResponse<SignInVO>)
    ),
    tag = "user"
)]
pub async fn login(arg: Json<SignInDTO>) -> impl IntoResponse {
    log::info!("login:{:?}", arg.0);
    let result = CONTEXT.sys_user_service.sign_in(&arg.0).await;
    use axum::http::StatusCode;
    match result {
        Ok(vo) => (StatusCode::OK, axum::Json(ApiResponse::success(vo))),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/user/info",
    responses(
        (status = 200, description = "获取成功", body = ApiResponse<SignInVO>),
        (status = 401, description = "认证失败", body = ApiResponse<SignInVO>)
    ),
    tag = "user"
)]
pub async fn info(req: Request) -> impl IntoResponse {
    use axum::http::StatusCode;
    let token = req.headers().get(TOKEN_KEY);
    match token {
        Some(token) => {
            let token = token.to_str().unwrap_or("");
            let token = token.trim_start_matches("Bearer ");
            let token = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            if let Err(e) = &token {
                return (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(ApiResponse::error("-1", &e.to_string())),
                );
            }
            let user_data = CONTEXT
                .sys_user_service
                .get_user_info_by_token(&token.unwrap_or_default())
                .await;
            match user_data {
                Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(ApiResponse::error("-1", &e.to_string())),
                ),
            }
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            axum::Json(ApiResponse::error("-1", "缺少访问令牌，请先登录")),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/user/add",
    request_body = UserAddDTO,
    responses(
        (status = 200, description = "添加成功", body = ApiResponse<u64>),
        (status = 400, description = "添加失败", body = ApiResponse<u64>)
    ),
    tag = "user"
)]
pub async fn add(arg: Json<UserAddDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let vo = CONTEXT.sys_user_service.add(arg.0).await;
    match vo {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/user/page",
    request_body = UserRolePageDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PageWrapper<crate::domain::vo::SysUserVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<PageWrapper<crate::domain::vo::SysUserVO>>)
    ),
    tag = "user"
)]
pub async fn page(arg: Json<UserRolePageDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let vo = CONTEXT.sys_user_service.role_page(&arg.0).await;
    let wrapper_data: Result<PageWrapper<crate::domain::vo::SysUserVO>, _> =
        vo.map(|p| PageWrapper {
            page: p.page_no,
            page_size: p.page_size,
            total: p.total,
            records: p.records,
        });
    match wrapper_data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/user/detail",
    request_body = IdDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<crate::domain::vo::SysUserVO>),
        (status = 400, description = "查询失败", body = ApiResponse<crate::domain::vo::SysUserVO>)
    ),
    tag = "user"
)]
pub async fn detail(arg: Json<IdDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let vo = CONTEXT.sys_user_service.detail(&arg.0).await;
    match vo {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/user/update",
    request_body = UserEditDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<u64>),
        (status = 400, description = "更新失败", body = ApiResponse<u64>)
    ),
    tag = "user"
)]
pub async fn update(arg: Json<UserEditDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    if let (Some(user_id), Some(state)) = (arg.0.id.as_ref(), arg.0.state.as_ref())
        && *state == 0
        && let Ok(is_admin) = CONTEXT.sys_user_service.is_super_admin(user_id).await
        && is_admin
    {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", "cannot_disable_admin")),
        );
    }
    let vo = CONTEXT.sys_user_service.edit(arg.0).await;
    match vo {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/user/remove",
    request_body = IdDTO,
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<u64>),
        (status = 400, description = "删除失败", body = ApiResponse<u64>)
    ),
    tag = "user"
)]
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let vo = CONTEXT
        .sys_user_service
        .remove(&arg.0.id.unwrap_or_default())
        .await;
    match vo {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}
