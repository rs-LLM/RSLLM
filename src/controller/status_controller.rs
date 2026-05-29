use axum::{Json, extract::State};
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, Instant};

use crate::context::ServiceContext;
use crate::domain::table::ai_hub::provider_config::ProviderConfig;
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};

static STARTED_AT: LazyLock<Instant> = LazyLock::new(Instant::now);
const HISTORY_WINDOW_SECONDS: i64 = 24 * 60 * 60;
const STATUS_OVERVIEW_CACHE_TTL_SECONDS: i64 = 20;

#[derive(Debug, Clone)]
struct StatusOverviewCache {
    cached_at_ts: i64,
    response: StatusOverviewResponse,
}

static STATUS_OVERVIEW_CACHE: LazyLock<RwLock<Option<StatusOverviewCache>>> =
    LazyLock::new(|| RwLock::new(None));
static STATUS_OVERVIEW_REFRESHING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusLevel {
    Operational,
    Degraded,
    MajorOutage,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusItem {
    pub id: String,
    pub name: String,
    pub status: StatusLevel,
    pub uptime_percent: f64,
    pub latency_ms: Option<u64>,
    pub last_checked_at: String,
    pub message: String,
    pub provider_group: Option<String>,
    pub history_24h: Vec<StatusHistoryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusHistoryPoint {
    pub timestamp: String,
    pub availability_percent: f64,
    pub total_samples: u64,
    pub success_samples: u64,
    pub error_samples: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusProviderGroupSummary {
    pub group: String,
    pub total: usize,
    pub operational: usize,
    pub degraded: usize,
    pub major_outage: usize,
    pub provider_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusSummary {
    pub total: usize,
    pub operational: usize,
    pub degraded: usize,
    pub major_outage: usize,
    pub down: usize,
    pub checktime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusMonitorGroup {
    pub name: String,
    pub index: usize,
    pub total: usize,
    pub operational: usize,
    pub degraded: usize,
    pub major_outage: usize,
    pub down: usize,
    pub monitors: Vec<StatusItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StatusOverviewResponse {
    pub updated_at: String,
    pub no_token_probe: bool,
    pub sum: StatusSummary,
    pub groups: Vec<StatusMonitorGroup>,
    pub monitors: Vec<StatusItem>,
    pub system_functions: Vec<StatusItem>,
    pub model_latency: Vec<StatusItem>,
    pub history_24h: Vec<StatusHistoryPoint>,
    pub provider_groups: Vec<StatusProviderGroupSummary>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ProbeStatusRequest {
    pub provider_codes: Option<Vec<String>>,
    pub provider_groups: Option<Vec<String>>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderUsageAggRow {
    model_code: String,
    model_name: String,
    provider_group: String,
    total_samples: i64,
    success_samples: Option<i64>,
    avg_latency_ms: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct HistoryHourRow {
    hour_ts: String,
    total_samples: i64,
    success_samples: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelHistoryHourRow {
    model_code: String,
    hour_ts: String,
    total_samples: i64,
    success_samples: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct SystemHistoryHourRow {
    service_id: String,
    hour_ts: String,
    total_samples: i64,
    success_samples: i64,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn build_item(
    id: &str,
    name: &str,
    status: StatusLevel,
    uptime_percent: f64,
    latency_ms: Option<u64>,
    message: String,
    provider_group: Option<String>,
) -> StatusItem {
    StatusItem {
        id: id.to_string(),
        name: name.to_string(),
        status,
        uptime_percent,
        latency_ms,
        last_checked_at: now_rfc3339(),
        message,
        provider_group,
        history_24h: Vec::new(),
    }
}

fn operational_item(
    id: &str,
    name: &str,
    message: String,
    latency_ms: Option<u64>,
    provider_group: Option<String>,
) -> StatusItem {
    build_item(
        id,
        name,
        StatusLevel::Operational,
        100.0,
        latency_ms,
        message,
        provider_group,
    )
}

fn degraded_item(
    id: &str,
    name: &str,
    uptime_percent: f64,
    message: String,
    latency_ms: Option<u64>,
    provider_group: Option<String>,
) -> StatusItem {
    build_item(
        id,
        name,
        StatusLevel::Degraded,
        uptime_percent,
        latency_ms,
        message,
        provider_group,
    )
}

fn major_outage_item(
    id: &str,
    name: &str,
    message: String,
    latency_ms: Option<u64>,
    provider_group: Option<String>,
) -> StatusItem {
    build_item(
        id,
        name,
        StatusLevel::MajorOutage,
        0.0,
        latency_ms,
        message,
        provider_group,
    )
}

fn get_cached_overview() -> Option<StatusOverviewResponse> {
    let guard = STATUS_OVERVIEW_CACHE.read().ok()?;
    let cache = guard.as_ref()?;
    if now_ts() - cache.cached_at_ts <= STATUS_OVERVIEW_CACHE_TTL_SECONDS {
        Some(cache.response.clone())
    } else {
        None
    }
}

fn get_cached_overview_any() -> Option<(StatusOverviewResponse, bool)> {
    let guard = STATUS_OVERVIEW_CACHE.read().ok()?;
    let cache = guard.as_ref()?;
    let is_stale = now_ts() - cache.cached_at_ts > STATUS_OVERVIEW_CACHE_TTL_SECONDS;
    Some((cache.response.clone(), is_stale))
}

fn set_cached_overview(response: &StatusOverviewResponse) {
    if let Ok(mut guard) = STATUS_OVERVIEW_CACHE.write() {
        *guard = Some(StatusOverviewCache {
            cached_at_ts: now_ts(),
            response: response.clone(),
        });
    }
}

fn trigger_refresh_overview_cache(ctx: Arc<ServiceContext>) {
    if STATUS_OVERVIEW_REFRESHING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    tokio::spawn(async move {
        if let Ok(data) = probe_overview(&ctx, None, None, None).await {
            set_cached_overview(&data);
        }
        STATUS_OVERVIEW_REFRESHING.store(false, Ordering::SeqCst);
    });
}

fn build_provider_filters_sql(
    provider_alias: &str,
    provider_codes_filter: &Option<Vec<String>>,
    provider_groups_filter: &Option<Vec<String>>,
    params: &mut Vec<rbs::Value>,
) -> String {
    let mut sql = String::new();

    if let Some(codes) = provider_codes_filter {
        let normalized = codes
            .iter()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if !normalized.is_empty() {
            let placeholders = std::iter::repeat_n("?", normalized.len())
                .collect::<Vec<_>>()
                .join(",");
            sql.push_str(&format!(
                " AND LOWER(TRIM({}.provider_code)) IN ({})",
                provider_alias, placeholders
            ));
            for code in normalized {
                params.push(rbs::value!(code));
            }
        }
    }

    if let Some(groups) = provider_groups_filter {
        let normalized = groups
            .iter()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if !normalized.is_empty() {
            let placeholders = std::iter::repeat_n("?", normalized.len())
                .collect::<Vec<_>>()
                .join(",");
            sql.push_str(&format!(
                " AND LOWER(TRIM({}.provider_type)) IN ({})",
                provider_alias, placeholders
            ));
            for group in normalized {
                params.push(rbs::value!(group));
            }
        }
    }

    sql
}

async fn load_provider_usage_rows(
    ctx: &Arc<ServiceContext>,
    provider_codes_filter: &Option<Vec<String>>,
    provider_groups_filter: &Option<Vec<String>>,
) -> Result<Vec<ProviderUsageAggRow>> {
    let from_ts = now_ts() - HISTORY_WINDOW_SECONDS;
    let from_dt = chrono::DateTime::<chrono::Utc>::from_timestamp(from_ts, 0)
        .map(|v| v.with_timezone(&chrono::Local).to_rfc3339())
        .unwrap_or_else(now_rfc3339);

    let mut params = vec![rbs::value!(from_dt)];
    let filters_sql = build_provider_filters_sql(
        "pcf",
        provider_codes_filter,
        provider_groups_filter,
        &mut params,
    );

    let sql = format!(
        r#"SELECT
            mb.model_code AS model_code,
            mb.name AS model_name,
            COALESCE(
                (
                    SELECT LOWER(TRIM(pc2.provider_type))
                    FROM model_provider_mapping mpm2
                    JOIN provider_config pc2 ON pc2.id = mpm2.provider_id
                    WHERE mpm2.model_id = mb.id AND pc2.status = 'active'
                    ORDER BY mpm2.priority DESC, mpm2.updated_at DESC
                    LIMIT 1
                ),
                'unknown'
            ) AS provider_group,
            COUNT(ul.id) AS total_samples,
            SUM(CASE WHEN (ul.status = 'success' OR ul.status_code = 200 OR ul.status_code = 0) THEN 1 ELSE 0 END) AS success_samples,
            AVG(ul.response_time_ms) AS avg_latency_ms
        FROM model_base mb
        LEFT JOIN ai_hub_usage_log ul ON ul.created_at >= ?
            AND (
                LOWER(TRIM(ul.model_id)) = LOWER(TRIM(COALESCE(mb.id, '')))
                OR LOWER(TRIM(ul.model_id)) = LOWER(TRIM(mb.model_code))
                OR LOWER(TRIM(substr(ul.model_id, CASE WHEN instr(ul.model_id, '/') > 0 THEN instr(ul.model_id, '/') + 1 ELSE 1 END))) = LOWER(TRIM(COALESCE(mb.id, '')))
                OR LOWER(TRIM(substr(ul.model_id, CASE WHEN instr(ul.model_id, '/') > 0 THEN instr(ul.model_id, '/') + 1 ELSE 1 END))) = LOWER(TRIM(mb.model_code))
            )
        WHERE mb.status = 'active'
          AND EXISTS (
                SELECT 1
                FROM model_provider_mapping mpmf
                JOIN provider_config pcf ON pcf.id = mpmf.provider_id
                WHERE mpmf.model_id = mb.id AND pcf.status = 'active'{}
          )
        GROUP BY mb.id, mb.model_code, mb.name
        ORDER BY mb.model_code ASC"#,
        filters_sql
    );

    let raw = ctx
        .rb
        .query(&sql, params)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;
    Ok(rbs::from_value(raw).unwrap_or_default())
}

async fn load_history_24h_from_usage(
    ctx: &Arc<ServiceContext>,
    provider_codes_filter: &Option<Vec<String>>,
    provider_groups_filter: &Option<Vec<String>>,
) -> Result<Vec<StatusHistoryPoint>> {
    let now_local = chrono::Local::now();
    let hour_now = now_local
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .unwrap_or(now_local);
    let from_dt = (hour_now - chrono::Duration::hours(23)).to_rfc3339();
    let _from_ts = now_ts() - HISTORY_WINDOW_SECONDS;
    let from_dt = chrono::DateTime::<chrono::Utc>::from_timestamp(_from_ts, 0)
        .map(|_| from_dt)
        .unwrap_or_else(now_rfc3339);

    let mut params = vec![rbs::value!(from_dt)];
    let filters_sql = build_provider_filters_sql(
        "pc",
        provider_codes_filter,
        provider_groups_filter,
        &mut params,
    );

    let sql = format!(
        r#"SELECT
            substr(ul.created_at, 1, 13) AS hour_ts,
            COUNT(ul.id) AS total_samples,
            SUM(CASE WHEN (ul.status = 'success' OR ul.status_code = 200 OR ul.status_code = 0) THEN 1 ELSE 0 END) AS success_samples
        FROM ai_hub_usage_log ul
        JOIN model_base mb ON mb.status = 'active'
            AND (
                LOWER(TRIM(ul.model_id)) = LOWER(TRIM(COALESCE(mb.id, '')))
                OR LOWER(TRIM(ul.model_id)) = LOWER(TRIM(mb.model_code))
                OR LOWER(TRIM(substr(ul.model_id, CASE WHEN instr(ul.model_id, '/') > 0 THEN instr(ul.model_id, '/') + 1 ELSE 1 END))) = LOWER(TRIM(COALESCE(mb.id, '')))
                OR LOWER(TRIM(substr(ul.model_id, CASE WHEN instr(ul.model_id, '/') > 0 THEN instr(ul.model_id, '/') + 1 ELSE 1 END))) = LOWER(TRIM(mb.model_code))
            )
        WHERE ul.created_at >= ?
          AND EXISTS (
                SELECT 1
                FROM model_provider_mapping mpm
                JOIN provider_config pc ON pc.id = mpm.provider_id
                WHERE mpm.model_id = mb.id AND pc.status = 'active'{}
          )
        GROUP BY hour_ts
        ORDER BY hour_ts ASC"#,
        filters_sql
    );

    let raw = ctx
        .rb
        .query(&sql, params)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let rows: Vec<HistoryHourRow> = rbs::from_value(raw).unwrap_or_default();
    let mut hour_map: HashMap<String, (u64, u64)> = HashMap::new();
    for row in rows {
        let total = row.total_samples.max(0) as u64;
        let success = row.success_samples.unwrap_or(0).max(0) as u64;
        if total == 0 {
            continue;
        }
        hour_map.insert(row.hour_ts, (total, success));
    }

    let mut series = Vec::with_capacity(24);
    for offset in (0..24).rev() {
        let slot = hour_now - chrono::Duration::hours(offset);
        let key = slot.format("%Y-%m-%dT%H").to_string();
        let (total, success) = hour_map.get(&key).copied().unwrap_or((0, 0));
        let availability = if total > 0 {
            (success as f64 / total as f64) * 100.0
        } else {
            100.0
        };
        series.push(StatusHistoryPoint {
            timestamp: slot.to_rfc3339(),
            availability_percent: availability,
            total_samples: total,
            success_samples: success,
            error_samples: total.saturating_sub(success),
        });
    }

    Ok(series)
}

async fn load_model_history_24h_map(
    ctx: &Arc<ServiceContext>,
    provider_codes_filter: &Option<Vec<String>>,
    provider_groups_filter: &Option<Vec<String>>,
) -> Result<HashMap<String, Vec<StatusHistoryPoint>>> {
    let now_local = chrono::Local::now();
    let hour_now = now_local
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .unwrap_or(now_local);
    let from_dt = (hour_now - chrono::Duration::hours(23)).to_rfc3339();

    let mut params = vec![rbs::value!(from_dt)];
    let filters_sql = build_provider_filters_sql(
        "pc",
        provider_codes_filter,
        provider_groups_filter,
        &mut params,
    );

    let sql = format!(
        r#"SELECT
            LOWER(TRIM(mb.model_code)) AS model_code,
            substr(ul.created_at, 1, 13) AS hour_ts,
            COUNT(ul.id) AS total_samples,
            SUM(CASE WHEN (ul.status = 'success' OR ul.status_code = 200 OR ul.status_code = 0) THEN 1 ELSE 0 END) AS success_samples
        FROM model_base mb
        JOIN ai_hub_usage_log ul ON ul.created_at >= ?
            AND (
                LOWER(TRIM(ul.model_id)) = LOWER(TRIM(COALESCE(mb.id, '')))
                OR LOWER(TRIM(ul.model_id)) = LOWER(TRIM(mb.model_code))
                OR LOWER(TRIM(substr(ul.model_id, CASE WHEN instr(ul.model_id, '/') > 0 THEN instr(ul.model_id, '/') + 1 ELSE 1 END))) = LOWER(TRIM(COALESCE(mb.id, '')))
                OR LOWER(TRIM(substr(ul.model_id, CASE WHEN instr(ul.model_id, '/') > 0 THEN instr(ul.model_id, '/') + 1 ELSE 1 END))) = LOWER(TRIM(mb.model_code))
            )
        WHERE mb.status = 'active'
          AND EXISTS (
                SELECT 1
                FROM model_provider_mapping mpm
                JOIN provider_config pc ON pc.id = mpm.provider_id
                WHERE mpm.model_id = mb.id AND pc.status = 'active'{}
          )
        GROUP BY LOWER(TRIM(mb.model_code)), hour_ts
        ORDER BY LOWER(TRIM(mb.model_code)) ASC, hour_ts ASC"#,
        filters_sql
    );

    let raw = ctx
        .rb
        .query(&sql, params)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;
    let rows: Vec<ModelHistoryHourRow> = rbs::from_value(raw).unwrap_or_default();

    let mut per_model_hour_map: HashMap<String, HashMap<String, (u64, u64)>> = HashMap::new();
    for row in rows {
        let total = row.total_samples.max(0) as u64;
        if total == 0 {
            continue;
        }
        let success = row.success_samples.unwrap_or(0).max(0) as u64;
        per_model_hour_map
            .entry(row.model_code)
            .or_default()
            .insert(row.hour_ts, (total, success));
    }

    let mut out = HashMap::new();
    for (model_code, hour_map) in per_model_hour_map {
        out.insert(model_code, build_hour_slots(hour_now, &hour_map, 100.0));
    }
    Ok(out)
}

fn build_provider_group_summary(items: &[StatusItem]) -> Vec<StatusProviderGroupSummary> {
    let mut map: HashMap<String, StatusProviderGroupSummary> = HashMap::new();

    for item in items {
        let group = item
            .provider_group
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let entry = map
            .entry(group.clone())
            .or_insert_with(|| StatusProviderGroupSummary {
                group,
                total: 0,
                operational: 0,
                degraded: 0,
                major_outage: 0,
                provider_codes: Vec::new(),
            });

        entry.total += 1;
        match item.status {
            StatusLevel::Operational => entry.operational += 1,
            StatusLevel::Degraded => entry.degraded += 1,
            StatusLevel::MajorOutage => entry.major_outage += 1,
        }
        entry.provider_codes.push(item.id.clone());
    }

    let mut result = map.into_values().collect::<Vec<_>>();
    result.sort_by(|a, b| a.group.cmp(&b.group));
    result
}

fn usage_row_to_status_item(row: ProviderUsageAggRow) -> StatusItem {
    let total = row.total_samples.max(0) as u64;
    let success = row.success_samples.unwrap_or(0).max(0) as u64;
    let success_rate = if total > 0 {
        (success as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let latency = row.avg_latency_ms.map(|v| v.max(0.0).round() as u64);

    if total == 0 {
        return operational_item(
            &row.model_code,
            &row.model_name,
            "no usage samples in last 24h".to_string(),
            Some(latency.unwrap_or(0)),
            Some(row.provider_group),
        );
    }

    if success == 0 {
        return major_outage_item(
            &row.model_code,
            &row.model_name,
            format!("0/{} success in last 24h", total),
            latency,
            Some(row.provider_group),
        );
    }

    if success_rate < 95.0 {
        return degraded_item(
            &row.model_code,
            &row.model_name,
            success_rate,
            format!("{}/{} success in last 24h", success, total),
            latency,
            Some(row.provider_group),
        );
    }

    build_item(
        &row.model_code,
        &row.model_name,
        StatusLevel::Operational,
        success_rate,
        latency,
        format!("{}/{} success in last 24h", success, total),
        Some(row.provider_group),
    )
}

fn count_levels(items: &[StatusItem]) -> (usize, usize, usize) {
    let mut operational = 0usize;
    let mut degraded = 0usize;
    let mut major_outage = 0usize;
    for item in items {
        match item.status {
            StatusLevel::Operational => operational += 1,
            StatusLevel::Degraded => degraded += 1,
            StatusLevel::MajorOutage => major_outage += 1,
        }
    }
    (operational, degraded, major_outage)
}

fn build_status_groups(
    system_functions: &[StatusItem],
    model_latency: &[StatusItem],
    provider_groups: &[StatusProviderGroupSummary],
) -> Vec<StatusMonitorGroup> {
    let mut groups = Vec::new();

    let (sys_ok, sys_deg, sys_out) = count_levels(system_functions);
    groups.push(StatusMonitorGroup {
        name: "system".to_string(),
        index: 0,
        total: system_functions.len(),
        operational: sys_ok,
        degraded: sys_deg,
        major_outage: sys_out,
        down: sys_deg + sys_out,
        monitors: system_functions.to_vec(),
    });

    for (idx, pg) in provider_groups.iter().enumerate() {
        let monitors = model_latency
            .iter()
            .filter(|item| {
                item.provider_group
                    .as_ref()
                    .map(|v| v.eq_ignore_ascii_case(&pg.group))
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();
        groups.push(StatusMonitorGroup {
            name: pg.group.clone(),
            index: idx + 1,
            total: pg.total,
            operational: pg.operational,
            degraded: pg.degraded,
            major_outage: pg.major_outage,
            down: pg.degraded + pg.major_outage,
            monitors,
        });
    }

    groups
}

fn build_status_summary(monitors: &[StatusItem]) -> StatusSummary {
    let (operational, degraded, major_outage) = count_levels(monitors);
    StatusSummary {
        total: monitors.len(),
        operational,
        degraded,
        major_outage,
        down: degraded + major_outage,
        checktime: now_rfc3339(),
    }
}

fn build_hour_slots(
    hour_now: chrono::DateTime<chrono::Local>,
    hour_map: &HashMap<String, (u64, u64)>,
    no_sample_availability: f64,
) -> Vec<StatusHistoryPoint> {
    let mut series = Vec::with_capacity(24);
    for offset in (0..24).rev() {
        let slot = hour_now - chrono::Duration::hours(offset);
        let key = slot.format("%Y-%m-%dT%H").to_string();
        let (total, success) = hour_map.get(&key).copied().unwrap_or((0, 0));
        let error = total.saturating_sub(success);
        let availability = if total > 0 {
            (success as f64 / total as f64) * 100.0
        } else {
            no_sample_availability
        };
        series.push(StatusHistoryPoint {
            timestamp: slot.to_rfc3339(),
            availability_percent: availability,
            total_samples: total,
            success_samples: success,
            error_samples: error,
        });
    }
    series
}

fn default_monitor_history_24h() -> Vec<StatusHistoryPoint> {
    let now_local = chrono::Local::now();
    let hour_now = now_local
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .unwrap_or(now_local);
    build_hour_slots(hour_now, &HashMap::new(), 100.0)
}

fn current_local_hour_key() -> String {
    let now_local = chrono::Local::now();
    let hour_now = now_local
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .unwrap_or(now_local);
    hour_now.format("%Y-%m-%dT%H").to_string()
}

async fn record_system_hourly_samples(
    ctx: &Arc<ServiceContext>,
    system_functions: &[StatusItem],
) -> Result<()> {
    let hour_key = current_local_hour_key();
    let now = now_rfc3339();

    for item in system_functions {
        let success = if matches!(item.status, StatusLevel::Operational) {
            1i64
        } else {
            0i64
        };
        let sql = r#"INSERT INTO system_status_hourly
            (service_id, hour_ts, total_samples, success_samples, created_at, updated_at)
            VALUES (?, ?, 1, ?, ?, ?)
            ON CONFLICT(service_id, hour_ts)
            DO UPDATE SET
              total_samples = total_samples + 1,
              success_samples = success_samples + excluded.success_samples,
              updated_at = excluded.updated_at"#;
        ctx.rb
            .exec(
                sql,
                vec![
                    rbs::value!(item.id.clone()),
                    rbs::value!(hour_key.clone()),
                    rbs::value!(success),
                    rbs::value!(now.clone()),
                    rbs::value!(now.clone()),
                ],
            )
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
    }

    let cutoff_key = (chrono::Local::now() - chrono::Duration::hours(36))
        .format("%Y-%m-%dT%H")
        .to_string();
    ctx.rb
        .exec(
            "DELETE FROM system_status_hourly WHERE hour_ts < ?",
            vec![rbs::value!(cutoff_key)],
        )
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(())
}

async fn load_system_history_24h_map(
    ctx: &Arc<ServiceContext>,
    system_functions: &[StatusItem],
) -> Result<HashMap<String, Vec<StatusHistoryPoint>>> {
    let now_local = chrono::Local::now();
    let hour_now = now_local
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .unwrap_or(now_local);

    let from_key = (hour_now - chrono::Duration::hours(23))
        .format("%Y-%m-%dT%H")
        .to_string();

    let ids = system_functions
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = std::iter::repeat_n("?", ids.len())
        .collect::<Vec<_>>()
        .join(",");

    let mut params = vec![rbs::value!(from_key)];
    for id in &ids {
        params.push(rbs::value!(id.clone()));
    }

    let sql = format!(
        "SELECT service_id, hour_ts, total_samples, success_samples
         FROM system_status_hourly
         WHERE hour_ts >= ? AND service_id IN ({})
         ORDER BY service_id ASC, hour_ts ASC",
        placeholders
    );

    let raw = ctx
        .rb
        .query(&sql, params)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;
    let rows: Vec<SystemHistoryHourRow> = rbs::from_value(raw).unwrap_or_default();

    let mut per_service_hour_map: HashMap<String, HashMap<String, (u64, u64)>> = HashMap::new();
    for row in rows {
        let total = row.total_samples.max(0) as u64;
        let success = row.success_samples.max(0) as u64;
        per_service_hour_map
            .entry(row.service_id)
            .or_default()
            .insert(row.hour_ts, (total, success));
    }

    let mut out = HashMap::new();
    for item in system_functions {
        if let Some(hour_map) = per_service_hour_map.get(&item.id) {
            out.insert(item.id.clone(), build_hour_slots(hour_now, hour_map, 100.0));
        } else {
            out.insert(item.id.clone(), synthetic_system_history_24h(&item.status));
        }
    }

    Ok(out)
}

fn synthetic_system_history_24h(status: &StatusLevel) -> Vec<StatusHistoryPoint> {
    let now_local = chrono::Local::now();
    let hour_now = now_local
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .unwrap_or(now_local);

    let mut hour_map = HashMap::new();
    for offset in (0..24).rev() {
        let slot = hour_now - chrono::Duration::hours(offset);
        let key = slot.format("%Y-%m-%dT%H").to_string();
        match status {
            StatusLevel::Operational => {
                hour_map.insert(key, (1, 1));
            }
            StatusLevel::Degraded | StatusLevel::MajorOutage => {
                hour_map.insert(key, (1, 0));
            }
        }
    }
    build_hour_slots(hour_now, &hour_map, 100.0)
}

pub(crate) async fn probe_overview(
    ctx: &Arc<ServiceContext>,
    provider_codes_filter: Option<Vec<String>>,
    provider_groups_filter: Option<Vec<String>>,
    timeout_ms: Option<u64>,
) -> Result<StatusOverviewResponse> {
    let mut system_functions = Vec::new();

    let uptime_secs = STARTED_AT.elapsed().as_secs();
    system_functions.push(operational_item(
        "api",
        "API Gateway",
        format!("service running for {}s", uptime_secs),
        Some(0),
        None,
    ));

    let db_start = Instant::now();
    let db_timeout_ms = timeout_ms.unwrap_or(1000).max(1).min(10_000);
    match tokio::time::timeout(
        Duration::from_millis(db_timeout_ms),
        ctx.rb.query("SELECT 1", vec![]),
    )
    .await
    {
        Ok(Ok(_)) => system_functions.push(operational_item(
            "database",
            "Database",
            "database ping ok".to_string(),
            Some(db_start.elapsed().as_millis() as u64),
            None,
        )),
        Ok(Err(err)) => system_functions.push(major_outage_item(
            "database",
            "Database",
            format!("database ping failed: {}", err),
            Some(db_start.elapsed().as_millis() as u64),
            None,
        )),
        Err(_elapsed) => system_functions.push(major_outage_item(
            "database",
            "Database",
            format!("database ping timeout: {}ms", db_timeout_ms),
            Some(db_start.elapsed().as_millis() as u64),
            None,
        )),
    }

    let provider_count = ProviderConfig::select_enabled_page(&ctx.rb, 1, 10_000)
        .await
        .map(|v| v.len())
        .unwrap_or(0);
    if provider_count > 0 {
        system_functions.push(operational_item(
            "providers",
            "Provider Registry",
            format!("{} active providers", provider_count),
            Some(0),
            None,
        ));
    } else {
        system_functions.push(degraded_item(
            "providers",
            "Provider Registry",
            0.0,
            "no active provider configured".to_string(),
            Some(0),
            None,
        ));
    }

    let provider_rows =
        load_provider_usage_rows(ctx, &provider_codes_filter, &provider_groups_filter).await?;
    let model_history_24h =
        load_model_history_24h_map(ctx, &provider_codes_filter, &provider_groups_filter).await?;
    let mut model_latency = provider_rows
        .into_iter()
        .map(usage_row_to_status_item)
        .collect::<Vec<_>>();
    for item in &mut model_latency {
        let key = item.id.trim().to_lowercase();
        item.history_24h = model_history_24h
            .get(&key)
            .cloned()
            .unwrap_or_else(default_monitor_history_24h);
    }

    if let Err(_err) = record_system_hourly_samples(ctx, &system_functions).await {}

    let system_history_24h_map = load_system_history_24h_map(ctx, &system_functions)
        .await
        .unwrap_or_default();
    for item in &mut system_functions {
        item.history_24h = system_history_24h_map
            .get(&item.id)
            .cloned()
            .unwrap_or_else(|| synthetic_system_history_24h(&item.status));
    }

    let history_24h =
        load_history_24h_from_usage(ctx, &provider_codes_filter, &provider_groups_filter).await?;
    let provider_groups = build_provider_group_summary(&model_latency);
    let groups = build_status_groups(&system_functions, &model_latency, &provider_groups);
    let monitors = groups
        .iter()
        .flat_map(|g| g.monitors.clone())
        .collect::<Vec<_>>();
    let sum = build_status_summary(&monitors);

    Ok(StatusOverviewResponse {
        updated_at: now_rfc3339(),
        no_token_probe: true,
        sum,
        groups,
        monitors,
        system_functions,
        model_latency,
        history_24h,
        provider_groups,
    })
}

#[utoipa::path(
    get,
    path = "/rsllm/public/status/overview",
    responses(
        (status = 200, description = "状态概览", body = ApiResponse<StatusOverviewResponse>)
    ),
    tag = "status"
)]
pub async fn get_status_overview(
    State(ctx): State<Arc<ServiceContext>>,
) -> Result<Json<ApiResponse<StatusOverviewResponse>>> {
    if let Some(cached) = get_cached_overview() {
        return Ok(Json(ApiResponse::success(cached)));
    }

    if let Some((cached, is_stale)) = get_cached_overview_any() {
        if is_stale {
            trigger_refresh_overview_cache(ctx.clone());
        }
        return Ok(Json(ApiResponse::success(cached)));
    }

    let data = probe_overview(&ctx, None, None, None).await?;
    set_cached_overview(&data);
    Ok(Json(ApiResponse::success(data)))
}

#[utoipa::path(
    post,
    path = "/rsllm/public/status/probe",
    request_body = ProbeStatusRequest,
    responses(
        (status = 200, description = "探活完成", body = ApiResponse<StatusOverviewResponse>)
    ),
    tag = "status"
)]
pub async fn probe_status(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<ProbeStatusRequest>,
) -> Result<Json<ApiResponse<StatusOverviewResponse>>> {
    let provider_codes = req.provider_codes;
    let provider_groups = req.provider_groups;
    let timeout_ms = req.timeout_ms;

    let data = probe_overview(
        &ctx,
        provider_codes.clone(),
        provider_groups.clone(),
        timeout_ms,
    )
    .await?;

    if provider_codes
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
        && provider_groups
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
    {
        set_cached_overview(&data);
    }

    Ok(Json(ApiResponse::success(data)))
}
