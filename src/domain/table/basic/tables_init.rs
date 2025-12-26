// 用途：导入RBAC模块
// 说明：用于调用RBAC相关的表同步功能
use crate::domain::table::rbac;
// 用途：导入RBAC相关表结构
// 说明：用于初始化系统默认的角色、权限和用户角色关联数据
use crate::domain::table::rbac::{RbacPermission, RbacRole, RbacRolePermission};
// 用途：导入系统字典表结构
// 说明：用于同步系统字典表结构
use crate::domain::table::sys_dict::SysDict;
// 用途：导入系统回收站表结构
// 说明：用于同步系统回收站表结构
use crate::domain::table::sys_trash::SysTrash;
// 用途：导入系统用户表结构
// 说明：用于同步系统用户表结构和初始化管理员用户
use crate::domain::table::sys_user::SysUser;
// 用途：导入键值配置表结构
// 说明：用于同步键值配置表结构和初始化配置
use crate::domain::table::key_value_config::KeyValueConfig;
// 用途：导入日志级别枚举
// 说明：用于控制日志输出级别
use log::LevelFilter;
// 用途：导入RBatis结构体
// 说明：用于数据库操作和表结构同步
use rbatis::RBatis;
// 用途：导入defer宏
// 说明：用于延迟执行代码，确保日志级别恢复
use rbatis::dark_std::defer;
// 用途：导入日志拦截器
// 说明：用于临时关闭日志输出，避免表同步过程中产生大量日志
use rbatis::intercept_log::LogInterceptor;
// 用途：导入日期时间类型
// 说明：用于记录创建时间
use rbatis::rbdc::DateTime;
// 用途：导入表同步相关的映射器
// 说明：用于根据数据库类型选择合适的表映射器
use rbatis::table_sync::{
    ColumnMapper, MssqlTableMapper, MysqlTableMapper, PGTableMapper, SqliteTableMapper,
};
// 用途：导入value宏
// 说明：用于构建查询条件
use rbs::value;
// 用途：导入序列化相关
// 说明：用于解析配置文件
use serde::Deserialize;
// 用途：导入ULID类型
// 说明：用于生成唯一标识符
use ulid::Ulid;

// 用途：权限配置结构体
// 说明：用于从配置文件解析权限数据
#[derive(Debug, Deserialize)]
struct PermissionConfig {
    // 用途：默认权限列表
    // 说明：系统启动时自动创建的默认权限
    #[serde(default)]
    default_permissions: Vec<PermissionItem>,
}

// 用途：权限项结构体
// 说明：单个权限的配置信息
#[derive(Debug, Deserialize)]
struct PermissionItem {
    // 用途：权限名称
    name: String,
    // 用途：权限标识
    permission: String,
    // 用途：权限路径
    #[serde(default)]
    path: Option<String>,
    // 用途：权限类型
    #[serde(default)]
    permission_type: Option<String>,
    // 用途：权限描述
    #[serde(default)]
    description: Option<String>,
    // 用途：排序字段
    #[serde(default)]
    sort_order: Option<i32>,
    // 用途：前端图标
    #[serde(default)]
    icon: Option<String>,
    // 用途：状态
    #[serde(default = "default_status")]
    status: i32,
}

// 用途：默认状态值
// 说明：权限默认启用
fn default_status() -> i32 {
    1
}

// 用途：同步数据库表结构
// 说明：根据表结构定义自动创建或更新数据库表
pub async fn sys_sync_tables(rb: &RBatis) {
    // 用途：获取日志拦截器
    // 说明：用于临时关闭日志输出
    let log_intercept = rb.get_intercept::<LogInterceptor>().expect("not find log interceptor");
    // 用途：保存当前日志级别
    // 说明：便于后续恢复
    let level = log_intercept.get_level_filter().clone();
    // 用途：关闭日志输出
    // 说明：避免表同步过程中产生大量日志，影响性能和可读性
    log_intercept.set_level_filter(LevelFilter::Off);
    // 用途：延迟恢复日志级别
    // 说明：确保无论函数如何退出，日志级别都会恢复
    defer!(|| {
        log_intercept.set_level_filter(level);
    });
    
    // 用途：根据数据库类型选择合适的表映射器
    // 说明：不同数据库的表结构定义语法不同，需要专用的映射器
    let mapper: &dyn ColumnMapper = {
        match rb.driver_type().unwrap_or_default() {
            "sqlite" => &SqliteTableMapper {},
            "mssql" => &MssqlTableMapper {},
            "mysql" => &MysqlTableMapper {},
            "postgres" => &PGTableMapper {},
            _ => {
                panic!("not find driver mapper")
            }
        }
    };
    
    // 用途：获取数据库连接
    // 说明：用于执行表同步操作
    let conn = rb.acquire().await.expect("connection database fail");
    
    // 用途：同步RBAC相关表结构
    // 说明：包含角色、权限和用户角色关联等表
    crate::domain::table::rbac::sync_tables(&conn, mapper).await;

    // 用途：同步系统组织表结构
    // 说明：存储系统组织架构信息
    crate::domain::table::basic::sys_organization::sync_organization_tables(&conn, mapper).await;

    // 用途：同步系统用户表结构
    // 说明：存储系统用户信息
    let table = SysUser {
        id: Some(Default::default()),
        account: Some(Default::default()),
        password: Some(Default::default()),
        name: Some(Default::default()),
        login_check: Some(Default::default()),
        state: Some(Default::default()),
        create_date: Some(Default::default()),
        balance: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "sys_user").await;
    
    // 用途：同步系统字典表结构
    // 说明：存储系统配置的字典数据
    let table = SysDict {
        id: Some(Default::default()),
        name: Some(Default::default()),
        code: Some(Default::default()),
        state: Some(Default::default()),
        create_date: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "sys_dict").await;
    
    // 用途：同步系统回收站表结构
    // 说明：用于存储被删除的数据，支持数据恢复
    let table = SysTrash {
        id: Some(Default::default()),
        table_name: Some(Default::default()),
        data: Some(Default::default()),
        create_date: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "sys_trash").await;

    // 用途：同步键值配置表结构
    // 说明：用于存储系统配置的键值对
    let table = KeyValueConfig {
        key: Default::default(),
        value: Default::default(),
        created_at: None,
        updated_at: None,
        description: None,
    };
    let _ = RBatis::sync(&conn, mapper, &table, "key_value_config").await;

    // 用途：再次同步RBAC相关表结构
    // 说明：确保RBAC表结构正确同步
    let _ = rbac::sync_tables(&conn, mapper).await;
}

