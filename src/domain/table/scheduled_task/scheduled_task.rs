use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: Option<String>,
    pub user_id: Option<String>,
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
    pub enabled: Option<i32>,
    pub next_run_time: Option<rbatis::rbdc::DateTime>,
    pub last_run_time: Option<rbatis::rbdc::DateTime>,
    pub run_count: Option<i32>,
    pub success_count: Option<i32>,
    pub failure_count: Option<i32>,
    pub status: Option<String>,
    pub created_at: Option<rbatis::rbdc::DateTime>,
    pub updated_at: Option<rbatis::rbdc::DateTime>,
}

impl Default for ScheduledTask {
    fn default() -> Self {
        ScheduledTask {
            id: None,
            user_id: None,
            task_name: None,
            task_description: None,
            task_type: None,
            trigger_type: None,
            trigger_config: None,
            hook_id: None,
            job_type: None,
            job_payload: None,
            condition_expression: None,
            condition_config: None,
            result_forward_config: None,
            notification_config: None,
            enabled: None,
            next_run_time: None,
            last_run_time: None,
            run_count: None,
            success_count: None,
            failure_count: None,
            status: None,
            created_at: None,
            updated_at: None,
        }
    }
}

crud!(ScheduledTask {});
