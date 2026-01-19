// 用途：导入应用程序配置结构体
// 说明：用于在服务上下文中存储和访问应用程序配置
use crate::config::application::ApplicationConfig;

// 用途：导入各种服务结构体
// 说明：用于在服务上下文中存储和管理各种业务服务实例
use crate::service::{
    BillingService, // 计费服务
    CacheService,   // 缓存服务
    // AI Hub 服务
    QuotaService,                  // 配额管理服务
    RbacPermissionAuditLogService, // 权限审计日志服务
    RbacPermissionService,         // 权限服务
    RbacRolePermissionService,     // 角色权限关联服务
    RbacRoleService,               // 角色服务
    RbacUserRoleService,           // 用户角色关联服务
    StorageService,                // 存储服务
    SysAuthService,                // 系统认证服务
    SysDictService,                // 系统字典服务
    SysOrganizationService,        // 系统组织服务
    SysTrashService,               // 系统回收站服务
    SysUserService,                // 系统用户服务
    UserLevelService,              // 用户等级服务
                                   // AnalyticsService,                // 统计分析服务 (暂未实现)
};

// 用途：导入AI Hub服务结构体
// 说明：用于模型和供应商管理服务
use crate::service::ai_hub::{
    ApiKeyService,             // API密钥管理服务
    BalanceService,            // 余额管理服务
    EncryptionService,         // 加密服务
    ProviderConfigServiceImpl, // 供应商配置服务实现
    RateLimitService,          // 速率限制服务
    UsageLogService,           // 用量记录服务
};

// 用途：导入RBatis结构体
// 说明：用于数据库操作和连接池管理
use rbatis::RBatis;

// 用途：导入日志拦截器
// 说明：用于配置数据库操作的日志级别
use rbatis::intercept_log::LogInterceptor;

// 用途：导入同步相关的类型
// 说明：用于创建线程安全的全局上下文
use std::sync::{Arc, LazyLock};

// 用途：导入时间相关的类型
// 说明：用于配置数据库连接超时时间
use std::time::Duration;

// 用途：导入缓存相关的类型
// 说明：用于内存缓存API密钥和模型列表
use rbatis::rbdc::DateTime;
use std::collections::HashMap;
use std::sync::RwLock;

/// 用途：全局服务上下文
/// 说明：提供应用程序级别的全局状态管理，包括配置、数据库连接和各种服务实例
pub static CONTEXT: LazyLock<ServiceContext> = LazyLock::new(ServiceContext::default);

/// 用途：数据库连接池宏
/// 说明：提供便捷的方式获取全局数据库连接池
#[macro_export]
macro_rules! pool {
    // 用途：宏定义
    // 说明：当调用pool!()宏时，返回全局上下文的数据库连接池引用
    () => {
        &*$crate::context::CONTEXT.rb
    };
}

/// 用途：服务上下文结构体
/// 说明：集中管理应用程序的所有核心组件，包括配置、数据库和各种服务
pub struct ServiceContext {
    pub config: ApplicationConfig, // 用途：应用程序配置
    // 说明：存储应用程序的所有配置信息
    pub rb: Arc<RBatis>, // 用途：数据库连接池
    // 说明：提供数据库操作的连接管理
    pub cache_service: CacheService, // 用途：缓存服务
    // 说明：提供缓存操作功能
    pub storage_service: StorageService, // 用途：存储服务
    // 说明：提供文件存储功能
    pub sys_user_service: SysUserService, // 用途：系统用户服务
    // 说明：处理用户相关的业务逻辑
    pub rbac_permission_service: RbacPermissionService, // 用途：权限服务
    // 说明：处理权限相关的业务逻辑
    pub rbac_permission_audit_log_service: RbacPermissionAuditLogService, // 用途：权限审计日志服务
    // 说明：处理权限审计日志相关业务逻辑
    pub rbac_role_service: RbacRoleService, // 用途：角色服务
    // 说明：处理角色相关的业务逻辑
    pub rbac_role_permission_service: RbacRolePermissionService, // 用途：角色权限关联服务
    // 说明：处理角色和权限的关联关系
    pub rbac_user_role_service: RbacUserRoleService, // 用途：用户角色关联服务
    // 说明：处理用户和角色的关联关系
    pub sys_dict_service: SysDictService, // 用途：系统字典服务
    // 说明：处理字典数据的业务逻辑
    pub sys_organization_service: SysOrganizationService, // 用途：系统组织服务
    // 说明：处理组织数据的业务逻辑
    pub sys_auth_service: SysAuthService, // 用途：系统认证服务
    // 说明：处理认证相关的业务逻辑
    pub sys_trash_service: SysTrashService, // 用途：系统回收站服务
    // 说明：处理回收站相关的业务逻辑

