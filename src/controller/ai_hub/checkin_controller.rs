use axum::Json;
use axum::extract::{Query, State};
use chrono::{Datelike, Utc};
use rand::Rng;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::CreateTransactionDTO;
use crate::domain::table::basic::KeyValueConfig;
use crate::domain::vo::response::ApiResponse;
use crate::error::{ApplicationError, ApplicationResult};
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::transaction_service::TransactionService;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckinStatusQuery {
    pub month: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckinRecordVO {
    pub checkin_date: String,
    pub quota_awarded: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckinStatsVO {
    pub total_quota: i64,
    pub total_checkins: i64,
    pub checkin_count: usize,
    pub checked_in_today: bool,
    pub records: Vec<CheckinRecordVO>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckinStatusData {
    pub enabled: bool,
    pub min_quota: i64,
    pub max_quota: i64,
    pub stats: CheckinStatsVO,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DoCheckinData {
    pub quota_awarded: i64,
    pub checkin_date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StoredCheckinRecord {
    checkin_date: String,
    quota_awarded: i64,
    created_at: String,
}

fn parse_bool_str(raw: &str) -> bool {
    let normalized = raw.trim().to_ascii_lowercase();
    matches!(normalized.as_str(), "true" | "1" | "yes" | "on")
}

fn normalize_month(raw: Option<&str>) -> String {
    let fallback = Utc::now().format("%Y-%m").to_string();
    let Some(value) = raw else {
        return fallback;
    };

    let trimmed = value.trim();
    if trimmed.len() != 7 {
        return fallback;
    }

    let bytes = trimmed.as_bytes();
    if bytes[4] != b'-' {
        return fallback;
    }

    let year_ok = bytes[0..4].iter().all(u8::is_ascii_digit);
    let month_ok = bytes[5..7].iter().all(u8::is_ascii_digit);
    if !year_ok || !month_ok {
        return fallback;
    }

    let month_num = trimmed[5..7].parse::<u32>().unwrap_or(0);
    if !(1..=12).contains(&month_num) {
        return fallback;
    }

    trimmed.to_string()
}

async fn read_checkin_settings(
    context: &Arc<ServiceContext>,
) -> ApplicationResult<(bool, i64, i64)> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let enabled_raw =
        KeyValueConfig::get_value(&mut conn, "system.checkin.enabled", "false").await?;
    let min_raw = KeyValueConfig::get_value(&mut conn, "system.checkin.min_quota", "1000").await?;
    let max_raw = KeyValueConfig::get_value(&mut conn, "system.checkin.max_quota", "10000").await?;

    let enabled = parse_bool_str(&enabled_raw);
    let min_quota = min_raw.trim().parse::<i64>().unwrap_or(1000);
    let max_quota = max_raw.trim().parse::<i64>().unwrap_or(10000);

    Ok((enabled, min_quota.max(1), max_quota.max(min_quota.max(1))))
}

fn checkin_kv_key(user_id: &str) -> String {
    format!("user.checkin.{}", user_id)
}

fn month_range(month: &str) -> (String, String) {
    let start = format!("{}-01", month);

    let (year, month_num) = match (
        month.get(0..4).and_then(|v| v.parse::<i32>().ok()),
        month.get(5..7).and_then(|v| v.parse::<u32>().ok()),
    ) {
        (Some(y), Some(m)) => (y, m),
        _ => return (start, format!("{}-31", month)),
    };

    let next_month = if month_num == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month_num + 1, 1)
    };
    let end_day = next_month
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(31);

    let end = format!("{}-{:02}", month, end_day);
    (start, end)
}

fn parse_stored_records(raw: &str) -> Vec<StoredCheckinRecord> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<StoredCheckinRecord>>(raw).unwrap_or_default()
}

fn filter_month_records(records: &[StoredCheckinRecord], month: &str) -> Vec<StoredCheckinRecord> {
    let (start, end) = month_range(month);
    records
        .iter()
        .filter(|item| item.checkin_date >= start && item.checkin_date <= end)
        .cloned()
        .collect()
}

#[utoipa::path(
    get,
    path = "/rsllm/api/user/checkin",
    params(
        ("month" = Option<String>, Query, description = "月份，格式 YYYY-MM，默认当前月份")
    ),
    responses(
        (status = 200, description = "获取签到状态成功", body = ApiResponse<CheckinStatusData>)
    ),
    tag = "user"
)]
pub async fn get_checkin_status(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Query(query): Query<CheckinStatusQuery>,
) -> ApplicationResult<Json<ApiResponse<CheckinStatusData>>> {
    let (enabled, min_quota, max_quota) = read_checkin_settings(&context).await?;
    let month = normalize_month(query.month.as_deref());

    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let key = checkin_kv_key(&jwt_auth.id);
    let raw = KeyValueConfig::get_value(&mut conn, &key, "[]").await?;
    let records_all = parse_stored_records(&raw);

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let checked_in_today = records_all.iter().any(|item| item.checkin_date == today);

    let total_checkins = records_all.len() as i64;
    let total_quota = records_all
        .iter()
        .map(|item| item.quota_awarded)
        .sum::<i64>();

    let mut month_records = filter_month_records(&records_all, &month);
    month_records.sort_by(|a, b| b.checkin_date.cmp(&a.checkin_date));

    let records = month_records
        .iter()
        .map(|item| CheckinRecordVO {
            checkin_date: item.checkin_date.clone(),
            quota_awarded: item.quota_awarded,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiResponse::success(CheckinStatusData {
        enabled,
        min_quota,
        max_quota,
        stats: CheckinStatsVO {
            total_quota,
            total_checkins,
            checkin_count: records.len(),
            checked_in_today,
            records,
        },
    })))
}

