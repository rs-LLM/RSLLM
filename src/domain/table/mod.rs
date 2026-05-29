//! Domain table module.
//! Exports table submodules and provides unified table/data sync entries.

/// Basic table module.
pub mod basic;
pub use basic::*;

// 导出rbac模块，用于直接访问RBAC相关表结构
pub use basic::rbac;

/// AI hub table module.
// 保留ai_hub模块的同步函数，但不导出其所有内容以避免名称冲突
pub mod ai_hub;

/// Scheduled task table module.
pub mod scheduled_task;

// 用途：统一的表结构同步入口
// 说明：同时调用basic、ai_hub和 scheduled_task 模块的表结构同步函数
pub async fn sync_tables(rb: &rbatis::RBatis) {
    // 调用basic模块的表结构同步
    basic::sys_sync_tables(rb).await;
    // 调用ai_hub模块的表结构同步
    ai_hub::ai_hub_sync_tables(rb).await;

    // 调用 scheduled_task 模块的表结构同步
    let mapper: &dyn rbatis::table_sync::ColumnMapper = {
        match rb.driver_type().unwrap_or_default() {
            "sqlite" => &rbatis::table_sync::SqliteTableMapper {},
            "mssql" => &rbatis::table_sync::MssqlTableMapper {},
            "mysql" => &rbatis::table_sync::MysqlTableMapper {},
            "postgres" => &rbatis::table_sync::PGTableMapper {},
            _ => {
                panic!("not find driver mapper")
            }
        }
    };
    scheduled_task::sync_scheduled_task_tables(rb, mapper).await;
}

// 用途：统一的表数据同步入口
// 说明：同时调用basic和ai_hub模块的表数据同步函数
pub async fn sync_tables_data(rb: &rbatis::RBatis) {
    // 调用basic模块的表数据同步
    basic::sys_sync_tables_data(rb).await;
    // 调用ai_hub模块的表数据同步
    ai_hub::ai_hub_sync_tables_data(rb).await;
}
