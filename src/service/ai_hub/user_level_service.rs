//! 用户等级管理服务模块
//! 提供用户等级的查询、创建、更新、删除和配置管理功能
use crate::domain::dto::ai_hub::user_level_config::{
    BatchDeleteUserLevelsDTO, BatchUpdateUserLevelStatusDTO, CreateUserLevelDTO,
    QueryUserLevelsDTO, UpdateUserLevelConfigDTO, UserLevelConfigVO, UserLevelDetailVO,
    UserLevelListResponse, UserLevelVO,
};
use crate::domain::table::ai_hub::user_level_config::UserLevelConfig;
use crate::domain::table::basic::SysUser;
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 更新用户等级DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserLevelDTO {
    pub level: i32,
}

/// 用户等级管理服务
///
/// 负责用户等级的查询、创建、更新、删除和配置管理
#[derive(Clone)]
pub struct UserLevelService {}

impl Default for UserLevelService {
    fn default() -> Self {
        Self::new()
    }
}

impl UserLevelService {
    /// 创建服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 获取所有用户等级配置
    pub async fn get_all_levels(&self) -> ApplicationResult<Vec<UserLevelConfigVO>> {
        let configs = UserLevelConfig::select_all(pool!()).await?;

        let vos: Vec<UserLevelConfigVO> = configs
            .into_iter()
            .filter_map(|config| {
                Some(UserLevelConfigVO {
                    id: config.id?,
                    level: config.level,
                    level_name: config.level_name,
                    description: config.description,
                    rpm_limit: config.rpm_limit,
                    tpm_limit: config.tpm_limit,
                    enabled: config.enabled,
                    created_at: config.created_at?,
                    updated_at: config.updated_at?,
                })
            })
            .collect();

        Ok(vos)
    }

    /// 分页查询用户等级配置
    pub async fn query_levels(
        &self,
        dto: QueryUserLevelsDTO,
    ) -> ApplicationResult<UserLevelListResponse> {
        let page = dto.page.unwrap_or(1);
        let page_size = dto.page_size.unwrap_or(20);
        let offset = (page - 1) * page_size;

        let mut conditions = vec![];
        if let Some(enabled) = dto.enabled {
            conditions.push(format!("enabled = {}", enabled));
        }

        let _where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let configs = UserLevelConfig::select_by_map(
            pool!(),
            rbs::value! {
                "enabled": dto.enabled
            },
        )
        .await?;

        let total = configs.len() as i64;
        let items: Vec<UserLevelConfigVO> = configs
            .into_iter()
            .skip(offset as usize)
            .take(page_size as usize)
            .filter_map(|config| {
                Some(UserLevelConfigVO {
                    id: config.id?,
                    level: config.level,
                    level_name: config.level_name,
                    description: config.description,
                    rpm_limit: config.rpm_limit,
                    tpm_limit: config.tpm_limit,
                    enabled: config.enabled,
                    created_at: config.created_at?,
                    updated_at: config.updated_at?,
                })
            })
            .collect();

        Ok(UserLevelListResponse {
            items,
            total,
            page,
            page_size,
        })
    }

