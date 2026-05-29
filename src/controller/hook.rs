use std::sync::{LazyLock, Mutex};

use ulid::Ulid;

use crate::context::ServiceContext;
use crate::domain::dto::hook::*;
use crate::domain::vo::hook::*;
use crate::error::Result;

#[derive(Clone, Debug)]
struct HookState {
    hook_id: String,
    hook_type: String,
    handler_name: String,
    config: Option<serde_json::Value>,
    registered_at: rbatis::rbdc::DateTime,
}

#[derive(Clone, Debug)]
struct HookEventState {
    event_id: String,
    hook_id: String,
    hook_type: String,
    timestamp: rbatis::rbdc::DateTime,
    data: serde_json::Value,
}

static HOOKS: LazyLock<Mutex<Vec<HookState>>> = LazyLock::new(|| Mutex::new(Vec::new()));
static EVENTS: LazyLock<Mutex<Vec<HookEventState>>> = LazyLock::new(|| Mutex::new(Vec::new()));

pub async fn register_hook(
    _state: &ServiceContext,
    params: RegisterHookRequest,
) -> Result<RegisterHookResponse> {
    let hook_id = Ulid::new().to_string();
    let now = rbatis::rbdc::DateTime::now();

    let hook = HookState {
        hook_id: hook_id.clone(),
        hook_type: params.hook_type,
        handler_name: params.handler_name,
        config: params.config,
        registered_at: now.clone(),
    };

    {
        let mut hooks = HOOKS
            .lock()
            .map_err(|e| crate::error::Error::E(format!("HOOKS 锁异常: {}", e)))?;
        hooks.push(hook.clone());
    }

    {
        let mut events = EVENTS
            .lock()
            .map_err(|e| crate::error::Error::E(format!("EVENTS 锁异常: {}", e)))?;
        events.push(HookEventState {
            event_id: Ulid::new().to_string(),
            hook_id: hook.hook_id.clone(),
            hook_type: hook.hook_type.clone(),
            timestamp: now.clone(),
            data: serde_json::json!({
                "action": "register",
                "handler_name": hook.handler_name,
                "config": hook.config,
            }),
        });
    }

    Ok(RegisterHookResponse { hook_id })
}

pub async fn unregister_hook(
    _state: &ServiceContext,
    params: UnregisterHookRequest,
) -> Result<UnregisterHookResponse> {
    let removed = {
        let mut hooks = HOOKS
            .lock()
            .map_err(|e| crate::error::Error::E(format!("HOOKS 锁异常: {}", e)))?;

        let idx_opt = hooks.iter().position(|h| h.hook_id == params.hook_id);
        idx_opt.map(|idx| hooks.remove(idx))
    };

    if let Some(hook) = removed {
        let now = rbatis::rbdc::DateTime::now();
        let mut events = EVENTS
            .lock()
            .map_err(|e| crate::error::Error::E(format!("EVENTS 锁异常: {}", e)))?;
        events.push(HookEventState {
            event_id: Ulid::new().to_string(),
            hook_id: hook.hook_id.clone(),
            hook_type: hook.hook_type.clone(),
            timestamp: now,
            data: serde_json::json!({
                "action": "unregister",
                "handler_name": hook.handler_name,
            }),
        });

        Ok(UnregisterHookResponse {
            hook_id: hook.hook_id,
        })
    } else {
        Ok(UnregisterHookResponse {
            hook_id: params.hook_id,
        })
    }
}

pub async fn list_hooks(
    _state: &ServiceContext,
    params: ListHooksQuery,
) -> Result<ListHooksResponse> {
    let hooks = HOOKS
        .lock()
        .map_err(|e| crate::error::Error::E(format!("HOOKS 锁异常: {}", e)))?
        .clone();

    let hooks: Vec<HookInfo> = hooks
        .into_iter()
        .filter(|h| {
            params
                .hook_type
                .as_ref()
                .map(|t| t == &h.hook_type)
                .unwrap_or(true)
        })
        .map(|h| HookInfo {
            hook_id: h.hook_id,
            hook_type: h.hook_type,
            handler_name: h.handler_name,
            config: h.config,
            registered_at: h.registered_at.to_string(),
        })
        .collect();

    Ok(ListHooksResponse {
        total: hooks.len() as i64,
        hooks,
    })
}

pub async fn list_events(
    _state: &ServiceContext,
    params: ListEventsQuery,
) -> Result<ListEventsResponse> {
    let mut events = EVENTS
        .lock()
        .map_err(|e| crate::error::Error::E(format!("EVENTS 锁异常: {}", e)))?
        .clone();

    events.retain(|e| {
        let hook_id_ok = params
            .hook_id
            .as_ref()
            .map(|id| id == &e.hook_id)
            .unwrap_or(true);
        let hook_type_ok = params
            .hook_type
            .as_ref()
            .map(|t| t == &e.hook_type)
            .unwrap_or(true);
        hook_id_ok && hook_type_ok
    });

    // 按时间倒序
    events.sort_by(|a, b| {
        b.timestamp
            .unix_timestamp_millis()
            .cmp(&a.timestamp.unix_timestamp_millis())
    });

    let limit = params.limit.unwrap_or(100).max(1).min(1000) as usize;
    let total = events.len() as i64;

    let events: Vec<EventInfo> = events
        .into_iter()
        .take(limit)
        .map(|e| EventInfo {
            event_id: e.event_id,
            hook_id: e.hook_id,
            hook_type: e.hook_type,
            timestamp: e.timestamp.to_string(),
            data: e.data,
        })
        .collect();

    Ok(ListEventsResponse { events, total })
}

pub async fn emit_event(
    state: &ServiceContext,
    params: EmitHookEventRequest,
) -> Result<EmitHookEventResponse> {
    let now = rbatis::rbdc::DateTime::now();
    let event_id = Ulid::new().to_string();

    let hook_type = {
        let hooks = HOOKS
            .lock()
            .map_err(|e| crate::error::Error::E(format!("HOOKS 锁异常: {}", e)))?;
        hooks
            .iter()
            .find(|h| h.hook_id == params.hook_id)
            .map(|h| h.hook_type.clone())
            .ok_or_else(|| {
                crate::error::Error::ValidationError(format!("hook_id 不存在: {}", params.hook_id))
            })?
    };

    // 记录事件
    {
        let mut events = EVENTS
            .lock()
            .map_err(|e| crate::error::Error::E(format!("EVENTS 锁异常: {}", e)))?;
        events.push(HookEventState {
            event_id: event_id.clone(),
            hook_id: params.hook_id.clone(),
            hook_type,
            timestamp: now,
            data: serde_json::json!({
                "action": "emit",
                "payload": params.data.clone()
            }),
        });
    }

    // 触发绑定该 hook_id 的任务（一期：直接触发执行）
    let triggered_tasks = crate::controller::scheduled_task::trigger_tasks_by_hook(
        state,
        &params.hook_id,
        &params.data,
    )
    .await?;

    Ok(EmitHookEventResponse {
        event_id,
        triggered_tasks,
    })
}
pub async fn clear_events(_state: &ServiceContext) -> Result<ClearEventsResponse> {
    let mut events = EVENTS
        .lock()
        .map_err(|e| crate::error::Error::E(format!("EVENTS 锁异常: {}", e)))?;
    events.clear();

    Ok(ClearEventsResponse { cleared: true })
}