    // AI Hub 服务
    pub quota_service: QuotaService, // 用途：配额管理服务
    // 说明：处理用户配额相关业务逻辑
    pub billing_service: BillingService, // 用途：计费服务
    // 说明：处理费用计算和配额检查
    pub user_level_service: UserLevelService, // 用途：用户等级服务
    // 说明：处理用户等级管理相关业务逻辑
    pub api_key_service: ApiKeyService, // 用途：API密钥管理服务
    // 说明：处理API密钥创建、验证和管理相关业务逻辑
    pub balance_service: BalanceService, // 用途：余额管理服务
    // 说明：处理用户余额查询和管理相关业务逻辑
    pub rate_limit_service: RateLimitService, // 用途：速率限制服务
    // 说明：处理RPM/TPM速率限制相关业务逻辑
    pub usage_log_service: UsageLogService, // 用途：用量记录服务
    // 说明：处理用量记录查询、统计和详情查看
    // pub analytics_service: AnalyticsService,         // 用途：统计分析服务 (暂未实现)
    // 说明：处理用量统计和分析

    // 新增的AI Hub管理服务 - 使用Arc包装以便共享
    pub provider_config_service: Arc<tokio::sync::RwLock<Option<ProviderConfigServiceImpl>>>, // 用途：供应商配置服务
    // 说明：管理AI服务供应商的配置
    pub provider_registry: Arc<tokio::sync::RwLock<crate::providers::registry::ProviderRegistry>>, // 用途：供应商注册表
    // 说明：管理和注册所有AI服务供应商
    pub model_router: Arc<crate::routers::model_router::ModelRouter>, // 用途：模型路由器
    // 说明：解析模型标识符并路由到正确的供应商和模型

    // 内存缓存字段
    pub api_key_cache: Arc<RwLock<HashMap<String, String>>>, // 用途：API密钥缓存
    // 说明：缓存已解密的API密钥，避免重复解密
    pub model_list_cache: Arc<RwLock<Option<serde_json::Value>>>, // 用途：模型列表缓存
    // 说明：缓存OpenAI格式的模型列表，减少数据库查询
    pub model_cache_ttl: Duration, // 用途：模型列表缓存TTL
    // 说明：模型列表缓存的有效期，默认5分钟
    pub model_cache_last_update: Arc<RwLock<DateTime>>, // 用途：模型列表缓存最后更新时间
                                                        // 说明：记录模型列表缓存的上次更新时间
}

/// 用途：ServiceContext的Clone实现
/// 说明：手动实现Clone，只克隆必要的部分
impl Clone for ServiceContext {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            rb: self.rb.clone(),
            cache_service: self.cache_service.clone(),
            storage_service: self.storage_service.clone(),
            sys_user_service: self.sys_user_service.clone(),
            rbac_permission_service: self.rbac_permission_service.clone(),
            rbac_permission_audit_log_service: self.rbac_permission_audit_log_service.clone(),
            rbac_role_service: self.rbac_role_service.clone(),
            rbac_role_permission_service: self.rbac_role_permission_service.clone(),
            rbac_user_role_service: self.rbac_user_role_service.clone(),
            sys_dict_service: self.sys_dict_service.clone(),
            sys_organization_service: self.sys_organization_service.clone(),
            sys_auth_service: self.sys_auth_service.clone(),
            sys_trash_service: self.sys_trash_service.clone(),
            quota_service: self.quota_service.clone(),
            billing_service: self.billing_service.clone(),
            user_level_service: self.user_level_service.clone(),
            api_key_service: self.api_key_service.clone(),
            balance_service: self.balance_service.clone(),
            rate_limit_service: self.rate_limit_service.clone(),
            usage_log_service: self.usage_log_service.clone(),
            provider_config_service: self.provider_config_service.clone(),
            provider_registry: self.provider_registry.clone(),
            model_router: self.model_router.clone(),
            api_key_cache: self.api_key_cache.clone(),
            model_list_cache: self.model_list_cache.clone(),
            model_cache_ttl: self.model_cache_ttl,
            model_cache_last_update: self.model_cache_last_update.clone(),
        }
    }
}

