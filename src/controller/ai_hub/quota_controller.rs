//! 配额管理控制器模块
//! 提供用户配额查询、充值、分配等API接口

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::error::{Error, Result};
use crate::service::ai_hub::{CreateQuotaDTO, QuotaQueryDTO, RechargeQuotaDTO, UpdateQuotaDTO, AllocateQuotaDTO};
use crate::domain::vo::ai_hub::user_quota::{AiHubUserQuotaVO, QuotaOverviewVO};

use axum::debug_handler;

/// 查询用户配额概览
///
/// 获取指定用户的配额使用情况，包括总配额、已用配额、剩余配额等
#[utoipa::path(
    get,
    path = "/api/v1/quota/overview",
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
    path = "/api/v1/quota",
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
    path = "/api/v1/quota",
    request_body = UpdateQuotaDTO,
    responses(
        (status = 200, description = "更新成功", body = AiHubUserQuotaVO),
        (status = 400, description = "参数错误"),
        (status = 404, description = "配额不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn update_quota(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<UpdateQuotaDTO>,
) -> Result<Json<AiHubUserQuotaVO>> {
    state.quota_service.update_quota(&dto.id, dto.clone()).await?;
    let quota = state.quota_service.get_quota(&dto.id).await?;
    Ok(Json(quota))
}

/// 充值用户配额
///
/// 为用户增加配额额度
#[utoipa::path(
    post,
    path = "/api/v1/quota/recharge",
    request_body = RechargeQuotaDTO,
    responses(
        (status = 200, description = "充值成功", body = AiHubUserQuotaVO),
        (status = 400, description = "参数错误"),
        (status = 404, description = "用户不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn recharge_quota(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<RechargeQuotaDTO>,
) -> Result<Json<AiHubUserQuotaVO>> {
    // 需要先获取配额ID，然后调用recharge方法
    // 这里假设recharge方法需要配额ID，需要检查服务方法签名
    // 临时实现：获取用户第一个活跃配额进行充值
    let quotas = state.quota_service.list_quotas(QuotaQueryDTO {
        user_id: Some(dto.user_id.clone()),
        quota_type: None,
        status: Some("active".to_string()),
        include_expired: Some(false),
        page: Some(1),
        page_size: Some(10),
    }).await?;
    
    if quotas.is_empty() {
        return Err(Error::BusinessError("No active quota found".to_string()));
    }
    
    let quota_id = quotas[0].id.clone().ok_or_else(|| Error::BusinessError("Quota ID missing".to_string()))?;
    state.quota_service.recharge(&quota_id, dto).await?;
    let quota = state.quota_service.get_quota(&quota_id).await?;
    Ok(Json(quota))
}

/// 分配配额
///
/// 管理员为用户分配配额
#[utoipa::path(
    post,
    path = "/api/v1/quota/allocate",
    request_body = AllocateQuotaDTO,
    responses(
        (status = 200, description = "分配成功", body = AiHubUserQuotaVO),
        (status = 400, description = "参数错误"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "用户不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn allocate_quota(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<AllocateQuotaDTO>,
) -> Result<Json<AiHubUserQuotaVO>> {
    // 验证管理员权限（简化实现）
    let role = headers
        .get("x-user-role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("user");
    
    if role != "admin" {
        return Err(Error::BusinessError("Insufficient permissions".to_string()));
    }
    
    let quota_id = state.quota_service.allocate_quota(dto).await?;
    let quota = state.quota_service.get_quota(&quota_id).await?;
    Ok(Json(quota))
}