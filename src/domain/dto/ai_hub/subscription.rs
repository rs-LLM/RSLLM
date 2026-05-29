use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPlanItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub price: f64,
    pub billing_cycle: String,
    pub currency: String,
    pub enabled: bool,
    pub user_level: Option<String>,
    pub quota_total: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPlansResponse {
    pub plans: Vec<SubscriptionPlanItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PurchaseSubscriptionRequest {
    pub plan_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PurchaseSubscriptionResponse {
    pub transaction_id: String,
    pub plan_id: String,
    pub amount: f64,
    pub currency: String,
    pub balance_after: f64,
    pub subscription: UserSubscriptionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserSubscriptionStatus {
    pub user_id: String,
    pub plan_id: String,
    pub plan_name: String,
    pub billing_cycle: String,
    pub currency: String,
    pub amount: f64,
    pub status: String,
    pub started_at: String,
    pub expires_at: String,
    pub transaction_id: String,
    #[serde(default)]
    pub quota_total: f64,
    #[serde(default)]
    pub quota_used: f64,
    #[serde(default = "default_quota_reset_cycle")]
    pub quota_reset_cycle: String,
    pub next_quota_reset_at: Option<String>,
}

fn default_quota_reset_cycle() -> String {
    "never".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SelfSubscriptionResponse {
    pub subscription: Option<UserSubscriptionStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSubscriptionPaymentOrderRequest {
    pub plan_id: String,
    pub provider: Option<String>,
    /// 是否余额优先。
    /// - true: 先尝试扣减钱包余额；余额不足时可按配置创建补差订单
    /// - false: 直接创建第三方支付订单
    pub wallet_first: Option<bool>,
    /// 指定本次优先使用的钱包金额（可选）。不传时默认尽可能使用余额。
    pub wallet_amount: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPaymentOrder {
    pub order_id: String,
    pub user_id: String,
    pub plan_id: String,
    pub plan_name: String,
    pub provider: String,
    pub amount: f64,
    pub currency: String,
    pub status: String,
    pub pay_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub confirmed_at: Option<String>,
    pub external_txn_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPaymentOrdersResponse {
    pub orders: Vec<SubscriptionPaymentOrder>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPaymentProviderItem {
    pub provider: String,
    pub display_name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPaymentProvidersResponse {
    pub providers: Vec<SubscriptionPaymentProviderItem>,
    pub default_provider: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfirmSubscriptionPaymentCallbackRequest {
    pub order_id: String,
    pub status: String,
    pub provider: Option<String>,
    pub external_txn_id: Option<String>,
    pub callback_token: Option<String>,
    pub event_id: Option<String>,
    pub timestamp: Option<i64>,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ConfirmSubscriptionPaymentCallbackResponse {
    pub order: SubscriptionPaymentOrder,
    pub subscription: Option<UserSubscriptionStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateSubscriptionPlansRequest {
    pub plans: Vec<SubscriptionPlanItem>,
}
