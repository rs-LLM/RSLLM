//! Hook DTO module.
//! Defines request and query payloads for hook registration, unregistration, listing, and event queries.

use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct RegisterHookRequest {
    pub hook_type: String,
    pub handler_name: String,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct UnregisterHookRequest {
    pub hook_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct ListHooksQuery {
    pub hook_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct ListEventsQuery {
    pub hook_id: Option<String>,
    pub hook_type: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct EmitHookEventRequest {
    pub hook_id: String,
    pub data: serde_json::Value,
}
