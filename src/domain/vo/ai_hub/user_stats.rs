//! 用户统计VO模块
//! 提供用户面板统计相关的视图对象

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::admin_stats::TimeDimension;

/// 用户概览统计VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserStatsVO {
    /// 用户ID
    pub user_id: String,
    /// 当前余额
    pub balance: f64,
    /// 总消费
    pub total_cost: f64,
    /// 总输入tokens
    pub total_input_tokens: i64,
    /// 总输出tokens
    pub total_output_tokens: i64,
    /// 总tokens
    pub total_tokens: i64,
    /// 数据更新时间
    pub updated_at: String,
}

/// 用户趋势统计VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserTrendStatsVO {
    /// 用户ID
    pub user_id: String,
    /// 时间维度
    pub dimension: TimeDimension,
    /// Token使用趋势
    pub token_trend: Vec<UserTrendDataPointVO>,
    /// 消费趋势
    pub cost_trend: Vec<UserTrendDataPointVO>,
    /// 数据更新时间
    pub updated_at: String,
}

/// 用户趋势数据点VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserTrendDataPointVO {
    /// 时间标签
    pub label: String,
    /// 输入tokens
    pub input_tokens: i64,
    /// 输出tokens
    pub output_tokens: i64,
    /// 总tokens
    pub total_tokens: i64,
    /// 消费金额
    pub cost: f64,
}

/// 刷新统计响应VO
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RefreshStatsResponseVO {
    /// 是否刷新成功
    pub refreshed: bool,
    /// 刷新时间
    pub timestamp: String,
}
