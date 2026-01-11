//! 用户等级配置相关 DTO
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 创建用户等级 DTO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateUserLevelDTO {
    pub level: String,
    pub level_name: String,
    pub description: Option<String>,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub rpm_burst: Option<i32>,
    pub tpm_burst: Option<i32>,
    pub enabled: i32,
}

/// 更新用户等级 DTO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateUserLevelConfigDTO {
    pub id: String,
    pub level_name: Option<String>,
    pub description: Option<String>,
    pub rpm_limit: Option<i32>,
    pub tpm_limit: Option<i32>,
    pub enabled: Option<i32>,
}

/// 用户等级配置 VO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserLevelConfigVO {
    pub id: String,
    pub level: String,
    pub level_name: String,
    pub description: Option<String>,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub enabled: i32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

/// 用户等级详情 VO（包含模型限制）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserLevelDetailVO {
    pub config: UserLevelConfigVO,
    pub model_limits:
        Vec<crate::domain::dto::ai_hub::user_level_model_limit::UserLevelModelLimitVO>,
}

/// 用户等级列表查询参数
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QueryUserLevelsDTO {
    pub enabled: Option<i32>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 用户等级列表响应
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserLevelListResponse {
    pub items: Vec<UserLevelConfigVO>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// 获取所有用户等级响应（不分页）
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ListUserLevelsResponse {
    pub items: Vec<UserLevelConfigVO>,
    pub total: i64,
}

/// 批量更新用户等级状态 DTO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BatchUpdateUserLevelStatusDTO {
    pub ids: Vec<String>,
    pub enabled: i32,
}

/// 批量删除用户等级 DTO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BatchDeleteUserLevelsDTO {
    pub ids: Vec<String>,
}

/// 当前用户等级信息 VO（用于前端显示）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserLevelVO {
    pub level: i32,
    pub level_name: String,
    pub max_requests_per_minute: i32,
    pub max_tokens_per_minute: i32,
    pub description: String,
}
