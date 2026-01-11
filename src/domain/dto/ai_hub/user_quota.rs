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
    // 用途：周期开始时间
    pub cycle_start: Option<String>,
    // 用途：周期结束时间
    pub cycle_end: Option<String>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：警告阈值（百分比）
    pub warning_threshold: Option<f64>,
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
