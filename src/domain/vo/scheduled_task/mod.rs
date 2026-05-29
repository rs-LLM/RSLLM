//! 定时任务视图对象模块。
//! 定义任务生命周期操作、任务列表、执行日志与统计响应场景使用的视图对象类型。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnableTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DisableTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunTaskResponse {
    pub task_id: String,
    pub execution_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskResponse {
    pub id: String,
    pub user_id: String,
    pub task_name: String,
    pub task_description: Option<String>,
    pub task_type: String,
    pub trigger_type: String,
    pub trigger_config: String,
    pub hook_id: Option<String>,
    pub job_type: String,
    pub job_payload: Option<String>,
    pub condition_expression: Option<String>,
    pub condition_config: Option<String>,
    pub result_forward_config: Option<String>,
    pub notification_config: Option<String>,
    pub enabled: bool,
    pub next_run_time: Option<String>,
    pub last_run_time: Option<String>,
    pub run_count: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskLogResponse {
    pub id: String,
    pub task_id: String,
    pub execution_id: String,
    pub status: String,
    pub result: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub execution_time_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskStatisticsResponse {
    pub task_id: String,
    pub total_runs: i32,
    pub successful_runs: i32,
    pub failed_runs: i32,
    pub success_rate: f64,
    pub average_execution_time_ms: f64,
    pub last_execution_time: Option<String>,
}
