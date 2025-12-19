mod basic;
pub use basic::*;

// 保留ai_hub模块的同步函数，但不导出其所有内容以避免名称冲突
pub mod ai_hub;

// 供应商相关的表结构，公开导出
pub mod provider;

// 用途：统一的表结构同步入口
// 说明：同时调用basic和ai_hub模块的表结构同步函数
pub async fn sync_tables(rb: &rbatis::RBatis) {
    // 调用basic模块的表结构同步
    basic::sys_sync_tables(rb).await;
    // 调用ai_hub模块的表结构同步
    ai_hub::ai_hub_sync_tables(rb).await;
}

// 用途：统一的表数据同步入口
// 说明：同时调用basic和ai_hub模块的表数据同步函数
pub async fn sync_tables_data(rb: &rbatis::RBatis) {
    // 调用basic模块的表数据同步
    basic::sys_sync_tables_data(rb).await;
    // 调用ai_hub模块的表数据同步
    ai_hub::ai_hub_sync_tables_data(rb).await;
}
