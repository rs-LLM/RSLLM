//! 统计分析视图对象模块。
//! 定义 AI Hub 统计分析场景使用的响应视图对象。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 用户用量统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserUsageStatsVO {
    /// 用户ID
    pub user_id: String,
    /// 总请求数
    pub total_requests: i64,
    /// 总输入tokens
    pub total_input_tokens: i64,
    /// 总输出tokens
    pub total_output_tokens: i64,
    /// 总tokens
    pub total_tokens: i64,
    /// 总费用
    pub total_cost: f64,
    /// 平均响应时间（毫秒）
    pub avg_response_time: f64,
    /// 模型统计
    pub model_stats: Vec<ModelStatsVO>,
    /// 日统计
    pub daily_stats: Vec<DailyStatsVO>,
}

/// 模型统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ModelStatsVO {
    /// 模型ID
    pub model_id: String,
    /// 请求数
    pub requests: i64,
    /// 输入tokens
    pub input_tokens: i64,
    /// 输出tokens
    pub output_tokens: i64,
    /// 费用
    pub cost: f64,
}

/// 日统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DailyStatsVO {
    /// 日期
    pub date: String,
    /// 请求数
    pub requests: i64,
    /// 输入tokens
    pub input_tokens: i64,
    /// 输出tokens
    pub output_tokens: i64,
    /// 费用
    pub cost: f64,
}

/// 系统统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SystemStatsVO {
    /// 总请求数
    pub total_requests: i64,
    /// 总tokens
    pub total_tokens: i64,
    /// 总收入
    pub total_revenue: f64,
    /// 活跃用户数
    pub active_users: i64,
    /// 热门用户
    pub top_users: Vec<TopUserStatsVO>,
    /// 热门模型
    pub top_models: Vec<ModelStatsVO>,
}

/// 热门用户统计视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct TopUserStatsVO {
    /// 用户ID
    pub user_id: String,
    /// 请求数
    pub requests: i64,
    /// tokens
    pub tokens: i64,
    /// 费用
    pub cost: f64,
}

/// 性能趋势视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PerformanceTrendsVO {
    /// 成功率（百分比）
    pub success_rate: f64,
    /// 平均响应时间（毫秒）
    pub avg_response_time: f64,
    /// 小时趋势
    pub hourly_trends: Vec<HourlyTrendVO>,
}

/// 小时趋势视图对象
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct HourlyTrendVO {
    /// 小时
    pub hour: String,
    /// 请求数
    pub requests: i64,
    /// 平均响应时间（毫秒）
    pub avg_response_time: f64,
    /// 错误率（百分比）
    pub error_rate: f64,
}
