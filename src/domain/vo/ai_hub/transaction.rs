// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Deserialize, Serialize};
// 用途：导入ToSchema trait
// 说明：用于OpenAPI文档生成
use utoipa::ToSchema;

// 用途：交易VO结构体
// 说明：用于API响应的交易记录数据
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

// 用途：余额VO结构体
// 说明：用于API响应的用户余额数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BalanceVO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：用户名
    pub username: String,
    // 用途：余额
    pub balance: f64,
}
