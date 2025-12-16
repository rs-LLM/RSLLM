// 用途：导入应用程序配置结构体
// 说明：用于在服务上下文中存储和访问应用程序配置
use crate::config::config::ApplicationConfig;

// 用途：导入各种服务结构体
// 说明：用于在服务上下文中存储和管理各种业务服务实例
use crate::service::{
    CacheService,                    // 缓存服务
    RbacPermissionService,           // 权限服务
    RbacRolePermissionService,       // 角色权限关联服务
    RbacRoleService,                 // 角色服务
    RbacUserRoleService,             // 用户角色关联服务
    StorageService,                  // 存储服务
    SysAuthService,                  // 系统认证服务
    SysDictService,                  // 系统字典服务
    SysTrashService,                 // 系统回收站服务
    SysUserService,                  // 系统用户服务
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

/// 用途：全局服务上下文
/// 说明：提供应用程序级别的全局状态管理，包括配置、数据库连接和各种服务实例
pub static CONTEXT: LazyLock<ServiceContext> = LazyLock::new(|| ServiceContext::default());

/// 用途：数据库连接池宏
/// 说明：提供便捷的方式获取全局数据库连接池
#[macro_export]
macro_rules! pool {
    // 用途：宏定义
    // 说明：当调用pool!()宏时，返回全局上下文的数据库连接池引用
    () => {
        &$crate::context::CONTEXT.rb
    };
}

/// 用途：服务上下文结构体
/// 说明：集中管理应用程序的所有核心组件，包括配置、数据库和各种服务
pub struct ServiceContext {
    pub config: ApplicationConfig,                    // 用途：应用程序配置
                                                    // 说明：存储应用程序的所有配置信息
    pub rb: RBatis,                                  // 用途：数据库连接池
                                                    // 说明：提供数据库操作的连接管理
    pub cache_service: CacheService,                 // 用途：缓存服务
                                                    // 说明：提供缓存操作功能
    pub storage_service: StorageService,             // 用途：存储服务
                                                    // 说明：提供文件存储功能
    pub sys_user_service: SysUserService,            // 用途：系统用户服务
                                                    // 说明：处理用户相关的业务逻辑
    pub rbac_permission_service: RbacPermissionService, // 用途：权限服务
                                                      // 说明：处理权限相关的业务逻辑
    pub rbac_role_service: RbacRoleService,          // 用途：角色服务
                                                    // 说明：处理角色相关的业务逻辑
    pub rbac_role_permission_service: RbacRolePermissionService, // 用途：角色权限关联服务
                                                                 // 说明：处理角色和权限的关联关系
    pub rbac_user_role_service: RbacUserRoleService, // 用途：用户角色关联服务
                                                    // 说明：处理用户和角色的关联关系
    pub sys_dict_service: SysDictService,            // 用途：系统字典服务
                                                    // 说明：处理字典数据的业务逻辑
    pub sys_auth_service: SysAuthService,            // 用途：系统认证服务
                                                    // 说明：处理认证相关的业务逻辑
    pub sys_trash_service: SysTrashService,          // 用途：系统回收站服务
                                                    // 说明：处理回收站相关的业务逻辑
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
}

/// 用途：ServiceContext的默认实现
/// 说明：提供服务上下文的默认初始化方式
impl Default for ServiceContext {
    fn default() -> Self {
        // 用途：加载应用程序配置
        // 说明：从配置文件中读取应用程序配置
        let config = ApplicationConfig::default();
        
        // 用途：创建服务上下文实例
        // 说明：初始化所有服务组件
        ServiceContext {
            // 用途：初始化RBatis实例
            // 说明：创建数据库连接池实例
            rb: {
                let rb = RBatis::new();
                rb
            },
            
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
            
            // 用途：初始化角色权限关联服务
            // 说明：创建角色权限关联服务实例，用于处理角色和权限的关联关系
            rbac_role_permission_service: RbacRolePermissionService {},
            
            // 用途：初始化用户角色关联服务
            // 说明：创建用户角色关联服务实例，用于处理用户和角色的关联关系
            rbac_user_role_service: RbacUserRoleService {},
            
            // 用途：初始化系统字典服务
            // 说明：创建字典服务实例，用于处理字典数据业务
            sys_dict_service: SysDictService {},
            
            // 用途：初始化系统认证服务
            // 说明：创建认证服务实例，用于处理认证相关业务
            sys_auth_service: SysAuthService {},
            
            // 用途：初始化系统回收站服务
            // 说明：创建回收站服务实例，用于处理回收站相关业务
            sys_trash_service: SysTrashService::new(),
            
            // 用途：设置应用程序配置
            // 说明：将加载的配置赋值给服务上下文
            config,
        }
    }
}
