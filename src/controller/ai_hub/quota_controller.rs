//! 配额管理控制器模块
//! 提供用户配额查询、充值、分配等API接口

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::vo::ai_hub::user_quota::{AiHubUserQuotaVO, QuotaOverviewVO};
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};
use crate::middleware::auth::checked_token;
use crate::service::ai_hub::{CreateQuotaDTO, ListQuotasResponse, QuotaQueryDTO, UpdateQuotaDTO};

use axum::debug_handler;

/// 查询用户配额概览
///
/// 获取指定用户的配额使用情况，包括总配额、已用配额、剩余配额等
#[utoipa::path(
    get,
    path = "/quota/overview",
    params(
        ("user_id" = String, Query, description = "用户ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = AiHubUserQuotaVO),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_quota_overview(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<QuotaQueryDTO>,
) -> Result<Json<QuotaOverviewVO>> {
    // 从请求头获取用户信息（简化实现，实际应从JWT解析）
    let _user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default_user");

    let user_id = params.user_id.as_deref().unwrap_or(_user_id);
    let quota = state.quota_service.get_overview(user_id).await?;
    Ok(Json(quota))
}

/// 创建用户配额
///
/// 为新用户创建配额记录
#[utoipa::path(
    post,
    path = "/quota",
    request_body = CreateQuotaDTO,
    responses(
        (status = 201, description = "创建成功", body = AiHubUserQuotaVO),
        (status = 400, description = "参数错误"),
        (status = 409, description = "配额已存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn create_quota(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<CreateQuotaDTO>,
) -> Result<Json<AiHubUserQuotaVO>> {
    let quota_id = state.quota_service.create_quota(dto).await?;
    let quota = state.quota_service.get_quota(&quota_id).await?;
    Ok(Json(quota))
}

/// 更新用户配额
///
/// 更新用户的配额配置
#[utoipa::path(
    put,
    path = "/quota",
    request_body = UpdateQuotaDTO,
    responses(
        (status = 200, description = "更新成功", body = AiHubUserQuotaVO),
        (status = 400, description = "参数错误"),
        (status = 404, description = "配额不存在"),
        (status = 403, description = "权限不足"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn update_quota(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<UpdateQuotaDTO>,
) -> Result<Json<AiHubUserQuotaVO>> {
    const TOKEN_KEY: &str = "Authorization";

    let _current_user_id = if let Some(auth_header) = headers.get(TOKEN_KEY) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ");
            if let Ok(jwt_token) = checked_token(token) {
                jwt_token.id
            } else {
                return Err(Error::BusinessError("无效的访问令牌".to_string()));
            }
        } else {
            return Err(Error::BusinessError("缺少访问令牌".to_string()));
        }
    } else {
        return Err(Error::BusinessError("缺少访问令牌".to_string()));
    };

    let _quota = state.quota_service.get_quota(&dto.id).await?;

    state
        .quota_service
        .update_quota(&dto.id, dto.clone())
        .await?;
    let updated_quota = state.quota_service.get_quota(&dto.id).await?;
    Ok(Json(updated_quota))
}

/// 列出配额
///
/// 管理员查询用户配额列表
#[utoipa::path(
    get,
    path = "/admin/quota",
    params(
        ("user_id" = Option<String>, Query, description = "用户ID"),
        ("quota_type" = Option<String>, Query, description = "配额类型"),
        ("status" = Option<String>, Query, description = "配额状态"),
        ("include_expired" = Option<bool>, Query, description = "是否包含已过期配额"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListQuotasResponse>),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn list_admin_quotas(
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<QuotaQueryDTO>,
) -> Result<Json<ApiResponse<ListQuotasResponse>>> {
    let quotas = state.quota_service.list_quotas(params).await?;
    Ok(Json(ApiResponse::success(quotas)))
}

/// 获取配额详情
///
/// 管理员查询单个用户配额详情
#[utoipa::path(
    get,
    path = "/admin/quota/{id}",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<AiHubUserQuotaVO>),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "配额不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_admin_quota(
    State(state): State<Arc<ServiceContext>>,
    id: axum::extract::Path<String>,
) -> Result<Json<ApiResponse<AiHubUserQuotaVO>>> {
    let quota = state.quota_service.get_quota(&id).await?;
    Ok(Json(ApiResponse::success(quota)))
}

// 删除配额
//
// 管理员删除用户配额
// #[utoipa::path(
//     delete,
//     path = "/api/v1/admin/quota/{id}",
//     responses(
//         (status = 204, description = "删除成功"),
//         (status = 400, description = "参数错误"),
//         (status = 401, description = "未授权"),
//         (status = 403, description = "权限不足"),
//         (status = 404, description = "配额不存在"),
//         (status = 500, description = "服务器错误")
//     ),
//     security(
//         ("api_key" = [])
//     )
// )]
// #[debug_handler]
// pub async fn delete_admin_quota(
//     State(state): State<Arc<ServiceContext>>,
//     id: axum::extract::Path<String>,
// ) -> Result<axum::http::StatusCode> {
//     state.quota_service.delete_quota(&id).await?;
//     Ok(axum::http::StatusCode::NO_CONTENT)
// }
