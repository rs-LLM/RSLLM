// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：创建账单DTO
// 说明：用于创建新的账单
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateBillingDTO {
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
    // 用途：备注
    pub remark: Option<String>,
}

// 用途：更新账单DTO
// 说明：用于更新现有的账单
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateBillingDTO {
    // 用途：总费用（分）
    pub total_amount: Option<f64>,
    // 用途：服务费用（分）
    pub service_amount: Option<f64>,
    // 用途：税费（分）
    pub tax_amount: Option<f64>,
    // 用途：总请求数
    pub total_requests: Option<i64>,
    // 用途：总token数
    pub total_tokens: Option<i64>,
    // 用途：支付状态
    pub payment_status: Option<String>,
    // 用途：支付时间
    pub payment_time: Option<String>,
    // 用途：账单状态
    pub bill_status: Option<String>,
    // 用途：备注
    pub remark: Option<String>,
}

// 用途：账单支付DTO
// 说明：用于账单支付操作
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PayBillingDTO {
    // 用途：支付方式
    pub payment_method: String,
    // 用途：支付流水号
    pub payment_reference: Option<String>,
    // 用途：支付备注
    pub payment_remark: Option<String>,
}

// 用途：账单查询DTO
// 说明：用于查询账单列表
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingQueryDTO {
    // 用途：用户ID
    pub user_id: Option<String>,
    // 用途：账单周期
    pub billing_cycle: Option<String>,
    // 用途：支付状态
    pub payment_status: Option<String>,
    // 用途：账单状态
    pub bill_status: Option<String>,
    // 用途：开始时间
    pub start_time: Option<String>,
    // 用途：结束时间
    pub end_time: Option<String>,
    // 用途：页码
    pub page: Option<i64>,
    // 用途：每页数量
    pub page_size: Option<i64>,
}

// 用途：账单统计查询DTO
// 说明：用于查询账单统计数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BillingStatisticsQueryDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：统计周期
    pub period: Option<String>,
    // 用途：开始时间
    pub start_time: Option<String>,
    // 用途：结束时间
    pub end_time: Option<String>,
}