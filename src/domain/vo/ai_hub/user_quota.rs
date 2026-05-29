//! 用户配额视图对象模块。
//! 定义 AI Hub 用户配额、配额概览与配额预警视图对象，供接口统一返回限额相关数据。

// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：用户配额视图对象结构体
// 说明：用于API响应的用户配额数据
/// 用户配额视图对象。
/// 表示用户在指定配额类型与周期下的限额配置、当前状态与速率限制信息。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AiHubUserQuotaVO {
    // 用途：配额ID
    pub id: Option<String>,
    // 用途：用户ID
    pub user_id: String,
    // 用途：配额类型
    pub quota_type: String,
    // 用途：周期开始时间
    pub cycle_start: Option<String>,
    // 用途：周期结束时间
    pub cycle_end: Option<String>,
    // 用途：状态
    pub status: String,
    // 用途：警告阈值（百分比）
    pub warning_threshold: Option<f64>,
    // 用途：是否需要警告
    pub need_warning: bool,
    // 用途：创建时间
    pub created_at: Option<String>,
    // 用途：RPM限制（每分钟请求数）
    pub rpm_limit: Option<i32>,
    // 用途：RPM已用（每分钟请求数）
    pub rpm_used: Option<i32>,
    // 用途：TPM限制（每分钟令牌数）
    pub tpm_limit: Option<i32>,
    // 用途：TPM已用（每分钟令牌数）
    pub tpm_used: Option<i32>,
}

// 用途：配额概览视图对象结构体
// 说明：用于展示用户配额的整体概览
/// 配额概览视图对象。
/// 表示用户当前活跃配额数量及配额明细集合。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QuotaOverviewVO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：活跃配额数量
    pub active_quota_count: i32,
    // 用途：配额列表
    pub quotas: Vec<AiHubUserQuotaVO>,
}

// 用途：配额警告视图对象结构体
// 说明：用于展示配额警告信息
/// 配额警告视图对象。
/// 表示当使用率接近或超过阈值时对用户配额发出的预警信息。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct QuotaWarningVO {
    // 用途：用户ID
    pub user_id: String,
    // 用途：配额ID
    pub quota_id: String,
    // 用途：配额类型
    pub quota_type: String,
    // 用途：当前使用率（百分比）
    pub current_usage_rate: f64,
    // 用途：警告阈值（百分比）
    pub warning_threshold: f64,
    // 用途：警告消息
    pub warning_message: String,
}
