// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入ToSchema trait
// 说明：用于OpenAPI文档生成
use utoipa::ToSchema;

// 用途：创建交易DTO
// 说明：用于创建新的交易记录（充值、扣减、设置余额）
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateTransactionDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：交易类型（recharge/deduct/set）
    pub transaction_type: String,
    // 用途：交易金额
    pub amount: f64,
    // 用途：原因
    pub reason: String,
}

// 用途：查询交易DTO
// 说明：用于查询交易记录列表
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QueryTransactionDTO {
    // 用途：用户ID（可选）
    pub user_id: Option<String>,
    // 用途：交易类型（可选）
    pub transaction_type: Option<String>,
    // 用途：页码
    pub page: Option<u64>,
    // 用途：每页数量
    pub page_size: Option<u64>,
}

// 用途：充值DTO
// 说明：用于用户充值操作
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RechargeDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：充值金额
    pub amount: f64,
    // 用途：充值原因
    pub reason: String,
}

// 用途：扣减DTO
// 说明：用于余额扣减操作
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DeductDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：扣减金额
    pub amount: f64,
    // 用途：扣减原因
    pub reason: String,
}

// 用途：设置余额DTO
// 说明：用于管理员直接设置用户余额
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SetBalanceDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：新余额
    pub amount: f64,
    // 用途：设置原因
    pub reason: String,
}
