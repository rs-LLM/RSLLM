use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use chrono::Utc;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::context::ServiceContext;
use crate::domain::table::key_value_config::KeyValueConfig;
use crate::domain::table::official_announcement::OfficialAnnouncement;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnnouncementTab {
    pub label: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AnnouncementItemVO {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub enabled: bool,
    pub popup: bool,
    #[serde(rename = "sortOrder")]
    pub sort_order: i32,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    pub tabs: Vec<AnnouncementTab>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateAnnouncementRequest {
    pub title: String,
    pub summary: Option<String>,
    pub enabled: Option<bool>,
    pub popup: Option<bool>,
    #[serde(rename = "sortOrder")]
    pub sort_order: Option<i32>,
    pub tabs: Option<Vec<AnnouncementTab>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub enabled: Option<bool>,
    pub popup: Option<bool>,
    #[serde(rename = "sortOrder")]
    pub sort_order: Option<i32>,
    pub tabs: Option<Vec<AnnouncementTab>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LegacyAnnouncementItem {
    pub id: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub enabled: Option<bool>,
    pub popup: Option<bool>,
    #[serde(rename = "sort_order")]
    pub sort_order: Option<i32>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    pub tabs: Option<Vec<AnnouncementTab>>,
}

fn normalize_tabs(tabs: Vec<AnnouncementTab>) -> Vec<AnnouncementTab> {
    tabs.into_iter()
        .map(|t| AnnouncementTab {
            label: t.label.trim().to_string(),
            content: t.content.trim().to_string(),
        })
        .filter(|t| !t.label.is_empty() || !t.content.is_empty())
        .collect()
}

fn to_vo(item: OfficialAnnouncement) -> AnnouncementItemVO {
    let tabs: Vec<AnnouncementTab> = serde_json::from_str(&item.content).unwrap_or_default();
    let updated_at = item.updated_at.map(|dt| dt.to_string()).unwrap_or_default();
    let updated_at = updated_at.split(' ').next().unwrap_or("").to_string();
    AnnouncementItemVO {
        id: item.id,
        title: item.title,
        summary: item.summary,
        enabled: item.enabled != 0,
        popup: item.popup != 0,
        sort_order: item.sort_order,
        updated_at: if updated_at.is_empty() {
            None
        } else {
            Some(updated_at)
        },
        tabs,
    }
}

async fn lazy_migrate_from_kv(context: &ServiceContext) {
    let conn = match context.rb.acquire().await {
        Ok(conn) => conn,
        Err(_) => return,
    };

    #[derive(Deserialize)]
    struct CntRow {
        cnt: i64,
    }

    let cnt_rows: Vec<CntRow> = conn
        .query_decode("SELECT COUNT(1) AS cnt FROM official_announcement", vec![])
        .await
        .unwrap_or_default();
    if cnt_rows.first().map(|r| r.cnt).unwrap_or(0) > 0 {
        return;
    }

    let legacy = KeyValueConfig::get_value(&conn, "system.official_announcements_items", "[]")
        .await
        .unwrap_or_else(|_| "[]".to_string());

    let list: Vec<LegacyAnnouncementItem> = serde_json::from_str(&legacy).unwrap_or_default();
    if list.is_empty() {
        return;
    }

    for (idx, item) in list.into_iter().enumerate() {
        let title = item.title.unwrap_or_default().trim().to_string();
        if title.is_empty() {
            continue;
        }
        let summary = item.summary.unwrap_or_default().trim().to_string();
        let enabled = if item.enabled.unwrap_or(true) { 1 } else { 0 };
        let popup = if item.popup.unwrap_or(false) { 1 } else { 0 };
        let tabs = normalize_tabs(item.tabs.unwrap_or_default());
        let content = serde_json::to_string(&tabs).unwrap_or_else(|_| "[]".to_string());
        let sort_order = item
            .sort_order
            .unwrap_or_else(|| 1_000_000_000i32.saturating_sub(idx as i32));
        let record = OfficialAnnouncement {
            id: item.id.unwrap_or_else(|| Ulid::new().to_string()),
            title,
            summary,
            content,
            enabled,
            popup,
            sort_order,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };
        let _ = OfficialAnnouncement::insert(&conn, &record).await;
    }
}

pub async fn list_announcements(
    State(context): State<Arc<ServiceContext>>,
) -> ApplicationResult<Json<ApiResponse<Vec<AnnouncementItemVO>>>> {
    lazy_migrate_from_kv(&context).await;

    let conn = context.rb.acquire().await?;
    let list: Vec<OfficialAnnouncement> = conn
        .query_decode(
            "SELECT * FROM official_announcement ORDER BY sort_order DESC, updated_at DESC, created_at DESC",
            vec![],
        )
        .await
        .unwrap_or_default();

    Ok(Json(ApiResponse::success(
        list.into_iter().map(to_vo).collect(),
    )))
}

pub async fn create_announcement(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<CreateAnnouncementRequest>,
) -> ApplicationResult<Json<ApiResponse<AnnouncementItemVO>>> {
    lazy_migrate_from_kv(&context).await;

    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Ok(Json(ApiResponse::error("VALIDATION_ERROR", "标题不能为空")));
    }

    let summary = req.summary.unwrap_or_default().trim().to_string();
    let enabled = if req.enabled.unwrap_or(true) { 1 } else { 0 };
    let popup = if req.popup.unwrap_or(false) { 1 } else { 0 };
    let tabs = normalize_tabs(req.tabs.unwrap_or_default());
    let content = serde_json::to_string(&tabs).unwrap_or_else(|_| "[]".to_string());
    let sort_order = req
        .sort_order
        .unwrap_or_else(|| Utc::now().timestamp() as i32);

    let record = OfficialAnnouncement {
        id: Ulid::new().to_string(),
        title,
        summary,
        content,
        enabled,
        popup,
        sort_order,
        created_at: Some(DateTime::now()),
        updated_at: Some(DateTime::now()),
    };

    let conn = context.rb.acquire().await?;
    OfficialAnnouncement::insert(&conn, &record).await?;

    Ok(Json(ApiResponse::success(to_vo(record))))
}

pub async fn update_announcement(
    State(context): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAnnouncementRequest>,
) -> ApplicationResult<Json<ApiResponse<AnnouncementItemVO>>> {
    lazy_migrate_from_kv(&context).await;

    let conn = context.rb.acquire().await?;
    let existing = OfficialAnnouncement::select_by_map(&conn, rbs::value! {"id": &id}).await?;
    let Some(mut existing) = existing.into_iter().next() else {
        return Ok(Json(ApiResponse::error("NOT_FOUND", "公告不存在")));
    };

    if let Some(title) = req.title {
        let t = title.trim().to_string();
        if t.is_empty() {
            return Ok(Json(ApiResponse::error("VALIDATION_ERROR", "标题不能为空")));
        }
        existing.title = t;
    }
    if let Some(summary) = req.summary {
        existing.summary = summary.trim().to_string();
    }
    if let Some(enabled) = req.enabled {
        existing.enabled = if enabled { 1 } else { 0 };
    }
    if let Some(popup) = req.popup {
        existing.popup = if popup { 1 } else { 0 };
    }
    if let Some(sort_order) = req.sort_order {
        existing.sort_order = sort_order;
    }
    if let Some(tabs) = req.tabs {
        let tabs = normalize_tabs(tabs);
        existing.content = serde_json::to_string(&tabs).unwrap_or_else(|_| "[]".to_string());
    }
    existing.updated_at = Some(DateTime::now());

    OfficialAnnouncement::update_by_map(&conn, &existing, rbs::value! {"id": &existing.id}).await?;
    Ok(Json(ApiResponse::success(to_vo(existing))))
}

pub async fn delete_announcement(
    State(context): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> ApplicationResult<Json<ApiResponse<bool>>> {
    lazy_migrate_from_kv(&context).await;

    let conn = context.rb.acquire().await?;
    let result = conn
        .exec(
            "DELETE FROM official_announcement WHERE id = ?",
            vec![rbs::value!(id)],
        )
        .await?;
    Ok(Json(ApiResponse::success(result.rows_affected > 0)))
}