/// 用途：ServiceContext的方法实现
/// 说明：提供服务上下文的各种操作方法
impl ServiceContext {
    /// 用途：初始化数据库连接池
    /// 说明：创建和配置数据库连接池，为应用程序提供数据库访问能力
    pub async fn init_database(&self) {
        // 用途：输出数据库初始化日志
        // 说明：告知用户正在初始化数据库连接池
        log::info!("[rsllm] rbatis pool init ({})...", self.config.db_url);

        // 用途：连接数据库
        // 说明：根据配置的数据库URL建立连接
        // 包含：自动根据数据库URL选择驱动
        self.rb
            .link(include!("../target/driver.rs"), &self.config.db_url)
            .await
            .expect("[rsllm] rbatis pool init fail!");

        // 用途：添加回收站服务拦截器
        // 说明：用于拦截数据库操作，实现回收站功能
        self.rb.intercepts.push(Arc::new(SysTrashService::new()));

        // 用途：获取数据库连接池
        // 说明：用于后续配置连接池参数
        let pool = self.rb.get_pool().expect("[rsllm] rbatis pool init fail!");

        // 用途：配置数据库操作日志级别
        // 说明：根据应用程序日志级别设置数据库操作的日志输出
        self.rb
            .get_intercept::<LogInterceptor>()
            .expect("rbatis LogInterceptor init fail!")
            .set_level_filter(log::max_level());

        // 用途：设置最大连接数
        // 说明：控制数据库连接池的最大连接数量，避免连接过多导致数据库压力过大
        pool.set_max_open_conns(self.config.db_pool_len as u64)
            .await;

        // 用途：设置连接超时时间
        // 说明：控制获取数据库连接的最大等待时间，避免长时间阻塞
        pool.set_timeout(Some(Duration::from_secs(
            self.config.db_pool_timeout as u64,
        )))
        .await;

        // 用途：输出数据库初始化成功日志
        // 说明：告知用户数据库连接池初始化成功，并显示连接池状态
        log::info!(
            "[rsllm] rbatis pool init success! pool state = {}",
            self.rb.get_pool().expect("pool not init!").state().await
        );
    }

    /// 用途：初始化供应商注册表
    /// 说明：从数据库加载供应商配置并初始化供应商注册表
    pub async fn init_providers(&self) {
        // 用途：输出供应商初始化日志
        // 说明：告知用户正在初始化供应商注册表
        log::info!("[rsllm] init providers...");

        // 用途：从数据库加载所有启用的供应商
        // 说明：只加载启用状态的供应商配置
        let rb = self.rb.clone();
        let providers = match crate::domain::table::ai_hub::provider_config::ProviderConfig::select_enabled_page(&rb, 1, 1000).await {
            Ok(providers) => {
                log::info!("[rsllm] loaded {} providers from database", providers.len());
                providers
            },
            Err(e) => {
                log::error!("[rsllm] load providers from db fail: {}", e);
                return;
            }
        };

        // 用途：创建加密服务用于解密API密钥
        // 说明：从环境变量获取加密密钥，如果未设置则使用默认密钥
        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .unwrap_or_else(|_| "rsllm_encryption_key_default_key".to_string());
        let encryption_service = EncryptionService::new(encryption_key.as_bytes())
            .expect("Failed to create encryption service");

        // 用途：初始化供应商注册表
        // 说明：根据加载的供应商配置创建供应商实例并注册到注册表中
        // 使用RwLock来修改provider_registry，并传递加密服务以解密API密钥
        let new_registry = crate::providers::registry::ProviderRegistry::from_db_with_encryption(
            &providers,
            Some(&encryption_service),
        );
        *self.provider_registry.write().await = new_registry;

        // 用途：输出供应商初始化成功日志
        // 说明：告知用户供应商初始化成功，并显示注册的供应商数量
        log::info!(
            "[rsllm] init providers success! count = {}",
            providers.len()
        );
    }

