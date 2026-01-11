use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserLevelModelRateLimit {
    pub id: Option<String>,
    pub user_level: String,
    pub model_key: String,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub enabled: bool,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

crud!(UserLevelModelRateLimit {});
