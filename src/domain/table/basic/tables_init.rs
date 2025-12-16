// 用途：导入密码检查枚举值
// 说明：用于初始化管理员用户的登录检查方式
use crate::domain::table::LoginCheck::PasswordCheck;
// 用途：导入RBAC模块
// 说明：用于调用RBAC相关的表同步功能
use crate::domain::table::rbac;
// 用途：导入RBAC相关表结构
// 说明：用于初始化系统默认的角色、权限和用户角色关联数据
use crate::domain::table::rbac::{RbacPermission, RbacRole, RbacRolePermission, RbacUserRole};
// 用途：导入系统字典表结构
// 说明：用于同步系统字典表结构
use crate::domain::table::sys_dict::SysDict;
// 用途：导入系统回收站表结构
// 说明：用于同步系统回收站表结构
use crate::domain::table::sys_trash::SysTrash;
// 用途：导入系统用户表结构
// 说明：用于同步系统用户表结构和初始化管理员用户
use crate::domain::table::sys_user::SysUser;
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

// 用途：同步数据库表结构
// 说明：根据表结构定义自动创建或更新数据库表
pub async fn sync_tables(rb: &RBatis) {
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

    // 用途：再次同步RBAC相关表结构
    // 说明：确保RBAC表结构正确同步
    let _ = rbac::sync_tables(&conn, mapper).await;
}

// 用途：初始化系统默认数据
// 说明：创建默认管理员用户、角色和权限，确保系统能正常使用
pub async fn sync_tables_data(rb: &RBatis) {
    // 用途：获取数据库连接
    // 说明：用于执行数据初始化操作
    let conn = rb.acquire().await.expect("init data fail");
    
    // 用途：检查管理员用户是否已存在
    // 说明：避免重复初始化数据
    if let Ok(v) = SysUser::select_by_map(&conn, value! {"id":"1"}).await {
        if v.len() > 0 {
            // 用途：如果用户已存在，直接返回
            // 说明：避免重复初始化数据
            return;
        }
    };
    
    // 用途：插入默认管理员用户
    // 说明：提供初始登录账号，方便管理员使用系统
    let _ = SysUser::insert(
        &conn,
        &SysUser {
            id: Some("1".to_string()),
            account: Some("00000000000".to_string()),
            password: Some("e10adc3949ba59abbe56e057f20f883e".to_string()), // 默认密码123456的MD5值
            name: Some("admin".to_string()),
            login_check: Some(PasswordCheck),
            state: Some(1), // 启用状态
            create_date: Some(DateTime::now()),
        },
    )
    .await;

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

    // 用途：关联管理员用户和角色
    // 说明：将管理员用户分配到管理员角色，获得角色对应的权限
    let _ = RbacUserRole::insert(
        &conn,
        &RbacUserRole {
            id: Some(1.to_string()),
            user_id: Some(1.to_string()),
            role_id: Some(1.to_string()),
            create_date: Some(DateTime::now()),
        },
    )
    .await;

    // 用途：定义默认系统权限
    // 说明：为管理员角色分配基本权限，确保系统功能可用
    let sys_permissions = vec![
        RbacPermission {
            id: Some(1.to_string()),
            name: Some("首页".to_string()),
            permission: Some("/".to_string()),
            path: Some("/".to_string()),
            create_date: Some(DateTime::now()),
        },
        RbacPermission {
            id: Some(9.to_string()),
            name: Some("user".to_string()),
            permission: Some("user".to_string()),
            path: Some("user".to_string()),
            create_date: Some(DateTime::now()),
        },
        RbacPermission {
            id: Some(10.to_string()),
            name: Some("setting".to_string()),
            permission: Some("setting".to_string()),
            path: Some("setting".to_string()),
            create_date: Some(DateTime::now()),
        },
    ];

    // 用途：插入默认权限并关联到管理员角色
    // 说明：为管理员角色提供基本权限，确保系统功能可用
    let mut index = 1;
    for permission in sys_permissions {
        // 用途：插入权限
        // 说明：创建系统默认权限
        let _ = RbacPermission::insert(&conn, &permission).await;
        
        // 用途：关联角色和权限
        // 说明：将权限分配给管理员角色
        let role_permission = RbacRolePermission {
            id: Some(index.to_string()),
            role_id: Some(1.to_string()),
            permission_id: permission.id.clone(),
            create_date: Some(DateTime::now()),
        };
        let _ = RbacRolePermission::insert(&conn, &role_permission).await;
        
        // 用途：递增索引
        // 说明：为下一条记录生成唯一ID
        index += 1;
    }
}
