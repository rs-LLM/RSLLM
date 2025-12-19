// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：账单VO结构体
// 说明：用于API响应的账单数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AiHubBillingVO {
    // 用途：账单ID
    pub id: Option<String>,
    // 用途：账单编号
    pub bill_number: String,
    // 用途：用户ID
    pub user_id: String,
    // 用途：账单周期
    pub billing_cycle: String,
    // 用途：总费用（分）
    pub total_amount: f64,
    // 用途：服务费用（分）
    pub service_amount: f64,
    // 用途：税费（分）
    pub tax_amount: f64,
    // 用途：总请求数
    pub total_requests: i64,
    // 用途：总token数
    pub total_tokens: i64,
    // 用途：支付状态
    pub payment_status: String,
    // 用途：支付时间
    pub payment_time: Option<String>,
    // 用途：账单状态
    pub bill_status: String,
    // 用途：备注
    pub remark: Option<String>,
    // 用途：创建时间
    pub created_at: Option<String>,
}

// 用途：账单概览VO结构体
// 说明：用于展示账单的整体概览
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingOverviewVO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：总账单数量
    pub total_bills: i32,
    // 用途：待支付账单数量
    pub pending_bills: i32,
    // 用途：已支付账单数量
    pub paid_bills: i32,
    // 用途：总费用（分）
    pub total_amount: f64,
    // 用途：已支付费用（分）
    pub paid_amount: f64,
    // 用途：待支付费用（分）
    pub pending_amount: f64,
    // 用途：账单列表
    pub bills: Vec<AiHubBillingVO>,
}

// 用途：账单统计VO结构体
// 说明：用于统计特定周期内的账单数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingStatisticsVO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：统计周期
    pub period: String,
    // 用途：总费用（分）
    pub total_amount: f64,
    // 用途：平均费用（分）
    pub average_amount: f64,
    // 用途：总请求数
    pub total_requests: i64,
    // 用途：总token数
    pub total_tokens: i64,
    // 用途：平均token数
    pub average_tokens: i64,
    // 用途：账单数量
    pub bill_count: i32,
}