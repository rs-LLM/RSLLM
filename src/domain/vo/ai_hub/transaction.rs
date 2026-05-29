//! 交易与余额视图对象模块。
//! 定义 AI Hub 账户交易记录与余额视图对象，供接口统一返回账务相关数据。

// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Deserialize, Serialize};
// 用途：导入ToSchema trait
// 说明：用于OpenAPI文档生成
use utoipa::ToSchema;

// 用途：交易视图对象结构体
// 说明：用于API响应的交易记录数据
/// 交易记录视图对象。
/// 表示一次余额变动事件的完整信息，包括变动前后余额、操作人和原因。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TransactionVO {
    // 用途：交易ID
    pub id: Option<String>,
    // 用途：用户ID
    pub user_id: String,
    // 用途：用户名
    pub username: Option<String>,
    // 用途：交易类型
    pub transaction_type: String,
    // 用途：交易类型显示名称
    pub transaction_type_name: String,
    // 用途：交易金额
    pub amount: f64,
    // 用途：交易前余额
    pub balance_before: f64,
    // 用途：交易后余额
    pub balance_after: f64,
    // 用途：操作人ID
    pub operator_id: Option<String>,
    // 用途：操作人用户名
    pub operator_name: Option<String>,
    // 用途：原因
    pub reason: String,
    // 用途：创建时间
    pub created_at: Option<String>,
}

// 用途：余额视图对象结构体
// 说明：用于API响应的用户余额数据
/// 用户余额视图对象。
/// 表示用户当前可用余额及其基础身份信息。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BalanceVO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：用户名
    pub username: String,
    // 用途：余额
    pub balance: f64,
    pub subscription_quota_total: f64,
    pub subscription_quota_used: f64,
    pub subscription_quota_remaining: f64,
    pub subscription_status: String,
    pub subscription_expires_at: Option<String>,
    pub subscription_next_reset_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TransactionCycleSummaryVO {
    pub cycle_start: Option<String>,
    pub cycle_end: Option<String>,
    pub deduct_count: usize,
    pub wallet_deduct_amount: f64,
    pub subscription_deduct_amount: f64,
    pub total_deduct_amount: f64,
    pub wallet_deduct_ratio: f64,
}
