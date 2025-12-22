// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：创建用户配额DTO
// 说明：用于创建新的用户配额
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateQuotaDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：配额类型
    pub quota_type: String,
    // 用途：总额度（分）
    pub total_quota: f64,
    // 用途：周期开始时间
    pub cycle_start: Option<String>,
    // 用途：周期结束时间
    pub cycle_end: Option<String>,
    // 用途：警告阈值（百分比）
    pub warning_threshold: Option<f64>,
}

// 用途：更新用户配额DTO
// 说明：用于更新现有的用户配额
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateQuotaDTO {
    // 用途：配额ID
    pub id: String,
    // 用途：总额度（分）
    pub total_quota: Option<f64>,
    // 用途：已用额度（分）
    pub used_quota: Option<f64>,
    // 用途：剩余额度（分）
    pub remaining_quota: Option<f64>,
    // 用途：周期开始时间
    pub cycle_start: Option<String>,
    // 用途：周期结束时间
    pub cycle_end: Option<String>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：警告阈值（百分比）
    pub warning_threshold: Option<f64>,
}

// 用途：配额充值DTO
// 说明：用于给用户配额充值
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RechargeQuotaDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：充值金额（分）
    pub amount: f64,
    // 用途：备注
    pub remark: Option<String>,
}

// 用途：配额扣减DTO
// 说明：用于扣减用户配额
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DeductQuotaDTO {
    // 用途：扣减金额（分）
    pub amount: f64,
    // 用途：关联的请求ID
    pub request_id: Option<String>,
    // 用途：备注
    pub remark: Option<String>,
}

// 用途：配额查询DTO
// 说明：用于查询用户配额列表
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QuotaQueryDTO {
    // 用途：用户ID
    pub user_id: Option<String>,
    // 用途：配额类型
    pub quota_type: Option<String>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：是否包含过期配额
    pub include_expired: Option<bool>,
    // 用途：页码
    pub page: Option<i64>,
    // 用途：每页数量
    pub page_size: Option<i64>,
}

// 用途：配额分配DTO
// 说明：用于给用户分配配额
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AllocateQuotaDTO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：配额类型
    pub quota_type: String,
    // 用途：分配额度（分）
    pub amount: f64,
    // 用途：是否覆盖现有配额
    pub overwrite: Option<bool>,
    // 用途：备注
    pub remark: Option<String>,
}