    /// 用途：初始化AI Hub管理服务
    /// 说明：创建模型和供应商管理服务实例
    pub async fn init_ai_hub_services(&self) {
        log::info!("[rsllm] init AI Hub management services...");

        // 创建加密服务 - 从环境变量获取加密密钥，如果未设置则使用默认密钥
        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .unwrap_or_else(|_| "rsllm_encryption_key_default_key".to_string());
        let encryption_service = EncryptionService::new(encryption_key.as_bytes())
            .expect("Failed to create encryption service");

        // 获取当前ServiceContext的Arc引用
        let ctx_arc = Arc::new(self.clone());

        // 初始化供应商配置服务（传递ServiceContext）
        let provider_config_service = ProviderConfigServiceImpl::new(encryption_service, ctx_arc);

        // 更新ServiceContext中的服务实例
        *self.provider_config_service.write().await = Some(provider_config_service);

        log::info!("[rsllm] AI Hub management services initialized successfully");
    }

    /// 用途：获取解密的API密钥（带缓存）
    /// 说明：从缓存中获取已解密的API密钥，如果缓存不存在则解密并缓存
    pub fn get_decrypted_api_key(
        &self,
        provider_id: &str,
        _encrypted_key: &str,
    ) -> Result<String, String> {
        // 尝试从缓存中获取
        {
            let cache = self.api_key_cache.read().unwrap();
            if let Some(key) = cache.get(provider_id) {
                return Ok(key.clone());
            }
        }

        // 缓存不存在，需要解密
        // 注意：这里需要从provider_config_service获取加密服务来解密
        // 由于ServiceContext没有直接访问加密服务，我们需要通过其他方式
        // 暂时返回错误，需要在provider_config_service中实现缓存逻辑
        Err("Encryption service not accessible from context".to_string())
    }

    /// 用途：更新API密钥缓存
    /// 说明：将解密的API密钥添加到缓存中
    pub fn update_api_key_cache(&self, provider_id: String, decrypted_key: String) {
        let mut cache = self.api_key_cache.write().unwrap();
        cache.insert(provider_id, decrypted_key);
    }

    /// 用途：清除API密钥缓存
    /// 说明：清除指定供应商的API密钥缓存
    pub fn clear_api_key_cache(&self, provider_id: &str) {
        let mut cache = self.api_key_cache.write().unwrap();
        cache.remove(provider_id);
    }

    /// 用途：获取模型列表缓存
    /// 说明：从缓存中获取OpenAI格式的模型列表
    pub fn get_model_list_cache(&self) -> Option<serde_json::Value> {
        let cache = self.model_list_cache.read().unwrap();
        cache.clone()
    }

    /// 用途：更新模型列表缓存
    /// 说明：将OpenAI格式的模型列表添加到缓存中，并更新最后更新时间
    pub fn update_model_list_cache(&self, model_list: serde_json::Value) {
        let mut cache = self.model_list_cache.write().unwrap();
        *cache = Some(model_list);

        let mut last_update = self.model_cache_last_update.write().unwrap();
        *last_update = DateTime::now();
    }

    /// 用途：清除模型列表缓存
    /// 说明：清除模型列表缓存
    pub fn clear_model_list_cache(&self) {
        let mut cache = self.model_list_cache.write().unwrap();
        *cache = None;
    }

    /// 用途：检查模型列表缓存是否过期
    /// 说明：根据TTL检查缓存是否需要更新
    pub fn is_model_list_cache_expired(&self) -> bool {
        let last_update = self.model_cache_last_update.read().unwrap();
        let now = DateTime::now();
        let elapsed = now.unix_timestamp() - last_update.unix_timestamp();

        elapsed > self.model_cache_ttl.as_secs() as i64
    }
}

