// 用途：导入日期时间类型
// 说明：用于记录账单周期、支付时间等
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：账单表结构体
// 说明：记录周期性账单信息，包括费用统计和支付状态
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHubBilling {
    // 用途：账单ID
    // 说明：账单的唯一标识符
    pub id: Option<String>,
    // 用途：账单编号
    // 说明：账单的业务编号，用于显示和查询
    pub bill_number: String,
    // 用途：用户ID
    // 说明：账单所属用户的唯一标识符
    pub user_id: String,
    // 用途：账单周期
    // 说明：账单的周期，如"2024-01"、"2024-Q1"等
    pub billing_cycle: String,
    // 用途：总费用（分）
    // 说明：账单总金额，单位为分
    pub total_amount: f64,
    // 用途：服务费用（分）
    // 说明：AI服务产生的费用，单位为分
    pub service_amount: f64,
    // 用途：税费（分）
    // 说明：税费金额，单位为分
    pub tax_amount: f64,
    // 用途：总请求数
    // 说明：账单周期内的总请求次数
    pub total_requests: i64,
    // 用途：总token数
    // 说明：账单周期内的总token消耗量
    pub total_tokens: i64,
    // 用途：支付状态
    // 说明：账单的支付状态，如"pending"、"paid"、"failed"、"cancelled"等
    pub payment_status: String,
    // 用途：支付时间
    // 说明：账单完成支付的时间
    pub payment_time: Option<DateTime>,
    // 用途：账单状态
    // 说明：账单的整体状态，如"draft"、"issued"、"paid"、"cancelled"等
    pub bill_status: String,
    // 用途：备注
    // 说明：账单的备注信息
    pub remark: Option<String>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成AiHubBilling的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(AiHubBilling {});