use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::UsageStatisticsQueryDTO;
use crate::domain::dto::ai_hub::dashboard_billing::{
    DashboardBillingUsageQuery, OpenAISubscriptionResponse, OpenAIUsageResponse,
};
use crate::error::{Error, Result};

fn get_user_id_from_headers(headers: &HeaderMap) -> Result<String> {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.trim().is_empty())
        .map(std::borrow::ToOwned::to_owned)
        .ok_or_else(|| Error::AuthError("Missing user identity".to_string()))
}

fn parse_access_until(expires_at: Option<String>) -> i64 {
    expires_at
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

pub async fn get_dashboard_billing_subscription(
    headers: HeaderMap,
    State(ctx): State<Arc<ServiceContext>>,
) -> Result<Json<OpenAISubscriptionResponse>> {
    let user_id = get_user_id_from_headers(&headers)?;
    let balance = ctx.balance_service.get_balance(&user_id).await?;

    let total_limit =
        (balance.balance.max(0.0) + balance.subscription_quota_total.max(0.0)).max(0.0);

    Ok(Json(OpenAISubscriptionResponse {
        object: "billing_subscription".to_string(),
        has_payment_method: true,
        soft_limit_usd: total_limit,
        hard_limit_usd: total_limit,
        system_hard_limit_usd: total_limit,
        access_until: parse_access_until(balance.subscription_expires_at),
    }))
}

pub async fn get_dashboard_billing_usage(
    headers: HeaderMap,
    State(ctx): State<Arc<ServiceContext>>,
    Query(query): Query<DashboardBillingUsageQuery>,
) -> Result<Json<OpenAIUsageResponse>> {
    let user_id = get_user_id_from_headers(&headers)?;

    let usage = ctx
        .usage_log_service
        .get_usage_statistics(UsageStatisticsQueryDTO {
            user_id: Some(user_id),
            period: None,
            start_time: query.start_date,
            end_time: query.end_date,
            model_ids: None,
            provider_ids: None,
        })
        .await
        .map_err(Error::from)?;

    Ok(Json(OpenAIUsageResponse {
        object: "list".to_string(),
        total_usage: usage.total_cost * 100.0,
    }))
}
