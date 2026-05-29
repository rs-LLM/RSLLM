use rbatis::crud;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskExecutionRecord {
    pub id: Option<String>,
    pub task_id: Option<String>,
    pub execution_id: Option<String>,
    pub status: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: Option<rbatis::rbdc::DateTime>,
    pub completed_at: Option<rbatis::rbdc::DateTime>,
    pub execution_time_ms: Option<i64>,
    pub created_at: Option<rbatis::rbdc::DateTime>,
}

impl Default for TaskExecutionRecord {
    fn default() -> Self {
        TaskExecutionRecord {
            id: None,
            task_id: None,
            execution_id: None,
            status: None,
            result: None,
            error_message: None,
            started_at: None,
            completed_at: None,
            execution_time_ms: None,
            created_at: None,
        }
    }
}

crud!(TaskExecutionRecord {});