#[utoipa::path(
    post,
    path = "/rsllm/api/user/checkin",
    responses(
        (status = 200, description = "签到成功", body = ApiResponse<DoCheckinData>)
    ),
    tag = "user"
)]
pub async fn do_checkin(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
) -> ApplicationResult<Json<ApiResponse<DoCheckinData>>> {
    let (enabled, min_quota, max_quota) = read_checkin_settings(&context).await?;
    if !enabled {
        return Err(ApplicationError::BusinessError {
            message: "签到功能未启用".to_string(),
            code: Some("CHECKIN_DISABLED".to_string()),
            context: Some("checkin.do".to_string()),
        });
    }

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut conn =
        context
            .rb
            .acquire_begin()
            .await
            .map_err(|e| ApplicationError::DatabaseError {
                message: format!("数据库连接失败: {}", e),
                operation: Some("acquire_begin".to_string()),
                table: None,
            })?;

    let key = checkin_kv_key(&jwt_auth.id);
    let raw = KeyValueConfig::get_value(&mut conn, &key, "[]").await?;
    let mut records = parse_stored_records(&raw);

    if records.iter().any(|item| item.checkin_date == today) {
        conn.rollback().await?;
        return Err(ApplicationError::BusinessError {
            message: "今日已签到".to_string(),
            code: Some("CHECKIN_ALREADY_DONE".to_string()),
            context: Some("checkin.do".to_string()),
        });
    }

    let quota_awarded = if max_quota > min_quota {
        let mut rng = rand::rng();
        rng.random_range(min_quota..=max_quota)
    } else {
        min_quota
    };

    let transaction_service = TransactionService {};
    let now = DateTime::now().to_string();
    transaction_service
        .create_transaction_with_tx(
            &mut conn,
            CreateTransactionDTO {
                user_id: jwt_auth.id.clone(),
                transaction_type: "recharge".to_string(),
                amount: quota_awarded as f64,
                reason: format!("用户签到奖励: date={}", today),
            },
            None,
        )
        .await?;

    records.push(StoredCheckinRecord {
        checkin_date: today.clone(),
        quota_awarded,
        created_at: now,
    });

    let payload = serde_json::to_string(&records).map_err(|e| ApplicationError::BusinessError {
        message: format!("签到记录序列化失败: {}", e),
        code: Some("CHECKIN_RECORD_SERIALIZE_FAILED".to_string()),
        context: Some("checkin.do".to_string()),
    })?;

    KeyValueConfig::set_value(&mut conn, &key, &payload, Some("用户签到记录")).await?;

    conn.commit().await?;

    Ok(Json(ApiResponse::success(DoCheckinData {
        quota_awarded,
        checkin_date: today,
    })))
}
