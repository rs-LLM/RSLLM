use std::sync::Arc;

use ulid::Ulid;

use crate::context::ServiceContext;
use crate::domain::dto::scheduled_task::*;
use crate::domain::table::scheduled_task::{ScheduledTask, TaskExecutionRecord};
use crate::domain::vo::response::PageWrapper;
use crate::domain::vo::scheduled_task::*;
use crate::error::{Error, Result};
use crate::pool;

fn resolve_page_i64(page: Option<i64>) -> Result<i64> {
    let page = page.unwrap_or(1);
    if page < 1 {
        return Err(Error::ValidationError(format!(
            "page must be >= 1, got {}",
            page
        )));
    }

    Ok(page)
}

fn resolve_page_size_i64(page_size: Option<i64>) -> Result<i64> {
    let page_size = page_size.unwrap_or(20);
    if page_size < 1 {
        return Err(Error::ValidationError(format!(
            "page_size must be >= 1, got {}",
            page_size
        )));
    }

    Ok(page_size)
}

fn resolve_page_u64(page: Option<i64>) -> Result<u64> {
    let page = resolve_page_i64(page)?;
    u64::try_from(page).map_err(|_| Error::ValidationError("page is too large".to_string()))
}

fn resolve_page_size_u64(page_size: Option<i64>) -> Result<u64> {
    let page_size = resolve_page_size_i64(page_size)?;
    u64::try_from(page_size)
        .map_err(|_| Error::ValidationError("page_size is too large".to_string()))
}

