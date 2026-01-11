use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserLevelConfig {
    pub id: Option<String>,
    pub level: String,
    pub level_name: String,
    pub description: Option<String>,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub enabled: i32,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

crud!(UserLevelConfig {});
