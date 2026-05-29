use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardBillingUsageQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OpenAISubscriptionResponse {
    pub object: String,
    pub has_payment_method: bool,
    pub soft_limit_usd: f64,
    pub hard_limit_usd: f64,
    pub system_hard_limit_usd: f64,
    pub access_until: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OpenAIUsageResponse {
    pub object: String,
    pub total_usage: f64,
}