/// 用途：ServiceContext的默认实现
/// 说明：提供服务上下文的默认初始化方式
impl Default for ServiceContext {
    fn default() -> Self {
        // 用途：加载应用程序配置
        // 说明：从配置文件中读取应用程序配置
        let config = ApplicationConfig::default();

        // 用途：创建共享的供应商注册表实例
        // 说明：provider_registry和model_router必须使用同一个实例
        let provider_registry = Arc::new(tokio::sync::RwLock::new(
            crate::providers::registry::ProviderRegistry::new(),
        ));
        let model_router = Arc::new(crate::routers::model_router::ModelRouter::new(
            provider_registry.clone(),
        ));

        // 用途：创建服务上下文实例
        // 说明：初始化所有服务组件
        ServiceContext {
            // 用途：初始化RBatis实例
            // 说明：创建数据库连接池实例
            rb: Arc::new(RBatis::new()),

            // 用途：初始化缓存服务
            // 说明：创建缓存服务实例，用于缓存数据
            cache_service: CacheService::new(&config).expect("[rsllm] cache service init fail!"),

            // 用途：初始化存储服务
            // 说明：创建存储服务实例，用于文件存储
            storage_service: StorageService::new(&config.storage)
                .expect("Failed to create storage service"),

            // 用途：初始化系统用户服务
            // 说明：创建用户服务实例，用于处理用户相关业务
            sys_user_service: SysUserService {},

            // 用途：初始化角色服务
            // 说明：创建角色服务实例，用于处理角色相关业务
            rbac_role_service: RbacRoleService {},

            // 用途：初始化权限服务
            // 说明：创建权限服务实例，用于处理权限相关业务
            rbac_permission_service: RbacPermissionService {},

            // 用途：初始化权限审计日志服务
            // 说明：创建权限审计日志服务实例，用于处理权限审计日志相关业务
            rbac_permission_audit_log_service: RbacPermissionAuditLogService {},

            // 用途：初始化角色权限关联服务
            // 说明：创建角色权限关联服务实例，用于处理角色和权限的关联关系
            rbac_role_permission_service: RbacRolePermissionService {},

            // 用途：初始化用户角色关联服务
            // 说明：创建用户角色关联服务实例，用于处理用户和角色的关联关系
            rbac_user_role_service: RbacUserRoleService {},

            // 用途：初始化系统字典服务
            // 说明：创建字典服务实例，用于处理字典数据业务
            sys_dict_service: SysDictService {},

            // 用途：初始化系统组织服务
            // 说明：创建组织服务实例，用于处理组织数据业务
            sys_organization_service: SysOrganizationService {},

            // 用途：初始化系统认证服务
            // 说明：创建认证服务实例，用于处理认证相关业务
            sys_auth_service: SysAuthService {},

            // 用途：初始化系统回收站服务
            // 说明：创建回收站服务实例，用于处理回收站相关业务
            sys_trash_service: SysTrashService::new(),

            // 用途：初始化配额管理服务
            // 说明：创建配额管理服务实例，用于处理用户配额相关业务
            quota_service: QuotaService {},

            // 用途：初始化计费服务
            // 说明：创建计费服务实例，用于处理费用计算和配额检查
            billing_service: BillingService {
                quota_service: QuotaService {},
            },

            // 用途：初始化用户等级服务
            // 说明：创建用户等级服务实例，用于处理用户等级管理相关业务
            user_level_service: UserLevelService {},

            // 用途：初始化API密钥管理服务
            // 说明：创建API密钥管理服务实例，用于处理API密钥创建、验证和管理相关业务
            api_key_service: ApiKeyService {},

            // 用途：初始化余额管理服务
            // 说明：创建余额管理服务实例，用于处理用户余额查询和管理相关业务
            balance_service: BalanceService::new(),

            // 用途：初始化速率限制服务
            // 说明：创建速率限制服务实例，用于处理RPM/TPM速率限制相关业务
            rate_limit_service: RateLimitService::new(),

            // 用途：初始化用量记录服务
            // 说明：创建用量记录服务实例，用于处理用量记录查询、统计和详情查看
            usage_log_service: UsageLogService {},

            // 用途：初始化统计分析服务
            // 说明：创建统计分析服务实例，用于处理用量统计和分析
            // analytics_service: AnalyticsService {}, // 暂未实现

            // 用途：初始化供应商注册表
            // 说明：创建供应商注册表实例，用于管理和注册所有AI服务供应商
            // 注意：使用上面创建的共享实例
            provider_registry,

            // 用途：初始化模型路由器
            // 说明：创建模型路由器实例，用于解析模型标识符并路由到正确的供应商和模型
            // 注意：使用上面创建的共享实例，与provider_registry使用同一个实例
            model_router,

            // 用途：初始化AI Hub管理服务
            // 说明：创建模型和供应商管理服务实例
            provider_config_service: Arc::new(tokio::sync::RwLock::new(None)), // 将在init_providers中初始化

            // 用途：初始化内存缓存
            // 说明：创建API密钥和模型列表的内存缓存
            api_key_cache: Arc::new(RwLock::new(HashMap::new())),
            model_list_cache: Arc::new(RwLock::new(None)),
            model_cache_ttl: Duration::from_secs(300), // 默认5分钟
            model_cache_last_update: Arc::new(RwLock::new(DateTime::now())),

            // 用途：设置应用程序配置
            // 说明：将加载的配置赋值给服务上下文
            config,
        }
    }
}
