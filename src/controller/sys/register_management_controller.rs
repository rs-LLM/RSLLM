use std::sync::Arc;

use axum::{Json, extract::State};
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::context::ServiceContext;
use crate::domain::dto::basic::register::UserRegisterDTO;
use crate::domain::table::invitation_code::InvitationCode;
use crate::domain::table::registration_review::RegistrationReview;
use crate::domain::vo::response::ApiResponse;
use crate::error::{ApplicationError, ApplicationResult};
use crate::middleware::auth_axum::JwtAuth;
use crate::service::sys::RegisterPolicyService;

#[derive(Clone, Debug, Deserialize)]
pub struct GenerateInvitationCodesRequest {
    pub quantity: Option<u32>,
    #[serde(rename = "maxUses")]
    pub max_uses: Option<i64>,
    #[serde(rename = "userLevel")]
    pub user_level: Option<String>,
    pub remark: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QueryInvitationCodesRequest {
    pub code: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateInvitationCodeRequest {
    pub id: String,
    pub status: String,
    pub remark: Option<String>,
    #[serde(rename = "maxUses")]
    pub max_uses: Option<i64>,
    #[serde(rename = "userLevel")]
    pub user_level: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QueryRegistrationReviewsRequest {
    pub status: Option<String>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReviewRegistrationRequest {
    pub id: String,
    pub status: String,
    pub note: Option<String>,
    #[serde(rename = "sendEmail")]
    pub send_email: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PagedInvitationCodesResponse {
    pub total: usize,
    pub items: Vec<InvitationCode>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PagedRegistrationReviewsResponse {
    pub total: usize,
    pub items: Vec<RegistrationReview>,
}

fn generate_invitation_code() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    let mut code = String::with_capacity(10);
    for _ in 0..10 {
        let idx = rng.random_range(0..CHARS.len());
        code.push(CHARS[idx] as char);
    }
    code
}

pub async fn generate_invitation_codes(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<GenerateInvitationCodesRequest>,
) -> ApplicationResult<Json<ApiResponse<Vec<InvitationCode>>>> {
    let quantity = req.quantity.unwrap_or(1).clamp(1, 200) as usize;
    let max_uses = req.max_uses.unwrap_or(1).max(1);
    let mut created = Vec::with_capacity(quantity);
    for _ in 0..quantity {
        let mut code = generate_invitation_code();
        loop {
            let exists = InvitationCode::select_by_map(
                context.rb.as_ref(),
                rbs::value! {"code": code.clone()},
            )
            .await?;
            if exists.is_empty() {
                break;
            }
            code = generate_invitation_code();
        }
        let now = DateTime::now();
        let item = InvitationCode {
            id: Some(Ulid::new().to_string()),
            code,
            status: "active".to_string(),
            max_uses,
            used_count: 0,
            user_level: req.user_level.clone(),
            operator_id: Some(jwt_auth.id.clone()),
            remark: req.remark.clone(),
            created_at: Some(now.clone()),
            updated_at: Some(now),
        };
        InvitationCode::insert(context.rb.as_ref(), &item).await?;
        created.push(item);
    }
    Ok(Json(ApiResponse::success(created)))
}

pub async fn list_invitation_codes(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<QueryInvitationCodesRequest>,
) -> ApplicationResult<Json<ApiResponse<PagedInvitationCodesResponse>>> {
    let mut items = InvitationCode::select_all(context.rb.as_ref()).await?;
    let code_filter = req.code.unwrap_or_default().trim().to_uppercase();
    let status_filter = req.status.unwrap_or_default().trim().to_lowercase();
    items.retain(|item| {
        let mut matched = true;
        if !code_filter.is_empty() {
            matched = matched && item.code.to_uppercase().contains(&code_filter);
        }
        if !status_filter.is_empty() {
            matched = matched && item.status.to_lowercase() == status_filter;
        }
        matched
    });
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let total = items.len();
    let page = req.page.unwrap_or(1).max(1) as usize;
    let page_size = req.page_size.unwrap_or(20).clamp(1, 200) as usize;
    let start = (page - 1) * page_size;
    let end = std::cmp::min(start + page_size, total);
    let paged = if start >= total {
        Vec::new()
    } else {
        items[start..end].to_vec()
    };
    Ok(Json(ApiResponse::success(PagedInvitationCodesResponse {
        total,
        items: paged,
    })))
}

pub async fn update_invitation_code(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<UpdateInvitationCodeRequest>,
) -> ApplicationResult<Json<ApiResponse<InvitationCode>>> {
    let mut items =
        InvitationCode::select_by_map(context.rb.as_ref(), rbs::value! {"id": req.id.clone()})
            .await?;
    let mut item = items
        .drain(..)
        .next()
        .ok_or_else(|| ApplicationError::NotFound {
            message: "邀请码不存在".to_string(),
            resource: Some("invitation_code".to_string()),
            id: Some(req.id.clone()),
        })?;

    let status = req.status.trim().to_lowercase();
    if status != "active" && status != "disabled" && status != "used" {
        return Err(ApplicationError::ValidationError {
            message: "邀请码状态仅支持 active/disabled/used".to_string(),
            field: Some("status".to_string()),
            value: Some(req.status),
        });
    }

    item.status = status;
    if let Some(max_uses) = req.max_uses {
        if max_uses < item.used_count {
            return Err(ApplicationError::ValidationError {
                message: "maxUses 不能小于已使用次数".to_string(),
                field: Some("maxUses".to_string()),
                value: Some(max_uses.to_string()),
            });
        }
        item.max_uses = max_uses;
    }
    item.user_level = req.user_level;
    item.operator_id = Some(jwt_auth.id.clone());
    item.remark = req.remark;
    item.updated_at = Some(DateTime::now());
    InvitationCode::update_by_map(context.rb.as_ref(), &item, rbs::value! {"id": req.id}).await?;
    Ok(Json(ApiResponse::success(item)))
}

pub async fn list_registration_reviews(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<QueryRegistrationReviewsRequest>,
) -> ApplicationResult<Json<ApiResponse<PagedRegistrationReviewsResponse>>> {
    let mut items = RegistrationReview::select_all(context.rb.as_ref()).await?;
    let status_filter = req.status.unwrap_or_default().trim().to_lowercase();
    if !status_filter.is_empty() {
        items.retain(|item| item.status.to_lowercase() == status_filter);
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let total = items.len();
    let page = req.page.unwrap_or(1).max(1) as usize;
    let page_size = req.page_size.unwrap_or(20).clamp(1, 200) as usize;
    let start = (page - 1) * page_size;
    let end = std::cmp::min(start + page_size, total);
    let paged = if start >= total {
        Vec::new()
    } else {
        items[start..end].to_vec()
    };
    Ok(Json(ApiResponse::success(
        PagedRegistrationReviewsResponse {
            total,
            items: paged,
        },
    )))
}

pub async fn review_registration(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<ReviewRegistrationRequest>,
) -> ApplicationResult<Json<ApiResponse<serde_json::Value>>> {
    let mut items =
        RegistrationReview::select_by_map(context.rb.as_ref(), rbs::value! {"id": req.id.clone()})
            .await?;
    let mut review = items
        .drain(..)
        .next()
        .ok_or_else(|| ApplicationError::NotFound {
            message: "注册申请不存在".to_string(),
            resource: Some("registration_review".to_string()),
            id: Some(req.id.clone()),
        })?;

    if review.status != "pending" {
        return Err(ApplicationError::ValidationError {
            message: "该申请已处理".to_string(),
            field: Some("status".to_string()),
            value: Some(review.status),
        });
    }

    let status = req.status.trim().to_lowercase();
    if status != "approved" && status != "rejected" {
        return Err(ApplicationError::ValidationError {
            message: "status 仅支持 approved/rejected".to_string(),
            field: Some("status".to_string()),
            value: Some(req.status),
        });
    }

    if status == "approved" {
        if let Some(code) = &review.invite_code {
            RegisterPolicyService::consume_invite_code(
                context.rb.as_ref(),
                code,
                Some(jwt_auth.id.clone()),
            )
            .await?;
        }
        let register_result = context
            .sys_user_service
            .register(&UserRegisterDTO {
                name: review.name.clone(),
                email: review.email.clone(),
                password: review.password.clone(),
                user_level: review.user_level.clone(),
                agree_terms: review.agree_terms,
            })
            .await?;
        if !register_result.success {
            return Err(ApplicationError::ValidationError {
                message: register_result.message,
                field: Some("register".to_string()),
                value: Some(review.email.clone()),
            });
        }
        review.status = "approved".to_string();
        review.reviewed_by = Some(jwt_auth.id.clone());
        review.review_note = req.note.clone();
        review.reviewed_at = Some(DateTime::now());
        review.updated_at = Some(DateTime::now());
        RegistrationReview::update_by_map(
            context.rb.as_ref(),
            &review,
            rbs::value! {"id": req.id.clone()},
        )
        .await?;

        let _ = RegisterPolicyService::send_mail(
            context.rb.as_ref(),
            &review.email,
            "RSLLM 注册审核通过",
            "您的注册申请已审核通过，现在可以直接登录系统。",
        )
        .await;

        return Ok(Json(ApiResponse::success(serde_json::json!({
            "status": "approved"
        }))));
    }

    review.status = "rejected".to_string();
    review.reviewed_by = Some(jwt_auth.id.clone());
    review.review_note = req.note.clone();
    review.reviewed_at = Some(DateTime::now());
    review.updated_at = Some(DateTime::now());
    RegistrationReview::update_by_map(
        context.rb.as_ref(),
        &review,
        rbs::value! {"id": req.id.clone()},
    )
    .await?;

    let policy = RegisterPolicyService::get_policy(context.rb.as_ref()).await;
    let should_send = req
        .send_email
        .unwrap_or(policy.register_review_reject_notify_enabled);
    if should_send {
        let _ = RegisterPolicyService::send_mail(
            context.rb.as_ref(),
            &review.email,
            "RSLLM 注册审核未通过",
            &format!(
                "您的注册申请未通过审核。{}",
                req.note
                    .as_ref()
                    .map(|v| format!(" 审核说明：{}", v))
                    .unwrap_or_default()
            ),
        )
        .await;
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "status": "rejected"
    }))))
}