// 用途：初始化系统默认数据
// 说明：从配置文件读取默认权限，创建默认角色和权限，确保系统能正常使用
pub async fn sys_sync_tables_data(rb: &RBatis) {
    // 用途：获取数据库连接
    // 说明：用于执行数据初始化操作
    let conn = rb.acquire().await.expect("init data fail");
    
    // 用途：初始化键值配置
    // 说明：确保is_init键存在，默认值为false
    let is_init_exists = KeyValueConfig::select_by_map(&conn, rbs::value!("key": "is_init")).await.unwrap_or_default();
    
    if is_init_exists.is_empty() {
        // 不存在is_init键，创建默认值
        let _ = KeyValueConfig::set_value(&conn, "is_init", "false", Some("系统初始化状态标记")).await;
    };
    
    // 用途：检查管理员角色是否已存在
    // 说明：避免重复初始化数据
    if let Ok(v) = RbacRole::select_by_map(&conn, value! {"id":"1"}).await {
        if v.len() > 0 {
            // 用途：如果角色已存在，直接返回
            // 说明：避免重复初始化数据
            return;
        }
    };

    // 用途：插入默认管理员角色
    // 说明：为管理员用户提供角色，实现RBAC权限控制
    let _ = RbacRole::insert(
        &conn,
        &RbacRole {
            id: Some(1.to_string()),
            name: Some("admin".to_string()),
            create_date: Some(DateTime::now()),
        },
    )
    .await;

    // 用途：从配置文件读取权限配置
    // 说明：使用配置文件替代硬编码的权限定义
    let permission_config = load_permission_config();
    
    // 用途：插入默认权限并关联到管理员角色
    // 说明：为管理员角色提供基本权限，确保系统功能可用
    let mut index = 1;
    for permission_item in permission_config.default_permissions {
        // 用途：生成权限ID
        // 说明：使用ULID生成唯一标识符
        let permission_id = Ulid::new().to_string();
        
        // 用途：创建权限对象
        // 说明：从配置项转换为数据库实体
        let permission = RbacPermission {
            id: Some(permission_id.clone()),
            name: Some(permission_item.name),
            permission: Some(permission_item.permission),
            path: permission_item.path,
            permission_type: permission_item.permission_type,
            description: permission_item.description,
            sort_order: permission_item.sort_order,
            icon: permission_item.icon,
            status: Some(permission_item.status),
            create_date: Some(DateTime::now()),
        };
        
        // 用途：插入权限
        // 说明：创建系统默认权限
        let _ = RbacPermission::insert(&conn, &permission).await;
        
        // 用途：关联角色和权限
        // 说明：将权限分配给管理员角色
        let role_permission = RbacRolePermission {
            id: Some(index.to_string()),
            role_id: Some(1.to_string()),
            permission_id: Some(permission_id),
            create_date: Some(DateTime::now()),
        };
        let _ = RbacRolePermission::insert(&conn, &role_permission).await;
        
        // 用途：递增索引
        // 说明：为下一条记录生成唯一ID
        index += 1;
    }
}

// 用途：加载权限配置文件
// 说明：从config/default_permissions.json5读取权限配置
fn load_permission_config() -> PermissionConfig {
    // 用途：读取配置文件内容
    // 说明：从默认路径读取权限配置
    let config_path = "config/default_permissions.json5";
    let config_content = std::fs::read_to_string(config_path)
        .unwrap_or_else(|_| {
            // 用途：配置文件读取失败时返回空配置
            // 说明：确保系统在配置文件缺失时仍能启动
            eprintln!("警告：无法读取权限配置文件 {}，将使用空配置", config_path);
            "{}".to_string()
        });
    
    // 用途：解析JSON5配置
    // 说明：将JSON5字符串解析为配置结构体
    let config: PermissionConfig = json5::from_str(&config_content)
        .unwrap_or_else(|e| {
            // 用途：配置解析失败时返回空配置
            // 说明：确保系统在配置文件格式错误时仍能启动
            eprintln!("警告：解析权限配置文件失败: {}，将使用空配置", e);
            PermissionConfig {
                default_permissions: vec![],
            }
        });
    
    config
}
