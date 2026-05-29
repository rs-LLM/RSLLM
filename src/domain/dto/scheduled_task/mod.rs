//! Scheduled task DTO module.
//! Defines request and query payloads for task creation, updates, listings, logs, and statistics.

use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct CreateTaskRequest {
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
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct UpdateTaskRequest {
    pub task_name: Option<String>,
    pub task_description: Option<String>,
    pub task_type: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<String>,
    pub hook_id: Option<String>,
    pub job_type: Option<String>,
    pub job_payload: Option<String>,
    pub condition_expression: Option<String>,
    pub condition_config: Option<String>,
    pub result_forward_config: Option<String>,
    pub notification_config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct ListTasksQuery {
    pub user_id: Option<String>,
    pub status: Option<String>,
    pub enabled: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct TaskLogsQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct TaskStatisticsQuery {
    pub task_id: Option<String>,
}