fn normalize_trigger_binding(
    trigger_type: &str,
    trigger_config: &str,
    hook_id: Option<String>,
) -> Result<(String, Option<String>)> {
    if trigger_type != "hook" {
        return Ok((trigger_config.to_string(), None));
    }

    let parsed = if trigger_config.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str::<serde_json::Value>(trigger_config)
            .map_err(|e| Error::ValidationError(format!("trigger_config 必须是合法 JSON: {}", e)))?
    };

    let resolved_hook_id = hook_id
        .or_else(|| {
            parsed
                .get("hook_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| Error::ValidationError("hook 触发任务必须提供 hook_id".to_string()))?;

    let mut object = parsed.as_object().cloned().unwrap_or_default();
    object.insert(
        "hook_id".to_string(),
        serde_json::Value::String(resolved_hook_id.clone()),
    );

    Ok((
        serde_json::to_string(&serde_json::Value::Object(object))
            .map_err(|e| Error::E(format!("序列化 trigger_config 失败: {}", e)))?,
        Some(resolved_hook_id),
    ))
}

async fn execute_task_async(
    state: ServiceContext,
    task: ScheduledTask,
    task_id: String,
    execution_id: String,
) {
    let execution_id_for_job = execution_id.clone();
    tokio::spawn(async move {
        let started_at = rbatis::rbdc::DateTime::now();

        let mut status = "failed".to_string();
        let mut result: Option<serde_json::Value> = None;
        let mut error_message: Option<String> = None;

        let job_type = task.job_type.clone().unwrap_or_default();
        let job_payload = task.job_payload.clone();

        match job_type.as_str() {
            "status_probe" => {
                let timeout_ms = job_payload
                    .as_deref()
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|v| v.clamp(1, 10_000));

                match crate::controller::status_controller::probe_overview(
                    &Arc::new(state),
                    None,
                    None,
                    timeout_ms,
                )
                .await
                {
                    Ok(data) => {
                        status = "success".to_string();
                        result = serde_json::to_value(data).ok();
                    }
                    Err(e) => {
                        error_message = Some(format!("status_probe failed: {}", e));
                    }
                }
            }
            _ => {
                error_message = Some(format!("未知的 job_type: {}", job_type));
            }
        }

        let completed_at = rbatis::rbdc::DateTime::now();
        let execution_time_ms =
            (completed_at.unix_timestamp_millis() - started_at.unix_timestamp_millis()).max(0);

        let record = TaskExecutionRecord {
            id: Some(Ulid::new().to_string()),
            task_id: Some(task_id.clone()),
            execution_id: Some(execution_id_for_job.clone()),
            status: Some(status.clone()),
            result,
            error_message,
            started_at: Some(started_at),
            completed_at: Some(completed_at),
            execution_time_ms: Some(execution_time_ms),
            created_at: Some(rbatis::rbdc::DateTime::now()),
        };

        if let Err(e) = TaskExecutionRecord::insert(pool!(), &record).await {
            log::error!(
                "[ScheduledTask] Failed to insert execution record task_id={} execution_id={} err={}",
                task_id,
                execution_id_for_job,
                e
            );
        }

        log::info!(
            "[ScheduledTask] Finished task_id={} execution_id={} status={}",
            task_id,
            execution_id_for_job,
            status
        );
    });
}

pub async fn trigger_tasks_by_hook(
    state: &ServiceContext,
    hook_id: &str,
    _event_data: &serde_json::Value,
) -> Result<i64> {
    let tasks = ScheduledTask::select_by_map(
        pool!(),
        rbs::value! {
            "enabled": 1,
            "trigger_type": "hook",
            "hook_id": hook_id,
        },
    )
    .await
    .map_err(|e| Error::E(format!("Failed to fetch hook tasks: {}", e)))?;

    let count = tasks.len() as i64;
    for task in tasks {
        let task_id = task.id.clone().unwrap_or_default();
        if task_id.is_empty() {
            continue;
        }
        let execution_id = Ulid::new().to_string();
        execute_task_async(state.clone(), task, task_id, execution_id).await;
    }

    Ok(count)
}

pub async fn create_task(
    _state: &ServiceContext,
    params: CreateTaskRequest,
) -> Result<CreateTaskResponse> {
    let task_id = Ulid::new().to_string();
    let now = rbatis::rbdc::DateTime::now();
    let (trigger_config, hook_id) =
        normalize_trigger_binding(&params.trigger_type, &params.trigger_config, params.hook_id)?;

    let task = ScheduledTask {
        id: Some(task_id.clone()),
        user_id: Some(params.user_id),
        task_name: Some(params.task_name),
        task_description: params.task_description,
        task_type: Some(params.task_type),
        trigger_type: Some(params.trigger_type),
        trigger_config: Some(trigger_config),
        hook_id,
        job_type: Some(params.job_type),
        job_payload: params.job_payload,
        condition_expression: params.condition_expression,
        condition_config: params.condition_config,
        result_forward_config: params.result_forward_config,
        notification_config: params.notification_config,
        enabled: Some(0),
        next_run_time: Some(now.clone()),
        last_run_time: None,
        run_count: Some(0),
        success_count: Some(0),
        failure_count: Some(0),
        status: Some("pending".to_string()),
        created_at: Some(now.clone()),
        updated_at: Some(now),
    };

    ScheduledTask::insert(pool!(), &task)
        .await
        .map_err(|e| Error::E(format!("Failed to create task: {}", e)))?;

    Ok(CreateTaskResponse { task_id })
}

pub async fn list_tasks(
    _state: &ServiceContext,
    params: ListTasksQuery,
) -> Result<ListTasksResponse> {
    let page = resolve_page_i64(params.page)?;
    let page_size = resolve_page_size_i64(params.page_size)?;
    let offset = page.saturating_sub(1).saturating_mul(page_size);

    let mut conditions: Vec<String> = Vec::new();
    let mut args: Vec<rbs::Value> = Vec::new();

    if let Some(user_id) = &params.user_id {
        args.push(rbs::value!(user_id));
        conditions.push(format!("user_id = ${}", args.len()));
    }

    if let Some(status) = &params.status {
        args.push(rbs::value!(status));
        conditions.push(format!("status = ${}", args.len()));
    }

    if let Some(enabled) = params.enabled {
        let enabled_value = if enabled { 1 } else { 0 };
        args.push(rbs::value!(enabled_value));
        conditions.push(format!("enabled = ${}", args.len()));
    }

    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let query = format!(
        "SELECT * FROM scheduled_task{} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        where_clause,
        args.len() + 1,
        args.len() + 2
    );

    let count_query = format!(
        "SELECT COUNT(*) as total FROM scheduled_task{}",
        where_clause
    );

    let mut list_args = args.clone();
    list_args.push(rbs::value!(page_size));
    list_args.push(rbs::value!(offset));

    let tasks_result = pool!()
        .query(&query, list_args)
        .await
        .map_err(|e| Error::E(format!("Failed to fetch tasks: {}", e)))?;

    let tasks: Vec<ScheduledTask> = if let Some(arr) = tasks_result.as_array() {
        arr.iter()
            .filter_map(|item| {
                let json_value = serde_json::to_value(item).ok()?;
                serde_json::from_value(json_value).ok()
            })
            .collect()
    } else {
        Vec::new()
    };

    let total_result = pool!()
        .query(&count_query, args)
        .await
        .map_err(|e| Error::E(format!("Failed to fetch task count: {}", e)))?;

    let total = if let Some(arr) = total_result.as_array() {
        arr.first()
            .and_then(|v| {
                let json_str = serde_json::to_string(v).ok()?;
                let json_obj: serde_json::Value = serde_json::from_str(&json_str).ok()?;
                json_obj.get("total").and_then(|t| t.as_i64())
            })
            .unwrap_or(0)
    } else {
        0
    };

    let task_responses: Vec<TaskResponse> = tasks
        .into_iter()
        .map(|t| TaskResponse {
            id: t.id.unwrap_or_default(),
            user_id: t.user_id.unwrap_or_default(),
            task_name: t.task_name.unwrap_or_default(),
            task_description: t.task_description,
            task_type: t.task_type.unwrap_or_default(),
            trigger_type: t.trigger_type.unwrap_or_default(),
            trigger_config: t.trigger_config.unwrap_or_default(),
            hook_id: t.hook_id,
            job_type: t.job_type.unwrap_or_default(),
            job_payload: t.job_payload,
            condition_expression: t.condition_expression,
            condition_config: t.condition_config,
            result_forward_config: t.result_forward_config,
            notification_config: t.notification_config,
            enabled: t.enabled.unwrap_or(0) == 1,
            next_run_time: t.next_run_time.map(|dt| dt.to_string()),
            last_run_time: t.last_run_time.map(|dt| dt.to_string()),
            run_count: t.run_count.unwrap_or(0),
            success_count: t.success_count.unwrap_or(0),
            failure_count: t.failure_count.unwrap_or(0),
            status: t.status.unwrap_or_default(),
            created_at: t.created_at.map(|dt| dt.to_string()).unwrap_or_default(),
            updated_at: t.updated_at.map(|dt| dt.to_string()).unwrap_or_default(),
        })
        .collect();

    Ok(ListTasksResponse {
        tasks: task_responses,
        total,
        page,
        page_size,
    })
}

pub async fn get_task(_state: &ServiceContext, task_id: &str) -> Result<TaskResponse> {
    let task_opt = ScheduledTask::select_by_map(pool!(), rbs::value! { "id": task_id })
        .await
        .map_err(|e| Error::E(format!("Failed to fetch task: {}", e)))?
        .first()
        .cloned();

    let task = match task_opt {
        Some(t) => t,
        None => return Err(Error::E(format!("Task not found: {}", task_id))),
    };

    Ok(TaskResponse {
        id: task.id.unwrap_or_default(),
        user_id: task.user_id.unwrap_or_default(),
        task_name: task.task_name.unwrap_or_default(),
        task_description: task.task_description,
        task_type: task.task_type.unwrap_or_default(),
        trigger_type: task.trigger_type.unwrap_or_default(),
        trigger_config: task.trigger_config.unwrap_or_default(),
        hook_id: task.hook_id,
        job_type: task.job_type.unwrap_or_default(),
        job_payload: task.job_payload,
        condition_expression: task.condition_expression,
        condition_config: task.condition_config,
        result_forward_config: task.result_forward_config,
        notification_config: task.notification_config,
        enabled: task.enabled.unwrap_or(0) == 1,
        next_run_time: task.next_run_time.map(|dt| dt.to_string()),
        last_run_time: task.last_run_time.map(|dt| dt.to_string()),
        run_count: task.run_count.unwrap_or(0),
        success_count: task.success_count.unwrap_or(0),
        failure_count: task.failure_count.unwrap_or(0),
        status: task.status.unwrap_or_default(),
        created_at: task.created_at.map(|dt| dt.to_string()).unwrap_or_default(),
        updated_at: task.updated_at.map(|dt| dt.to_string()).unwrap_or_default(),
    })
}

pub async fn update_task(
    _state: &ServiceContext,
    task_id: &str,
    params: UpdateTaskRequest,
) -> Result<UpdateTaskResponse> {
    let task_opt = ScheduledTask::select_by_map(pool!(), rbs::value! { "id": task_id })
        .await
        .map_err(|e| Error::E(format!("Failed to fetch task: {}", e)))?
        .first()
        .cloned();

    let mut task = match task_opt {
        Some(t) => t,
        None => return Err(Error::E(format!("Task not found: {}", task_id))),
    };

    if let Some(task_name) = params.task_name {
        task.task_name = Some(task_name);
    }
    if let Some(task_description) = params.task_description {
        task.task_description = Some(task_description);
    }
    if let Some(task_type) = params.task_type {
        task.task_type = Some(task_type);
    }
    if let Some(trigger_type) = params.trigger_type {
        task.trigger_type = Some(trigger_type);
    }
    let trigger_config_for_normalize = params
        .trigger_config
        .as_deref()
        .unwrap_or(task.trigger_config.as_deref().unwrap_or_default());
    let (normalized_trigger_config, normalized_hook_id) = normalize_trigger_binding(
        task.trigger_type.as_deref().unwrap_or_default(),
        trigger_config_for_normalize,
        params.hook_id.or(task.hook_id.clone()),
    )?;
    task.trigger_config = Some(normalized_trigger_config);
    task.hook_id = normalized_hook_id;

    if let Some(job_type) = params.job_type {
        task.job_type = Some(job_type);
    }
    if let Some(job_payload) = params.job_payload {
        task.job_payload = Some(job_payload);
    }
    if let Some(condition_expression) = params.condition_expression {
        task.condition_expression = Some(condition_expression);
    }
    if let Some(condition_config) = params.condition_config {
        task.condition_config = Some(condition_config);
    }
    if let Some(result_forward_config) = params.result_forward_config {
        task.result_forward_config = Some(result_forward_config);
    }
    if let Some(notification_config) = params.notification_config {
        task.notification_config = Some(notification_config);
    }

    let now = rbatis::rbdc::DateTime::now();
    task.updated_at = Some(now.clone());

    pool!().exec(
        "UPDATE scheduled_task SET task_name=$1, task_description=$2, task_type=$3, trigger_type=$4, trigger_config=$5, hook_id=$6, job_type=$7, job_payload=$8, condition_expression=$9, condition_config=$10, result_forward_config=$11, notification_config=$12, updated_at=$13 WHERE id=$14",
        vec![
            rbs::value!(task.task_name.unwrap_or_default()),
            rbs::value!(task.task_description.clone().unwrap_or_default()),
            rbs::value!(task.task_type.unwrap_or_default()),
            rbs::value!(task.trigger_type.unwrap_or_default()),
            rbs::value!(task.trigger_config.unwrap_or_default()),
            rbs::value!(task.hook_id.clone().unwrap_or_default()),
            rbs::value!(task.job_type.unwrap_or_default()),
            rbs::value!(task.job_payload.clone().unwrap_or_default()),
            rbs::value!(task.condition_expression.clone().unwrap_or_default()),
            rbs::value!(task.condition_config.clone().unwrap_or_default()),
            rbs::value!(task.result_forward_config.clone().unwrap_or_default()),
            rbs::value!(task.notification_config.clone().unwrap_or_default()),
            rbs::value!(now),
            rbs::value!(task_id),
        ]
    ).await
        .map_err(|e| Error::E(format!("Failed to update task: {}", e)))?;

    Ok(UpdateTaskResponse {
        task_id: task_id.to_string(),
    })
}

pub async fn delete_task(_state: &ServiceContext, task_id: &str) -> Result<DeleteTaskResponse> {
    let _task_opt = ScheduledTask::select_by_map(pool!(), rbs::value! { "id": task_id })
        .await
        .map_err(|e| Error::E(format!("Failed to fetch task: {}", e)))?
        .first()
        .cloned()
        .ok_or_else(|| Error::E(format!("Task not found: {}", task_id)))?;

    pool!()
        .exec(
            "DELETE FROM scheduled_task WHERE id=$1",
            vec![rbs::value!(task_id)],
        )
        .await
        .map_err(|e| Error::E(format!("Failed to delete task: {}", e)))?;

    Ok(DeleteTaskResponse {
        task_id: task_id.to_string(),
    })
}

pub async fn enable_task(_state: &ServiceContext, task_id: &str) -> Result<EnableTaskResponse> {
    let now = rbatis::rbdc::DateTime::now();
    pool!()
        .exec(
            "UPDATE scheduled_task SET enabled=1, status='pending', updated_at=$1 WHERE id=$2",
            vec![rbs::value!(now), rbs::value!(task_id)],
        )
        .await
        .map_err(|e| Error::E(format!("Failed to enable task: {}", e)))?;

    Ok(EnableTaskResponse {
        task_id: task_id.to_string(),
    })
}

pub async fn disable_task(_state: &ServiceContext, task_id: &str) -> Result<DisableTaskResponse> {
    let now = rbatis::rbdc::DateTime::now();
    pool!()
        .exec(
            "UPDATE scheduled_task SET enabled=0, status='disabled', updated_at=$1 WHERE id=$2",
            vec![rbs::value!(now), rbs::value!(task_id)],
        )
        .await
        .map_err(|e| Error::E(format!("Failed to disable task: {}", e)))?;

    Ok(DisableTaskResponse {
        task_id: task_id.to_string(),
    })
}

pub async fn run_task_now(state: &ServiceContext, task_id: &str) -> Result<RunTaskResponse> {
    let task_opt = ScheduledTask::select_by_map(pool!(), rbs::value! { "id": task_id })
        .await
        .map_err(|e| Error::E(format!("Failed to fetch task: {}", e)))?
        .first()
        .cloned();

    let task = match task_opt {
        Some(t) => t,
        None => return Err(Error::E(format!("Task not found: {}", task_id))),
    };

    if task.enabled != Some(1) {
        return Err(Error::E(format!("Task is not enabled: {}", task_id)));
    }

    let execution_id = ulid::Ulid::new().to_string();
    execute_task_async(
        state.clone(),
        task,
        task_id.to_string(),
        execution_id.clone(),
    )
    .await;

    Ok(RunTaskResponse {
        task_id: task_id.to_string(),
        execution_id,
    })
}

pub async fn get_task_logs(
    _state: &ServiceContext,
    task_id: &str,
    params: TaskLogsQuery,
) -> Result<PageWrapper<TaskLogResponse>> {
    let page = resolve_page_u64(params.page)?;
    let page_size = resolve_page_size_u64(params.page_size)?;
    let offset = page.saturating_sub(1).saturating_mul(page_size);

    let logs_result = pool!().query(
        "SELECT * FROM task_execution_record WHERE task_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        vec![rbs::value!(task_id), rbs::value!(page_size), rbs::value!(offset)]
    ).await
        .map_err(|e| Error::E(format!("Failed to fetch task logs: {}", e)))?;

    let logs: Vec<TaskExecutionRecord> = if let Some(arr) = logs_result.as_array() {
        arr.iter()
            .filter_map(|item| {
                let json_value = serde_json::to_value(item).ok()?;
                serde_json::from_value(json_value).ok()
            })
            .collect()
    } else {
        Vec::new()
    };

    let total = logs.len() as u64;
    let paginated_logs = logs.into_iter().collect::<Vec<_>>();

    let log_responses: Vec<TaskLogResponse> = paginated_logs
        .into_iter()
        .map(|log| TaskLogResponse {
            id: log.id.unwrap_or_default(),
            task_id: log.task_id.unwrap_or_default(),
            execution_id: log.execution_id.unwrap_or_default(),
            status: log.status.unwrap_or_default(),
            result: log
                .result
                .map(|v| serde_json::to_string(&v).unwrap_or_default()),
            error_message: log.error_message,
            started_at: log.started_at.map(|dt| dt.to_string()).unwrap_or_default(),
            completed_at: log.completed_at.map(|dt| dt.to_string()),
            execution_time_ms: log.execution_time_ms.unwrap_or(0),
            created_at: log.created_at.map(|dt| dt.to_string()).unwrap_or_default(),
        })
        .collect();

    Ok(PageWrapper {
        records: log_responses,
        total,
        page,
        page_size,
    })
}

pub async fn get_all_logs(
    _state: &ServiceContext,
    params: TaskLogsQuery,
) -> Result<PageWrapper<TaskLogResponse>> {
    let page = resolve_page_u64(params.page)?;
    let page_size = resolve_page_size_u64(params.page_size)?;
    let offset = page.saturating_sub(1).saturating_mul(page_size);

    let logs_result = pool!()
        .query(
            "SELECT * FROM task_execution_record ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            vec![rbs::value!(page_size), rbs::value!(offset)],
        )
        .await
        .map_err(|e| Error::E(format!("Failed to fetch all task logs: {}", e)))?;

    let logs: Vec<TaskExecutionRecord> = if let Some(arr) = logs_result.as_array() {
        arr.iter()
            .filter_map(|item| {
                let json_value = serde_json::to_value(item).ok()?;
                serde_json::from_value(json_value).ok()
            })
            .collect()
    } else {
        Vec::new()
    };

    let total = logs.len() as u64;
    let paginated_logs = logs.into_iter().collect::<Vec<_>>();

    let log_responses: Vec<TaskLogResponse> = paginated_logs
        .into_iter()
        .map(|log| TaskLogResponse {
            id: log.id.unwrap_or_default(),
            task_id: log.task_id.unwrap_or_default(),
            execution_id: log.execution_id.unwrap_or_default(),
            status: log.status.unwrap_or_default(),
            result: log
                .result
                .map(|v| serde_json::to_string(&v).unwrap_or_default()),
            error_message: log.error_message,
            started_at: log.started_at.map(|dt| dt.to_string()).unwrap_or_default(),
            completed_at: log.completed_at.map(|dt| dt.to_string()),
            execution_time_ms: log.execution_time_ms.unwrap_or(0),
            created_at: log.created_at.map(|dt| dt.to_string()).unwrap_or_default(),
        })
        .collect();

    Ok(PageWrapper {
        records: log_responses,
        total,
        page,
        page_size,
    })
}

pub async fn get_task_statistics(
    _state: &ServiceContext,
    task_id: Option<String>,
) -> Result<TaskStatisticsResponse> {
    if let Some(tid) = task_id {
        let _task_opt = ScheduledTask::select_by_map(pool!(), rbs::value! { "id": tid.clone() })
            .await
            .map_err(|e| Error::E(format!("Failed to fetch task: {}", e)))?
            .first()
            .cloned()
            .ok_or_else(|| Error::E(format!("Task not found: {}", tid)))?;

        let logs_result = pool!()
            .query(
                "SELECT * FROM task_execution_record WHERE task_id = $1 ORDER BY created_at DESC",
                vec![rbs::value!(&tid)],
            )
            .await
            .map_err(|e| Error::E(format!("Failed to fetch task logs: {}", e)))?;

        let logs: Vec<TaskExecutionRecord> = if let Some(arr) = logs_result.as_array() {
            arr.iter()
                .filter_map(|item| {
                    let json_value = serde_json::to_value(item).ok()?;
                    serde_json::from_value(json_value).ok()
                })
                .collect()
        } else {
            Vec::new()
        };

        let total_runs = logs.len() as i32;
        let successful_runs = logs
            .iter()
            .filter(|log| log.status.as_ref().map(|s| s == "success").unwrap_or(false))
            .count() as i32;
        let failed_runs = logs
            .iter()
            .filter(|log| log.status.as_ref().map(|s| s == "failed").unwrap_or(false))
            .count() as i32;

        let success_rate = if total_runs > 0 {
            successful_runs as f64 / total_runs as f64 * 100.0
        } else {
            0.0
        };

        let average_execution_time_ms = if total_runs > 0 {
            let total_time: i64 = logs
                .iter()
                .map(|log| log.execution_time_ms.unwrap_or(0))
                .sum();
            total_time as f64 / total_runs as f64
        } else {
            0.0
        };

        let last_execution_time = logs
            .first()
            .and_then(|log| log.completed_at.clone())
            .map(|dt| dt.to_string());

        Ok(TaskStatisticsResponse {
            task_id: tid,
            total_runs,
            successful_runs,
            failed_runs,
            success_rate,
            average_execution_time_ms,
            last_execution_time,
        })
    } else {
        let logs_result = pool!()
            .query(
                "SELECT * FROM task_execution_record ORDER BY created_at DESC",
                vec![],
            )
            .await
            .map_err(|e| Error::E(format!("Failed to fetch all task logs: {}", e)))?;

        let logs: Vec<TaskExecutionRecord> = if let Some(arr) = logs_result.as_array() {
            arr.iter()
                .filter_map(|item| {
                    let json_value = serde_json::to_value(item).ok()?;
                    serde_json::from_value(json_value).ok()
                })
                .collect()
        } else {
            Vec::new()
        };

        let total_runs = logs.len() as i32;
        let successful_runs = logs
            .iter()
            .filter(|log| log.status.as_ref().map(|s| s == "success").unwrap_or(false))
            .count() as i32;
        let failed_runs = logs
            .iter()
            .filter(|log| log.status.as_ref().map(|s| s == "failed").unwrap_or(false))
            .count() as i32;

        let success_rate = if total_runs > 0 {
            successful_runs as f64 / total_runs as f64 * 100.0
        } else {
            0.0
        };

        let average_execution_time_ms = if total_runs > 0 {
            let total_time: i64 = logs
                .iter()
                .map(|log| log.execution_time_ms.unwrap_or(0))
                .sum();
            total_time as f64 / total_runs as f64
        } else {
            0.0
        };

        let last_execution_time = logs
            .first()
            .and_then(|log| log.completed_at.clone())
            .map(|dt| dt.to_string());

        Ok(TaskStatisticsResponse {
            task_id: "all".to_string(),
            total_runs,
            successful_runs,
            failed_runs,
            success_rate,
            average_execution_time_ms,
            last_execution_time,
        })
    }
}
