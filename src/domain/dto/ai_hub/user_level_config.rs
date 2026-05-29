//! 用户等级配置相关 DTO
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

/// 用户等级配置视图对象
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "id": "123",
    "level": "vip",
    "level_name": "VIP用户",
    "description": "VIP用户等级",
    "rpm_limit": 100,
    "tpm_limit": 1000,
    "enabled": 1,
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
}))]
pub struct UserLevelConfigVO {
    pub id: String,
    pub level: String,
    pub level_name: String,
    pub description: Option<String>,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
    pub enabled: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 用户等级详情视图对象（包含模型限制）
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

/// 当前用户等级信息视图对象（用于前端显示）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserLevelVO {
    pub level: i32,
    pub level_name: String,
    pub max_requests_per_minute: i32,
    pub max_tokens_per_minute: i32,
    pub description: String,
}
