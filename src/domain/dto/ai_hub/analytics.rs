//! 统计分析DTO模块
//! 提供统计分析相关的数据传输对象

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 用户用量统计查询DTO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserUsageStatsQueryDTO {
    /// 用户ID
    pub user_id: Option<String>,
    /// 开始时间 (可选)
    pub start_time: Option<String>,
    /// 结束时间 (可选)
    pub end_time: Option<String>,
    /// 模型ID (可选)
    pub model_id: Option<String>,
}

/// 系统统计查询DTO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SystemStatsQueryDTO {
    /// 开始时间 (可选)
    pub start_time: Option<String>,
    /// 结束时间 (可选)
    pub end_time: Option<String>,
}

/// 性能趋势查询DTO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PerformanceTrendsQueryDTO {
    /// 天数 (默认7天)
    pub days: Option<i64>,
}
