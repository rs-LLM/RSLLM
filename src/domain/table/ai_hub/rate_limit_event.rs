use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitEvent {
    pub id: Option<String>,
    pub user_id: String,
    pub api_key_id: Option<String>,
    pub limit_type: String,
    pub limit_value: i32,
    pub current_value: i32,
    pub ip_address: Option<String>,
    pub request_id: Option<String>,
    pub create_time: DateTime,
}

crud!(RateLimitEvent {});
