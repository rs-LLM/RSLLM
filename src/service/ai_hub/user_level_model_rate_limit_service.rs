//! 等级与模型速率限制服务模块
//! 提供等级与模型速率限制的查询、创建、更新和删除功能
use crate::domain::dto::ai_hub::user_level_model_limit::{
    CreateUserLevelModelLimitDTO, ListUserLevelModelLimitsResponse, UpdateUserLevelModelLimitDTO,
    UserLevelModelLimitQueryDTO, UserLevelModelLimitVO,
};
use crate::domain::table::ai_hub::user_level_model_rate_limit::UserLevelModelRateLimit;
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use rbatis::rbdc::DateTime;

/// 等级与模型速率限制服务
///
/// 负责等级与模型速率限制的查询、创建、更新和删除
#[derive(Clone)]
pub struct UserLevelModelRateLimitService {}

impl Default for UserLevelModelRateLimitService {
    fn default() -> Self {
        Self::new()
    }
}

impl UserLevelModelRateLimitService {
    /// 创建服务实例
    pub fn new() -> Self {
        Self {}
    }

    /// 将 DateTime 转换为字符串
    fn datetime_to_string(dt: DateTime) -> String {
        dt.to_string()
    }

    /// 创建等级模型限制
    pub async fn create_limit(
        &self,
        dto: CreateUserLevelModelLimitDTO,
    ) -> ApplicationResult<UserLevelModelLimitVO> {
        let existing = UserLevelModelRateLimit::select_by_map(
            pool!(),
            rbs::value! {
                "user_level": &dto.user_level,
                "model_key": &dto.model_key
            },
        )
        .await?;

        if !existing.is_empty() {
            return Err(ApplicationError::BusinessError {
                message: format!(
                    "Limit for level {} and model {} already exists",
                    dto.user_level, dto.model_key
                ),
                code: Some("LIMIT_ALREADY_EXISTS".to_string()),
                context: Some(
                    "A rate limit configuration for this user level and model already exists"
                        .to_string(),
                ),
            });
        }

        let now = DateTime::now();
        let limit = UserLevelModelRateLimit {
            id: Some(ulid::Ulid::new().to_string()),
            user_level: dto.user_level.clone(),
            model_key: dto.model_key.clone(),
            rpm_limit: dto.rpm_limit,
            tpm_limit: dto.tpm_limit,
            enabled: dto.enabled,
            created_at: Some(now.clone()),
            updated_at: Some(now.clone()),
        };

        let id = limit
            .id
            .clone()
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "Failed to generate limit ID".to_string(),
                code: Some("LIMIT_ID_GENERATION_FAILED".to_string()),
                context: Some("Failed to generate limit ID after successful creation".to_string()),
            })?;

        UserLevelModelRateLimit::insert(pool!(), &limit).await?;

        Ok(UserLevelModelLimitVO {
            id,
            user_level: limit.user_level,
            model_key: limit.model_key,
            rpm_limit: limit.rpm_limit,
            tpm_limit: limit.tpm_limit,
            enabled: limit.enabled,
            created_at: Self::datetime_to_string(limit.created_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "Limit missing created_at".to_string(),
                    code: Some("MISSING_CREATED_AT".to_string()),
                    context: None,
                }
            })?),
            updated_at: Self::datetime_to_string(limit.updated_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "Limit missing updated_at".to_string(),
                    code: Some("MISSING_UPDATED_AT".to_string()),
                    context: None,
                }
            })?),
        })
    }

    /// 更新等级模型限制
    pub async fn update_limit(
        &self,
        id: &str,
        dto: UpdateUserLevelModelLimitDTO,
    ) -> ApplicationResult<UserLevelModelLimitVO> {
        let existing = UserLevelModelRateLimit::select_by_map(pool!(), rbs::value! { "id": id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("Limit {} not found", id),
                resource: Some("user_level_model_limit".to_string()),
                id: Some(id.to_string()),
            })?;

        let updated_limit = UserLevelModelRateLimit {
            id: existing.id,
            user_level: existing.user_level,
            model_key: existing.model_key,
            rpm_limit: dto.rpm_limit.unwrap_or(existing.rpm_limit),
            tpm_limit: dto.tpm_limit.unwrap_or(existing.tpm_limit),
            enabled: dto.enabled.unwrap_or(existing.enabled),
            created_at: existing.created_at,
            updated_at: Some(DateTime::now()),
        };

        UserLevelModelRateLimit::update_by_map(pool!(), &updated_limit, rbs::value! { "id": id })
            .await?;

        Ok(UserLevelModelLimitVO {
            id: updated_limit
                .id
                .ok_or_else(|| ApplicationError::BusinessError {
                    message: "Limit missing id".to_string(),
                    code: Some("MISSING_ID".to_string()),
                    context: None,
                })?,
            user_level: updated_limit.user_level,
            model_key: updated_limit.model_key,
            rpm_limit: updated_limit.rpm_limit,
            tpm_limit: updated_limit.tpm_limit,
            enabled: updated_limit.enabled,
            created_at: Self::datetime_to_string(updated_limit.created_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "Limit missing created_at".to_string(),
                    code: Some("MISSING_CREATED_AT".to_string()),
                    context: None,
                }
            })?),
            updated_at: Self::datetime_to_string(updated_limit.updated_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "Limit missing updated_at".to_string(),
                    code: Some("MISSING_UPDATED_AT".to_string()),
                    context: None,
                }
            })?),
        })
    }

    /// 删除等级模型限制
    pub async fn delete_limit(&self, id: &str) -> ApplicationResult<()> {
        UserLevelModelRateLimit::delete_by_map(pool!(), rbs::value! { "id": id }).await?;

        Ok(())
    }

    /// 获取等级模型限制
    pub async fn get_limit(&self, id: &str) -> ApplicationResult<UserLevelModelLimitVO> {
        let limit = UserLevelModelRateLimit::select_by_map(pool!(), rbs::value! { "id": id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("Limit {} not found", id),
                resource: Some("user_level_model_limit".to_string()),
                id: Some(id.to_string()),
            })?;

        Ok(UserLevelModelLimitVO {
            id: limit.id.ok_or_else(|| ApplicationError::BusinessError {
                message: "Limit missing id".to_string(),
                code: Some("MISSING_ID".to_string()),
                context: None,
            })?,
            user_level: limit.user_level,
            model_key: limit.model_key,
            rpm_limit: limit.rpm_limit,
            tpm_limit: limit.tpm_limit,
            enabled: limit.enabled,
            created_at: Self::datetime_to_string(limit.created_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "Limit missing created_at".to_string(),
                    code: Some("MISSING_CREATED_AT".to_string()),
                    context: None,
                }
            })?),
            updated_at: Self::datetime_to_string(limit.updated_at.ok_or_else(|| {
                ApplicationError::BusinessError {
                    message: "Limit missing updated_at".to_string(),
                    code: Some("MISSING_UPDATED_AT".to_string()),
                    context: None,
                }
            })?),
        })
    }

    /// 根据用户等级和模型键获取限制
    pub async fn get_limit_by_level_and_model(
        &self,
        user_level: &str,
        model_key: &str,
    ) -> ApplicationResult<Option<UserLevelModelLimitVO>> {
        let limits = UserLevelModelRateLimit::select_by_map(
            pool!(),
            rbs::value! {
                "user_level": user_level,
                "model_key": model_key
            },
        )
        .await?;

        if let Some(limit) = limits.first() {
            let limit = limit.clone();
            Ok(Some(UserLevelModelLimitVO {
                id: limit.id.ok_or_else(|| ApplicationError::BusinessError {
                    message: "Limit missing id".to_string(),
                    code: Some("MISSING_ID".to_string()),
                    context: None,
                })?,
                user_level: limit.user_level,
                model_key: limit.model_key,
                rpm_limit: limit.rpm_limit,
                tpm_limit: limit.tpm_limit,
                enabled: limit.enabled,
                created_at: Self::datetime_to_string(limit.created_at.ok_or_else(|| {
                    ApplicationError::BusinessError {
                        message: "Limit missing created_at".to_string(),
                        code: Some("MISSING_CREATED_AT".to_string()),
                        context: None,
                    }
                })?),
                updated_at: Self::datetime_to_string(limit.updated_at.ok_or_else(|| {
                    ApplicationError::BusinessError {
                        message: "Limit missing updated_at".to_string(),
                        code: Some("MISSING_UPDATED_AT".to_string()),
                        context: None,
                    }
                })?),
            }))
        } else {
            Ok(None)
        }
    }

    /// 获取用户等级的所有模型限制
    pub async fn get_limits_by_level(
        &self,
        user_level: &str,
    ) -> ApplicationResult<Vec<UserLevelModelLimitVO>> {
        let limits = UserLevelModelRateLimit::select_by_map(
            pool!(),
            rbs::value! { "user_level": user_level },
        )
        .await?;

        let vos: Vec<UserLevelModelLimitVO> = limits
            .into_iter()
            .filter_map(|limit| {
                Some(UserLevelModelLimitVO {
                    id: limit.id?,
                    user_level: limit.user_level,
                    model_key: limit.model_key,
                    rpm_limit: limit.rpm_limit,
                    tpm_limit: limit.tpm_limit,
                    enabled: limit.enabled,
                    created_at: Self::datetime_to_string(limit.created_at?),
                    updated_at: Self::datetime_to_string(limit.updated_at?),
                })
            })
            .collect();

        Ok(vos)
    }

    /// 查询等级模型限制列表
    pub async fn list_limits(
        &self,
        dto: UserLevelModelLimitQueryDTO,
    ) -> ApplicationResult<ListUserLevelModelLimitsResponse> {
        let page = dto.page.unwrap_or(1) as i64;
        let page_size = dto.page_size.unwrap_or(10) as i64;
        let offset = ((page - 1) * page_size) as usize;

        let mut conditions = rbs::value! {};

        if let Some(user_level) = &dto.user_level {
            conditions["user_level"] = rbs::value!(user_level);
        }

        if let Some(model_key) = &dto.model_key {
            conditions["model_key"] = rbs::value!(model_key);
        }

        if let Some(enabled) = dto.enabled {
            conditions["enabled"] = rbs::value!(if enabled { 1 } else { 0 });
        }

        let limits = if conditions.is_empty() {
            UserLevelModelRateLimit::select_all(pool!()).await?
        } else {
            UserLevelModelRateLimit::select_by_map(pool!(), conditions).await?
        };

        let total = limits.len() as i64;

        let paginated_limits: Vec<UserLevelModelRateLimit> = limits
            .into_iter()
            .skip(offset)
            .take(page_size as usize)
            .collect();

        let items: Vec<UserLevelModelLimitVO> = paginated_limits
            .into_iter()
            .filter_map(|limit| {
                Some(UserLevelModelLimitVO {
                    id: limit.id?,
                    user_level: limit.user_level,
                    model_key: limit.model_key,
                    rpm_limit: limit.rpm_limit,
                    tpm_limit: limit.tpm_limit,
                    enabled: limit.enabled,
                    created_at: Self::datetime_to_string(limit.created_at?),
                    updated_at: Self::datetime_to_string(limit.updated_at?),
                })
            })
            .collect();

        Ok(ListUserLevelModelLimitsResponse { items, total })
    }

    /// 批量创建等级模型限制
    pub async fn batch_create_limits(
        &self,
        user_level: &str,
        model_limits: Vec<(String, i32, i32)>,
    ) -> ApplicationResult<Vec<UserLevelModelLimitVO>> {
        let mut results = Vec::new();

        for (model_key, rpm_limit, tpm_limit) in model_limits {
            let existing = UserLevelModelRateLimit::select_by_map(
                pool!(),
                rbs::value! {
                    "user_level": user_level,
                    "model_key": &model_key
                },
            )
            .await?;

            if existing.is_empty() {
                let now = DateTime::now();
                let limit = UserLevelModelRateLimit {
                    id: Some(ulid::Ulid::new().to_string()),
                    user_level: user_level.to_string(),
                    model_key: model_key.clone(),
                    rpm_limit,
                    tpm_limit,
                    enabled: true,
                    created_at: Some(now.clone()),
                    updated_at: Some(now.clone()),
                };

                let id = limit
                    .id
                    .clone()
                    .ok_or_else(|| ApplicationError::BusinessError {
                        message: "Failed to generate limit ID".to_string(),
                        code: Some("LIMIT_ID_GENERATION_FAILED".to_string()),
                        context: Some(
                            "Failed to generate limit ID after successful creation".to_string(),
                        ),
                    })?;

                UserLevelModelRateLimit::insert(pool!(), &limit).await?;

                let created_at =
                    limit
                        .created_at
                        .ok_or_else(|| ApplicationError::BusinessError {
                            message: "Limit missing created_at".to_string(),
                            code: Some("MISSING_CREATED_AT".to_string()),
                            context: None,
                        })?;
                let updated_at =
                    limit
                        .updated_at
                        .ok_or_else(|| ApplicationError::BusinessError {
                            message: "Limit missing updated_at".to_string(),
                            code: Some("MISSING_UPDATED_AT".to_string()),
                            context: None,
                        })?;

                results.push(UserLevelModelLimitVO {
                    id,
                    user_level: limit.user_level,
                    model_key: limit.model_key,
                    rpm_limit: limit.rpm_limit,
                    tpm_limit: limit.tpm_limit,
                    enabled: limit.enabled,
                    created_at: Self::datetime_to_string(created_at),
                    updated_at: Self::datetime_to_string(updated_at),
                });
            }
        }

        Ok(results)
    }
}