    /// 根据等级代码获取等级配置
    pub async fn get_level_by_code(
        &self,
        level_code: &str,
    ) -> ApplicationResult<UserLevelConfigVO> {
        let config = UserLevelConfig::select_by_map(pool!(), rbs::value! { "level": level_code })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("User level {} not found", level_code),
                resource: Some("user_level".to_string()),
                id: Some(level_code.to_string()),
            })?;

        Ok(UserLevelConfigVO {
            id: config.id.ok_or_else(|| ApplicationError::BusinessError {
                message: "User level config missing id".to_string(),
                code: Some("MISSING_ID".to_string()),
                context: None,
            })?,
            level: config.level,
            level_name: config.level_name,
            description: config.description,
            rpm_limit: config.rpm_limit,
            tpm_limit: config.tpm_limit,
            enabled: config.enabled,
            created_at: config
                .created_at
                .ok_or_else(|| ApplicationError::BusinessError {
                    message: "User level config missing created_at".to_string(),
                    code: Some("MISSING_CREATED_AT".to_string()),
                    context: None,
                })?,
            updated_at: config
                .updated_at
                .ok_or_else(|| ApplicationError::BusinessError {
                    message: "User level config missing updated_at".to_string(),
                    code: Some("MISSING_UPDATED_AT".to_string()),
                    context: None,
                })?,
        })
    }

    /// 获取用户等级详情（包含模型限制）
    pub async fn get_level_detail(&self, level_code: &str) -> ApplicationResult<UserLevelDetailVO> {
        let config = self.get_level_by_code(level_code).await?;

        let model_limits = crate::service::ai_hub::UserLevelModelRateLimitService::new()
            .get_limits_by_level(level_code)
            .await?;

        Ok(UserLevelDetailVO {
            config,
            model_limits,
        })
    }

    /// 获取用户当前等级
    pub async fn get_user_level(&self, user_id: &str) -> ApplicationResult<String> {
        let user = SysUser::select_by_map(pool!(), rbs::value! { "id": user_id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("User {} not found", user_id),
                resource: Some("user".to_string()),
                id: Some(user_id.to_string()),
            })?;

        user.user_level
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "User level not set".to_string(),
                code: Some("USER_LEVEL_NOT_SET".to_string()),
                context: Some("User does not have a level assigned".to_string()),
            })
    }

    /// 获取用户当前等级详细信息
    pub async fn get_user_level_info(&self, user_id: &str) -> ApplicationResult<UserLevelVO> {
        let user = SysUser::select_by_map(pool!(), rbs::value! { "id": user_id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("User {} not found", user_id),
                resource: Some("user".to_string()),
                id: Some(user_id.to_string()),
            })?;

        let user_level_str = user
            .user_level
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "User level not set".to_string(),
                code: Some("USER_LEVEL_NOT_SET".to_string()),
                context: Some("User does not have a level assigned".to_string()),
            })?;

        let configs = UserLevelConfig::select_by_map(
            pool!(),
            rbs::value! { "level": user_level_str.clone() },
        )
        .await?;

        if configs.is_empty() {
            return Err(ApplicationError::NotFound {
                message: format!("User level config not found for level: {}", user_level_str),
                resource: Some("user_level_config".to_string()),
                id: Some(user_level_str),
            });
        }

        let config = &configs[0];

        let level_num: i32 = config.level.trim_start_matches('L').parse().unwrap_or(1);

        Ok(UserLevelVO {
            level: level_num,
            level_name: config.level_name.clone(),
            max_requests_per_minute: config.rpm_limit,
            max_tokens_per_minute: config.tpm_limit,
            description: config.description.clone().unwrap_or_default(),
        })
    }

    /// 创建用户等级
    pub async fn create_level(
        &self,
        dto: CreateUserLevelDTO,
    ) -> ApplicationResult<UserLevelConfigVO> {
        let existing =
            UserLevelConfig::select_by_map(pool!(), rbs::value! { "level": &dto.level }).await?;

        if !existing.is_empty() {
            return Err(ApplicationError::BusinessError {
                message: format!("User level {} already exists", dto.level),
                code: Some("LEVEL_ALREADY_EXISTS".to_string()),
                context: Some("A user level with this code already exists".to_string()),
            });
        }

        let now = DateTime::now();
        let config = UserLevelConfig {
            id: Some(ulid::Ulid::new().to_string()),
            level: dto.level.clone(),
            level_name: dto.level_name,
            description: dto.description,
            rpm_limit: dto.rpm_limit,
            tpm_limit: dto.tpm_limit,
            enabled: dto.enabled,
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };

        let id = config
            .id
            .clone()
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "Failed to generate level ID".to_string(),
                code: Some("LEVEL_ID_GENERATION_FAILED".to_string()),
                context: Some("Failed to generate level ID after successful creation".to_string()),
            })?;

        UserLevelConfig::insert(pool!(), &config).await?;

        Ok(UserLevelConfigVO {
            id,
            level: config.level,
            level_name: config.level_name,
            description: config.description,
            rpm_limit: config.rpm_limit,
            tpm_limit: config.tpm_limit,
            enabled: config.enabled,
            created_at: config
                .created_at
                .ok_or_else(|| ApplicationError::BusinessError {
                    message: "Level missing created_at".to_string(),
                    code: Some("MISSING_CREATED_AT".to_string()),
                    context: None,
                })?,
            updated_at: config
                .updated_at
                .ok_or_else(|| ApplicationError::BusinessError {
                    message: "Level missing updated_at".to_string(),
                    code: Some("MISSING_UPDATED_AT".to_string()),
                    context: None,
                })?,
        })
    }

    /// 更新用户等级配置
    pub async fn update_level(
        &self,
        dto: UpdateUserLevelConfigDTO,
    ) -> ApplicationResult<UserLevelConfigVO> {
        let existing = UserLevelConfig::select_by_map(pool!(), rbs::value! { "id": &dto.id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("User level config {} not found", dto.id),
                resource: Some("user_level_config".to_string()),
                id: Some(dto.id.clone()),
            })?;

        let updated_config = UserLevelConfig {
            id: existing.id,
            level: existing.level,
            level_name: dto.level_name.unwrap_or(existing.level_name),
            description: dto.description.or(existing.description),
            rpm_limit: dto.rpm_limit.unwrap_or(existing.rpm_limit),
            tpm_limit: dto.tpm_limit.unwrap_or(existing.tpm_limit),
            enabled: dto.enabled.unwrap_or(existing.enabled),
            created_at: existing.created_at,
            updated_at: Some(DateTime::now()),
        };

        UserLevelConfig::update_by_map(pool!(), &updated_config, rbs::value! { "id": &dto.id })
            .await?;

        Ok(UserLevelConfigVO {
            id: updated_config
                .id
                .ok_or_else(|| ApplicationError::BusinessError {
                    message: "User level config missing id".to_string(),
                    code: Some("MISSING_ID".to_string()),
                    context: None,
                })?,
            level: updated_config.level,
            level_name: updated_config.level_name,
            description: updated_config.description,
            rpm_limit: updated_config.rpm_limit,
            tpm_limit: updated_config.tpm_limit,
            enabled: updated_config.enabled,
            created_at: updated_config.created_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "User level config missing created_at".to_string(),
                    code: Some("MISSING_CREATED_AT".to_string()),
                    context: None,
                }
            })?,
            updated_at: updated_config.updated_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "User level config missing updated_at".to_string(),
                    code: Some("MISSING_UPDATED_AT".to_string()),
                    context: None,
                }
            })?,
        })
    }

    /// 更新用户等级
    pub async fn update_user_level(
        &self,
        user_id: &str,
        dto: UpdateUserLevelDTO,
    ) -> ApplicationResult<()> {
        let user = SysUser::select_by_map(pool!(), rbs::value! { "id": user_id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("User {} not found", user_id),
                resource: Some("user".to_string()),
                id: Some(user_id.to_string()),
            })?;

        let level_code = format!("L{}", dto.level);
        let level_config =
            UserLevelConfig::select_by_map(pool!(), rbs::value! { "level": &level_code })
                .await?
                .first()
                .cloned()
                .ok_or_else(|| ApplicationError::NotFound {
                    message: format!("User level {} not found", level_code),
                    resource: Some("user_level".to_string()),
                    id: Some(level_code.to_string()),
                })?;

        if level_config.enabled != 1 {
            return Err(ApplicationError::BusinessError {
                message: format!("User level {} is disabled", level_code),
                code: Some("USER_LEVEL_DISABLED".to_string()),
                context: Some("Cannot assign a disabled user level".to_string()),
            });
        }

        SysUser::update_by_map(
            pool!(),
            &SysUser {
                id: user.id,
                account: user.account,
                password: user.password,
                name: user.name,
                email: user.email,
                login_check: user.login_check,
                state: user.state,
                create_date: user.create_date,
                balance: user.balance,
                user_level: Some(level_code),
            },
            rbs::value! { "id": user_id },
        )
        .await?;

        Ok(())
    }

    /// 删除用户等级
    pub async fn delete_level(&self, id: &str) -> ApplicationResult<()> {
        let existing = UserLevelConfig::select_by_map(pool!(), rbs::value! { "id": id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("User level config {} not found", id),
                resource: Some("user_level_config".to_string()),
                id: Some(id.to_string()),
            })?;

        let users_with_level =
            SysUser::select_by_map(pool!(), rbs::value! { "user_level": &existing.level }).await?;

        if !users_with_level.is_empty() {
            return Err(ApplicationError::BusinessError {
                message: format!(
                    "Cannot delete level {}: {} users are assigned to this level",
                    existing.level,
                    users_with_level.len()
                ),
                code: Some("LEVEL_IN_USE".to_string()),
                context: Some(
                    "Cannot delete a user level that is currently assigned to users".to_string(),
                ),
            });
        }

        UserLevelConfig::delete_by_map(pool!(), rbs::value! { "id": id }).await?;

        Ok(())
    }

    /// 批量更新用户等级状态
    pub async fn batch_update_status(
        &self,
        dto: BatchUpdateUserLevelStatusDTO,
    ) -> ApplicationResult<usize> {
        let mut updated_count = 0;

        for id in &dto.ids {
            let existing = UserLevelConfig::select_by_map(pool!(), rbs::value! { "id": id })
                .await?
                .first()
                .cloned()
                .ok_or_else(|| ApplicationError::NotFound {
                    message: format!("User level config {} not found", id),
                    resource: Some("user_level_config".to_string()),
                    id: Some(id.clone()),
                })?;

            let updated_config = UserLevelConfig {
                id: existing.id,
                level: existing.level,
                level_name: existing.level_name,
                description: existing.description,
                rpm_limit: existing.rpm_limit,
                tpm_limit: existing.tpm_limit,
                enabled: dto.enabled,
                created_at: existing.created_at,
                updated_at: Some(DateTime::now()),
            };

            UserLevelConfig::update_by_map(pool!(), &updated_config, rbs::value! { "id": id })
                .await?;

            updated_count += 1;
        }

        Ok(updated_count)
    }

    /// 批量删除用户等级
    pub async fn batch_delete(&self, dto: BatchDeleteUserLevelsDTO) -> ApplicationResult<usize> {
        let mut deleted_count = 0;

        for id in &dto.ids {
            match self.delete_level(id).await {
                Ok(_) => deleted_count += 1,
                Err(e) => {
                    return Err(ApplicationError::BusinessError {
                        message: format!("Failed to delete level {}: {}", id, e),
                        code: Some("BATCH_DELETE_FAILED".to_string()),
                        context: Some("One or more levels could not be deleted".to_string()),
                    });
                }
            }
        }

        Ok(deleted_count)
    }

    /// 初始化默认用户等级配置
    pub async fn init_default_levels(&self) -> ApplicationResult<()> {
        let default_levels = vec![
            ("L1", "Level 1", "基础用户等级", 60, 40000, 1),
            ("L2", "Level 2", "标准用户等级", 120, 90000, 2),
            ("L3", "Level 3", "高级用户等级", 300, 250000, 3),
            ("L4", "Level 4", "专业用户等级", 600, 600000, 4),
            ("L5", "Level 5", "企业用户等级", 1200, 1500000, 5),
        ];

        for (level_code, level_name, description, rpm_limit, tpm_limit, _priority) in default_levels
        {
            let existing =
                UserLevelConfig::select_by_map(pool!(), rbs::value! { "level": level_code })
                    .await?;

            if existing.is_empty() {
                let now = DateTime::now();
                let config = UserLevelConfig {
                    id: Some(ulid::Ulid::new().to_string()),
                    level: level_code.to_string(),
                    level_name: level_name.to_string(),
                    description: Some(description.to_string()),
                    rpm_limit,
                    tpm_limit,
                    enabled: 1,
                    created_at: Some(now.clone()),
                    updated_at: Some(now.clone()),
                };

                UserLevelConfig::insert(pool!(), &config).await?;
            }
        }

        Ok(())
    }
}
