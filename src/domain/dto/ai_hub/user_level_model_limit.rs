use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateUserLevelModelLimitDTO {
    pub user_level: String,
    pub model_key: String,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateUserLevelModelLimitDTO {
    pub id: String,
    pub rpm_limit: Option<i32>,
    pub tpm_limit: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserLevelModelLimitQueryDTO {
    pub user_level: Option<String>,
    pub model_key: Option<String>,
    pub enabled: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserLevelModelLimitVO {
    pub id: String,
    pub user_level: String,
    pub model_key: String,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BatchCreateUserLevelModelLimitDTO {
    pub user_level: String,
    pub limits: Vec<ModelLimitItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ModelLimitItem {
    pub model_key: String,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ListUserLevelModelLimitsResponse {
    pub items: Vec<UserLevelModelLimitVO>,
    pub total: i64,
}
