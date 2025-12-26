// 用途：导入日期时间类型
// 说明：用于记录交易时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：交易类型枚举
// 说明：定义交易的不同类型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    // 用途：充值
    // 说明：用户余额增加
    #[serde(rename = "recharge")]
    Recharge,
    // 用途：扣减
    // 说明：用户余额减少，通常用于API调用计费
    #[serde(rename = "deduct")]
    Deduct,
    // 用途：设置
    // 说明：管理员直接设置用户余额
    #[serde(rename = "set")]
    Set,
}

// 用途：交易表结构体
// 说明：记录用户余额的所有变更历史，支持账单查询和审计
// 注意：字段名使用驼峰命名以匹配数据库表结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    // 用途：交易ID
    // 说明：交易记录的唯一标识符
    pub id: Option<String>,
    // 用途：用户ID
    // 说明：交易所属用户的唯一标识符
    pub userId: String,
    // 用途：交易类型
    // 说明：交易类型，包括充值、扣减、设置
    #[serde(rename = "type")]
    pub type_: String,
    // 用途：交易金额
    // 说明：交易的金额，正数表示增加，负数表示减少
    pub amount: f64,
    // 用途：交易前余额
    // 说明：交易执行前的用户余额
    pub balanceBefore: f64,
    // 用途：交易后余额
    // 说明：交易执行后的用户余额
    pub balanceAfter: f64,
    // 用途：操作人ID
    // 说明：执行交易操作的用户ID，系统自动操作时为空
    pub operatorId: Option<String>,
    // 用途：原因
    // 说明：交易的原因或备注
    pub reason: String,
    // 用途：创建时间
    // 说明：交易记录的创建时间
    pub createdAt: Option<DateTime>,
}

// 用途：生成Transaction的CRUD操作宏
// 说明：自动实现增删改查等基本操作，使用自定义表名"user_transaction"避免SQL保留字冲突
crud!(Transaction {}, "user_transaction");
