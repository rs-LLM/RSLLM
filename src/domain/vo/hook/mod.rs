//! Hook 视图对象模块。
//! 定义 Hook 注册、查询与事件管理场景的响应视图对象。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterHookResponse {
    pub hook_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UnregisterHookResponse {
    pub hook_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HookInfo {
    pub hook_id: String,
    pub hook_type: String,
    pub handler_name: String,
    pub config: Option<serde_json::Value>,
    pub registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListHooksResponse {
    pub hooks: Vec<HookInfo>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventInfo {
    pub event_id: String,
    pub hook_id: String,
    pub hook_type: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListEventsResponse {
    pub events: Vec<EventInfo>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClearEventsResponse {
    pub cleared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmitHookEventResponse {
    pub event_id: String,
    pub triggered_tasks: i64,
}
