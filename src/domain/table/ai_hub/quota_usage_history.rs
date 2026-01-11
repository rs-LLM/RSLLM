use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuotaUsageHistory {
    pub id: Option<String>,
    pub user_id: String,
    pub api_key_id: Option<String>,
    pub quota_type: String,
    pub used_amount: f64,
    pub remaining_amount: f64,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub create_time: DateTime,
}

crud!(QuotaUsageHistory {});
