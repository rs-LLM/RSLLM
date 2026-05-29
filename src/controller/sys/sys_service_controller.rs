use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::context::CONTEXT;
use crate::context::ServiceContext;
use crate::domain::table::ai_hub::model_base::ModelBase;
use crate::domain::table::ai_hub::user_level_config::UserLevelConfig;
use crate::domain::table::key_value_config::KeyValueConfig;
use crate::domain::table::official_announcement::OfficialAnnouncement;
use crate::domain::vo::response::ApiResponse;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ServiceStatusVO {
    pub name: String,
    pub status: bool,
    pub description: String,
    pub response_time: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicBrandingConfig {
    pub site_name: String,
    pub panel_title: String,
    pub logo_url: String,
    pub logo_dark_url: String,
    pub official_about_title: String,
    pub official_about_content: String,
    pub official_about_highlights_title: String,
    pub official_about_highlights_items: String,
    pub official_about_timeline_title: String,
    pub official_about_timeline_items: String,
    pub official_announcements_enabled: String,
    pub official_announcements_popup_enabled: String,
    pub official_announcements_popup_once: String,
    pub official_announcements_title: String,
    pub official_announcements_items: String,
    pub official_home_page_enabled: String,
    pub official_home_page_title: String,
    pub official_home_page_content: String,
    pub register_email_verify_enabled: String,
    pub register_captcha_enabled: String,
    pub allow_register: String,
    pub invite_code_required: String,
    pub register_review_enabled: String,
    pub invite_code_bypass_review: String,
    pub openai_api_scheme: String,
    pub openai_api_host: String,
    pub openai_api_port: String,
    pub openai_api_path: String,
    pub subscription_payment_allow_combined: String,
    pub legal_user_agreement: String,
    pub legal_privacy_policy: String,
    pub fetch_enable_ssrf_protection: String,
    pub fetch_allow_private_ip: String,
    pub fetch_domain_filter_mode: String,
    pub fetch_ip_filter_mode: String,
    pub fetch_domain_list: String,
    pub fetch_ip_list: String,
    pub fetch_allowed_ports: String,
    pub fetch_apply_ip_filter_for_domain: String,
    pub checkin_enabled: String,
    pub checkin_min_quota: String,
    pub checkin_max_quota: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SystemConfigResponse {
    pub site_name: String,
    pub panel_title: String,
    pub logo_url: String,
    pub logo_dark_url: String,
    pub official_about_title: String,
    pub official_about_content: String,
    pub official_about_highlights_title: String,
    pub official_about_highlights_items: String,
    pub official_about_timeline_title: String,
    pub official_about_timeline_items: String,
    pub official_announcements_enabled: String,
    pub official_announcements_popup_enabled: String,
    pub official_announcements_popup_once: String,
    pub official_announcements_title: String,
    pub official_announcements_items: String,
    pub official_home_page_enabled: String,
    pub official_home_page_title: String,
    pub official_home_page_content: String,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_from_name: String,
    pub smtp_starttls: String,
    pub register_email_verify_enabled: String,
    pub register_captcha_enabled: String,
    pub allow_register: String,
    pub invite_code_required: String,
    pub register_review_enabled: String,
    pub invite_code_bypass_review: String,
    pub openai_api_scheme: String,
    pub openai_api_host: String,
    pub openai_api_port: String,
    pub openai_api_path: String,
    pub subscription_payment_allow_combined: String,
    pub login_fail_retry: String,
    pub login_fail_retry_wait_sec: String,
    pub trash_recycle_days: String,
    pub datetime_format: String,
    pub sms_cache_send_key_prefix: String,
    pub register_review_reject_notify_enabled: String,
    pub legal_user_agreement: String,
    pub legal_privacy_policy: String,
    pub fetch_enable_ssrf_protection: String,
    pub fetch_allow_private_ip: String,
    pub fetch_domain_filter_mode: String,
    pub fetch_ip_filter_mode: String,
    pub fetch_domain_list: String,
    pub fetch_ip_list: String,
    pub fetch_allowed_ports: String,
    pub fetch_apply_ip_filter_for_domain: String,
    pub oidc_enabled: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub oidc_well_known: String,
    pub oidc_authorization_endpoint: String,
    pub oidc_token_endpoint: String,
    pub oidc_user_info_endpoint: String,
    pub checkin_enabled: String,
    pub checkin_min_quota: String,
    pub checkin_max_quota: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SystemConfigUpdateRequest {
    pub site_name: Option<String>,
    pub panel_title: Option<String>,
    pub logo_url: Option<String>,
    pub logo_dark_url: Option<String>,
    pub official_about_title: Option<String>,
    pub official_about_content: Option<String>,
    pub official_about_highlights_title: Option<String>,
    pub official_about_highlights_items: Option<String>,
    pub official_about_timeline_title: Option<String>,
    pub official_about_timeline_items: Option<String>,
    pub official_announcements_enabled: Option<String>,
    pub official_announcements_popup_enabled: Option<String>,
    pub official_announcements_popup_once: Option<String>,
    pub official_announcements_title: Option<String>,
    pub official_announcements_items: Option<String>,
    pub official_home_page_enabled: Option<String>,
    pub official_home_page_title: Option<String>,
    pub official_home_page_content: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
    pub smtp_from_name: Option<String>,
    pub smtp_starttls: Option<String>,
    pub register_email_verify_enabled: Option<String>,
    pub register_captcha_enabled: Option<String>,
    pub allow_register: Option<String>,
    pub invite_code_required: Option<String>,
    pub register_review_enabled: Option<String>,
    pub invite_code_bypass_review: Option<String>,
    pub login_fail_retry: Option<String>,
    pub login_fail_retry_wait_sec: Option<String>,
    pub trash_recycle_days: Option<String>,
    pub datetime_format: Option<String>,
    pub sms_cache_send_key_prefix: Option<String>,
    pub register_review_reject_notify_enabled: Option<String>,
    pub legal_user_agreement: Option<String>,
    pub legal_privacy_policy: Option<String>,
    pub fetch_enable_ssrf_protection: Option<String>,
    pub fetch_allow_private_ip: Option<String>,
    pub fetch_domain_filter_mode: Option<String>,
    pub fetch_ip_filter_mode: Option<String>,
    pub fetch_domain_list: Option<String>,
    pub fetch_ip_list: Option<String>,
    pub fetch_allowed_ports: Option<String>,
    pub fetch_apply_ip_filter_for_domain: Option<String>,
    pub oidc_enabled: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_well_known: Option<String>,
    pub oidc_authorization_endpoint: Option<String>,
    pub oidc_token_endpoint: Option<String>,
    pub oidc_user_info_endpoint: Option<String>,
    pub checkin_enabled: Option<String>,
    pub checkin_min_quota: Option<String>,
    pub checkin_max_quota: Option<String>,
    pub subscription_payment_allow_combined: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicContentResponse {
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicLegalResponse {
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicNoticeResponse {
    pub title: String,
    pub content: String,
    pub popup_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicPricingItem {
    pub model_code: String,
    pub name: String,
    pub model_type: String,
    pub currency: String,
    pub input_price: f64,
    pub output_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    pub patch_multiplier: f64,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicPricingResponse {
    pub currency: String,
    pub models: Vec<PublicPricingItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicLevelRatioItem {
    pub level: String,
    pub level_name: String,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicRatioConfigResponse {
    pub default_patch_multiplier: f64,
    pub levels: Vec<PublicLevelRatioItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AdminModelRatioItem {
    pub model_code: String,
    pub name: String,
    pub model_type: String,
    pub patch_multiplier: f64,
    pub input_price: f64,
    pub output_price: f64,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AdminRatioConfigResponse {
    pub default_patch_multiplier: f64,
    pub levels: Vec<PublicLevelRatioItem>,
    pub models: Vec<AdminModelRatioItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateModelPatchMultiplierItem {
    pub model_code: String,
    pub patch_multiplier: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateAdminRatioConfigRequest {
    pub default_patch_multiplier: Option<f64>,
    pub model_patch_multipliers: Option<Vec<UpdateModelPatchMultiplierItem>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RatioSyncRequest {
    pub apply_default_to_null_only: Option<bool>,
    pub normalize_non_positive: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RatioSyncResponse {
    pub scanned_models: i32,
    pub updated_models: i32,
    pub success_models: i32,
    pub failed_models: i32,
    pub skipped_models: i32,
    pub synchronized_from_external: bool,
    pub default_patch_multiplier: f64,
    pub sync_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TestSmtpRequest {
    pub to_email: Option<String>,
}

const DEFAULT_PATCH_MULTIPLIER_CONFIG_KEY: &str = "system.pricing.default_patch_multiplier";
const RATIO_SYNC_ENABLED_CONFIG_KEY: &str = "system.ratio_sync.enabled";
const RATIO_SYNC_SOURCE_URL_CONFIG_KEY: &str = "system.ratio_sync.source_url";
const RATIO_SYNC_API_KEY_CONFIG_KEY: &str = "system.ratio_sync.api_key";

fn normalize_bool_string(value: &mut Option<String>) {
    if let Some(raw) = value.as_ref() {
        let normalized = if ["1", "true", "yes", "on"]
            .iter()
            .any(|v| raw.eq_ignore_ascii_case(v))
        {
            "true"
        } else {
            "false"
        };
        *value = Some(normalized.to_string());
    }
}

fn validate_optional_json_array(value: &Option<String>, field_name: &str) -> Result<(), String> {
    let Some(raw) = value else {
        return Ok(());
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let parsed: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| format!("{} 不是合法 JSON: {}", field_name, e))?;
    if !parsed.is_array() {
        return Err(format!("{} 必须是 JSON 数组", field_name));
    }

    Ok(())
}

fn validate_optional_positive_i64(
    value: &Option<String>,
    field_name: &str,
    min: i64,
    max: i64,
) -> Result<(), String> {
    let Some(raw) = value else {
        return Ok(());
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let parsed = trimmed
        .parse::<i64>()
        .map_err(|_| format!("{} 必须是整数", field_name))?;

    if parsed < min || parsed > max {
        return Err(format!("{} 必须在 {} 到 {} 之间", field_name, min, max));
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/service/list",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<ServiceStatusVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<ServiceStatusVO>>)
    ),
    tag = "service"
)]
pub async fn get_service_list(_req: Request) -> impl IntoResponse {
    let start_time = std::time::Instant::now();

    let services = vec![
        ServiceStatusVO {
            name: "database".to_string(),
            status: check_database_health().await,
            description: "数据库服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
        ServiceStatusVO {
            name: "cache".to_string(),
            status: check_cache_health().await,
            description: "缓存服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
        ServiceStatusVO {
            name: "auth".to_string(),
            status: check_auth_health().await,
            description: "认证服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
        ServiceStatusVO {
            name: "rbac".to_string(),
            status: check_rbac_health().await,
            description: "权限控制服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
    ];

    (StatusCode::OK, Json(ApiResponse::success(services)))
}

pub async fn get_public_branding_config(
    State(context): State<Arc<ServiceContext>>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let data = PublicBrandingConfig {
        site_name: KeyValueConfig::get_value(&mut conn, "system.site_name", "RSLLM")
            .await
            .unwrap_or_else(|_| "RSLLM".to_string()),
        panel_title: KeyValueConfig::get_value(&mut conn, "system.panel_title", "RSLLM Panel")
            .await
            .unwrap_or_else(|_| "RSLLM Panel".to_string()),
        logo_url: KeyValueConfig::get_value(&mut conn, "system.logo_url", "/rsllm.png")
            .await
            .unwrap_or_else(|_| "/rsllm.png".to_string()),
        logo_dark_url: KeyValueConfig::get_value(&mut conn, "system.logo_dark_url", "/rsllm.png")
            .await
            .unwrap_or_else(|_| "/rsllm.png".to_string()),
        official_about_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_title",
            "关于 RSLLM",
        )
        .await
        .unwrap_or_else(|_| "关于 RSLLM".to_string()),
        official_about_content: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_content",
            "RSLLM 致力于为团队提供稳定、高效、可治理的大模型中台能力。",
        )
        .await
        .unwrap_or_else(|_| {
            "RSLLM 致力于为团队提供稳定、高效、可治理的大模型中台能力。".to_string()
        }),
        official_about_highlights_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_highlights_title",
            "为什么选择 RSLLM",
        )
        .await
        .unwrap_or_else(|_| "为什么选择 RSLLM".to_string()),
        official_about_highlights_items: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_highlights_items",
            "[]",
        )
        .await
        .unwrap_or_else(|_| "[]".to_string()),
        official_about_timeline_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_timeline_title",
            "发展历程",
        )
        .await
        .unwrap_or_else(|_| "发展历程".to_string()),
        official_about_timeline_items: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_timeline_items",
            "[]",
        )
        .await
        .unwrap_or_else(|_| "[]".to_string()),
        official_announcements_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_announcements_popup_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_popup_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_announcements_popup_once: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_popup_once",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_announcements_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_title",
            "平台公告",
        )
        .await
        .unwrap_or_else(|_| "平台公告".to_string()),
        official_announcements_items: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_items",
            "[]",
        )
        .await
        .unwrap_or_else(|_| "[]".to_string()),
        official_home_page_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.official_home_page_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_home_page_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_home_page_title",
            "RSLLM · AI 中转站",
        )
        .await
        .unwrap_or_else(|_| "RSLLM · AI 中转站".to_string()),
        official_home_page_content: KeyValueConfig::get_value(
            &mut conn,
            "system.official_home_page_content",
            "# 欢迎使用 RSLLM\n\n请在系统配置中自定义首页内容。",
        )
        .await
        .unwrap_or_else(|_| "# 欢迎使用 RSLLM\n\n请在系统配置中自定义首页内容。".to_string()),
        openai_api_scheme: KeyValueConfig::get_value(
            &mut conn,
            "system.openai_api_scheme",
            "https",
        )
        .await
        .unwrap_or_else(|_| "https".to_string()),
        openai_api_host: KeyValueConfig::get_value(
            &mut conn,
            "system.openai_api_host",
            "api.example.com",
        )
        .await
        .unwrap_or_else(|_| "api.example.com".to_string()),
        openai_api_port: KeyValueConfig::get_value(&mut conn, "system.openai_api_port", "")
            .await
            .unwrap_or_else(|_| "".to_string()),
        openai_api_path: KeyValueConfig::get_value(&mut conn, "system.openai_api_path", "/v1")
            .await
            .unwrap_or_else(|_| "/v1".to_string()),
        subscription_payment_allow_combined: KeyValueConfig::get_value(
            &mut conn,
            "system.subscription_payment_allow_combined",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        legal_user_agreement: KeyValueConfig::get_value(
            &mut conn,
            "system.legal.user_agreement",
            "",
        )
        .await
        .unwrap_or_else(|_| "".to_string()),
        legal_privacy_policy: KeyValueConfig::get_value(
            &mut conn,
            "system.legal.privacy_policy",
            "",
        )
        .await
        .unwrap_or_else(|_| "".to_string()),
        fetch_enable_ssrf_protection: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.enable_ssrf_protection",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        fetch_allow_private_ip: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.allow_private_ip",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        fetch_domain_filter_mode: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.domain_filter_mode",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        fetch_ip_filter_mode: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.ip_filter_mode",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        fetch_domain_list: KeyValueConfig::get_value(&mut conn, "system.fetch.domain_list", "[]")
            .await
            .unwrap_or_else(|_| "[]".to_string()),
        fetch_ip_list: KeyValueConfig::get_value(&mut conn, "system.fetch.ip_list", "[]")
            .await
            .unwrap_or_else(|_| "[]".to_string()),
        fetch_allowed_ports: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.allowed_ports",
            "[\"80\",\"443\",\"8080\",\"8443\"]",
        )
        .await
        .unwrap_or_else(|_| "[\"80\",\"443\",\"8080\",\"8443\"]".to_string()),
        fetch_apply_ip_filter_for_domain: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.apply_ip_filter_for_domain",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        checkin_enabled: KeyValueConfig::get_value(&mut conn, "system.checkin.enabled", "false")
            .await
            .unwrap_or_else(|_| "false".to_string()),
        checkin_min_quota: KeyValueConfig::get_value(&mut conn, "system.checkin.min_quota", "1000")
            .await
            .unwrap_or_else(|_| "1000".to_string()),
        checkin_max_quota: KeyValueConfig::get_value(
            &mut conn,
            "system.checkin.max_quota",
            "10000",
        )
        .await
        .unwrap_or_else(|_| "10000".to_string()),

        register_email_verify_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.register_email_verify_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        register_captcha_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.register_captcha_enabled",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        allow_register: KeyValueConfig::get_value(&mut conn, "system.allow_register", "true")
            .await
            .unwrap_or_else(|_| "true".to_string()),
        invite_code_required: KeyValueConfig::get_value(
            &mut conn,
            "system.invite_code_required",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        register_review_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.register_review_enabled",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        invite_code_bypass_review: KeyValueConfig::get_value(
            &mut conn,
            "system.invite_code_bypass_review",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
    };

    (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
}

pub async fn get_system_config(State(context): State<Arc<ServiceContext>>) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let data = SystemConfigResponse {
        site_name: KeyValueConfig::get_value(&mut conn, "system.site_name", "RSLLM")
            .await
            .unwrap_or_else(|_| "RSLLM".to_string()),
        panel_title: KeyValueConfig::get_value(&mut conn, "system.panel_title", "RSLLM Panel")
            .await
            .unwrap_or_else(|_| "RSLLM Panel".to_string()),
        logo_url: KeyValueConfig::get_value(&mut conn, "system.logo_url", "/rsllm.png")
            .await
            .unwrap_or_else(|_| "/rsllm.png".to_string()),
        logo_dark_url: KeyValueConfig::get_value(&mut conn, "system.logo_dark_url", "/rsllm.png")
            .await
            .unwrap_or_else(|_| "/rsllm.png".to_string()),
        official_about_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_title",
            "关于 RSLLM",
        )
        .await
        .unwrap_or_else(|_| "关于 RSLLM".to_string()),
        official_about_content: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_content",
            "RSLLM 致力于为团队提供稳定、高效、可治理的大模型中台能力。",
        )
        .await
        .unwrap_or_else(|_| {
            "RSLLM 致力于为团队提供稳定、高效、可治理的大模型中台能力。".to_string()
        }),
        official_about_highlights_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_highlights_title",
            "为什么选择 RSLLM",
        )
        .await
        .unwrap_or_else(|_| "为什么选择 RSLLM".to_string()),
        official_about_highlights_items: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_highlights_items",
            "[]",
        )
        .await
        .unwrap_or_else(|_| "[]".to_string()),
        official_about_timeline_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_timeline_title",
            "发展历程",
        )
        .await
        .unwrap_or_else(|_| "发展历程".to_string()),
        official_about_timeline_items: KeyValueConfig::get_value(
            &mut conn,
            "system.official_about_timeline_items",
            "[]",
        )
        .await
        .unwrap_or_else(|_| "[]".to_string()),
        official_announcements_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_announcements_popup_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_popup_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_announcements_popup_once: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_popup_once",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_announcements_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_title",
            "平台公告",
        )
        .await
        .unwrap_or_else(|_| "平台公告".to_string()),
        official_announcements_items: KeyValueConfig::get_value(
            &mut conn,
            "system.official_announcements_items",
            "[]",
        )
        .await
        .unwrap_or_else(|_| "[]".to_string()),
        official_home_page_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.official_home_page_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        official_home_page_title: KeyValueConfig::get_value(
            &mut conn,
            "system.official_home_page_title",
            "RSLLM · AI 中转站",
        )
        .await
        .unwrap_or_else(|_| "RSLLM · AI 中转站".to_string()),
        official_home_page_content: KeyValueConfig::get_value(
            &mut conn,
            "system.official_home_page_content",
            "# 欢迎使用 RSLLM\n\n请在系统配置中自定义首页内容。",
        )
        .await
        .unwrap_or_else(|_| "# 欢迎使用 RSLLM\n\n请在系统配置中自定义首页内容。".to_string()),
        openai_api_scheme: KeyValueConfig::get_value(
            &mut conn,
            "system.openai_api_scheme",
            "https",
        )
        .await
        .unwrap_or_else(|_| "https".to_string()),
        openai_api_host: KeyValueConfig::get_value(
            &mut conn,
            "system.openai_api_host",
            "api.example.com",
        )
        .await
        .unwrap_or_else(|_| "api.example.com".to_string()),
        openai_api_port: KeyValueConfig::get_value(&mut conn, "system.openai_api_port", "")
            .await
            .unwrap_or_else(|_| "".to_string()),
        openai_api_path: KeyValueConfig::get_value(&mut conn, "system.openai_api_path", "/v1")
            .await
            .unwrap_or_else(|_| "/v1".to_string()),
        subscription_payment_allow_combined: KeyValueConfig::get_value(
            &mut conn,
            "system.subscription_payment_allow_combined",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        legal_user_agreement: KeyValueConfig::get_value(
            &mut conn,
            "system.legal.user_agreement",
            "",
        )
        .await
        .unwrap_or_else(|_| "".to_string()),
        legal_privacy_policy: KeyValueConfig::get_value(
            &mut conn,
            "system.legal.privacy_policy",
            "",
        )
        .await
        .unwrap_or_else(|_| "".to_string()),
        fetch_enable_ssrf_protection: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.enable_ssrf_protection",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        fetch_allow_private_ip: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.allow_private_ip",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        fetch_domain_filter_mode: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.domain_filter_mode",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        fetch_ip_filter_mode: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.ip_filter_mode",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        fetch_domain_list: KeyValueConfig::get_value(&mut conn, "system.fetch.domain_list", "[]")
            .await
            .unwrap_or_else(|_| "[]".to_string()),
        fetch_ip_list: KeyValueConfig::get_value(&mut conn, "system.fetch.ip_list", "[]")
            .await
            .unwrap_or_else(|_| "[]".to_string()),
        fetch_allowed_ports: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.allowed_ports",
            "[\"80\",\"443\",\"8080\",\"8443\"]",
        )
        .await
        .unwrap_or_else(|_| "[\"80\",\"443\",\"8080\",\"8443\"]".to_string()),
        fetch_apply_ip_filter_for_domain: KeyValueConfig::get_value(
            &mut conn,
            "system.fetch.apply_ip_filter_for_domain",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        oidc_enabled: KeyValueConfig::get_value(&mut conn, "system.oidc.enabled", "false")
            .await
            .unwrap_or_else(|_| "false".to_string()),
        oidc_client_id: KeyValueConfig::get_value(&mut conn, "system.oidc.client_id", "")
            .await
            .unwrap_or_else(|_| "".to_string()),
        oidc_client_secret: KeyValueConfig::get_value(&mut conn, "system.oidc.client_secret", "")
            .await
            .unwrap_or_else(|_| "".to_string()),
        oidc_well_known: KeyValueConfig::get_value(&mut conn, "system.oidc.well_known", "")
            .await
            .unwrap_or_else(|_| "".to_string()),
        oidc_authorization_endpoint: KeyValueConfig::get_value(
            &mut conn,
            "system.oidc.authorization_endpoint",
            "",
        )
        .await
        .unwrap_or_else(|_| "".to_string()),
        oidc_token_endpoint: KeyValueConfig::get_value(&mut conn, "system.oidc.token_endpoint", "")
            .await
            .unwrap_or_else(|_| "".to_string()),
        oidc_user_info_endpoint: KeyValueConfig::get_value(
            &mut conn,
            "system.oidc.user_info_endpoint",
            "",
        )
        .await
        .unwrap_or_else(|_| "".to_string()),
        checkin_enabled: KeyValueConfig::get_value(&mut conn, "system.checkin.enabled", "false")
            .await
            .unwrap_or_else(|_| "false".to_string()),
        checkin_min_quota: KeyValueConfig::get_value(&mut conn, "system.checkin.min_quota", "1000")
            .await
            .unwrap_or_else(|_| "1000".to_string()),
        checkin_max_quota: KeyValueConfig::get_value(
            &mut conn,
            "system.checkin.max_quota",
            "10000",
        )
        .await
        .unwrap_or_else(|_| "10000".to_string()),
        smtp_host: KeyValueConfig::get_value(&mut conn, "system.smtp_host", "")
            .await
            .unwrap_or_default(),
        smtp_port: KeyValueConfig::get_value(&mut conn, "system.smtp_port", "465")
            .await
            .unwrap_or_else(|_| "465".to_string()),
        smtp_username: KeyValueConfig::get_value(&mut conn, "system.smtp_username", "")
            .await
            .unwrap_or_default(),
        smtp_password: KeyValueConfig::get_value(&mut conn, "system.smtp_password", "")
            .await
            .unwrap_or_default(),
        smtp_from: KeyValueConfig::get_value(&mut conn, "system.smtp_from", "")
            .await
            .unwrap_or_default(),
        smtp_from_name: KeyValueConfig::get_value(&mut conn, "system.smtp_from_name", "RSLLM")
            .await
            .unwrap_or_else(|_| "RSLLM".to_string()),
        smtp_starttls: KeyValueConfig::get_value(&mut conn, "system.smtp_starttls", "true")
            .await
            .unwrap_or_else(|_| "true".to_string()),
        register_email_verify_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.register_email_verify_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        register_captcha_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.register_captcha_enabled",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        allow_register: KeyValueConfig::get_value(&mut conn, "system.allow_register", "true")
            .await
            .unwrap_or_else(|_| "true".to_string()),
        invite_code_required: KeyValueConfig::get_value(
            &mut conn,
            "system.invite_code_required",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        register_review_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.register_review_enabled",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        invite_code_bypass_review: KeyValueConfig::get_value(
            &mut conn,
            "system.invite_code_bypass_review",
            "false",
        )
        .await
        .unwrap_or_else(|_| "false".to_string()),
        register_review_reject_notify_enabled: KeyValueConfig::get_value(
            &mut conn,
            "system.register_review_reject_notify_enabled",
            "true",
        )
        .await
        .unwrap_or_else(|_| "true".to_string()),
        login_fail_retry: KeyValueConfig::get_value(
            &mut conn,
            "system.login_fail_retry",
            &CONTEXT.config.login_fail_retry.to_string(),
        )
        .await
        .unwrap_or_else(|_| CONTEXT.config.login_fail_retry.to_string()),
        login_fail_retry_wait_sec: KeyValueConfig::get_value(
            &mut conn,
            "system.login_fail_retry_wait_sec",
            &CONTEXT.config.login_fail_retry_wait_sec.to_string(),
        )
        .await
        .unwrap_or_else(|_| CONTEXT.config.login_fail_retry_wait_sec.to_string()),
        trash_recycle_days: KeyValueConfig::get_value(
            &mut conn,
            "system.trash_recycle_days",
            &CONTEXT.config.trash_recycle_days.to_string(),
        )
        .await
        .unwrap_or_else(|_| CONTEXT.config.trash_recycle_days.to_string()),
        datetime_format: KeyValueConfig::get_value(
            &mut conn,
            "system.datetime_format",
            &CONTEXT.config.datetime_format,
        )
        .await
        .unwrap_or_else(|_| CONTEXT.config.datetime_format.clone()),
        sms_cache_send_key_prefix: KeyValueConfig::get_value(
            &mut conn,
            "system.sms_cache_send_key_prefix",
            &CONTEXT.config.sms_cache_send_key_prefix,
        )
        .await
        .unwrap_or_else(|_| CONTEXT.config.sms_cache_send_key_prefix.clone()),
    };

    (StatusCode::OK, Json(ApiResponse::success(data))).into_response()
}

pub async fn update_system_config(
    State(context): State<Arc<ServiceContext>>,
    Json(mut req): Json<SystemConfigUpdateRequest>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let invalid_field_response = |message: String| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "INVALID_CONFIG",
                &message,
            )),
        )
            .into_response()
    };

    let json_fields = [
        (
            &req.official_about_highlights_items,
            "official_about_highlights_items",
        ),
        (
            &req.official_about_timeline_items,
            "official_about_timeline_items",
        ),
        (&req.fetch_domain_list, "fetch_domain_list"),
        (&req.fetch_ip_list, "fetch_ip_list"),
        (&req.fetch_allowed_ports, "fetch_allowed_ports"),
    ];

    for (value, field_name) in json_fields {
        if let Err(err) = validate_optional_json_array(value, field_name) {
            return invalid_field_response(err);
        }
    }

    if let Err(err) =
        validate_optional_positive_i64(&req.checkin_min_quota, "checkin_min_quota", 1, 100000000)
    {
        return invalid_field_response(err);
    }
    if let Err(err) =
        validate_optional_positive_i64(&req.checkin_max_quota, "checkin_max_quota", 1, 100000000)
    {
        return invalid_field_response(err);
    }
    if let (Some(min_raw), Some(max_raw)) = (
        req.checkin_min_quota.as_ref(),
        req.checkin_max_quota.as_ref(),
    ) {
        let min_trimmed = min_raw.trim();
        let max_trimmed = max_raw.trim();
        if !min_trimmed.is_empty() && !max_trimmed.is_empty() {
            let min_parsed = match min_trimmed.parse::<i64>() {
                Ok(v) => v,
                Err(_) => {
                    return invalid_field_response("checkin_min_quota 必须是整数".to_string());
                }
            };
            let max_parsed = match max_trimmed.parse::<i64>() {
                Ok(v) => v,
                Err(_) => {
                    return invalid_field_response("checkin_max_quota 必须是整数".to_string());
                }
            };
            if min_parsed > max_parsed {
                return invalid_field_response(
                    "checkin_min_quota 不能大于 checkin_max_quota".to_string(),
                );
            }
        }
    }

    if let Some(port) = req.smtp_port.as_ref() {
        if !port.trim().is_empty() {
            let parsed_port = port.trim().parse::<u16>();
            if parsed_port.is_err() {
                return invalid_field_response("smtp_port 必须是 1-65535 的数字端口".to_string());
            }
        }
    }

    normalize_bool_string(&mut req.checkin_enabled);
    normalize_bool_string(&mut req.official_announcements_enabled);
    normalize_bool_string(&mut req.official_announcements_popup_enabled);
    normalize_bool_string(&mut req.official_announcements_popup_once);
    normalize_bool_string(&mut req.official_home_page_enabled);
    normalize_bool_string(&mut req.smtp_starttls);
    normalize_bool_string(&mut req.register_email_verify_enabled);
    normalize_bool_string(&mut req.register_captcha_enabled);
    normalize_bool_string(&mut req.allow_register);
    normalize_bool_string(&mut req.invite_code_required);
    normalize_bool_string(&mut req.register_review_enabled);
    normalize_bool_string(&mut req.invite_code_bypass_review);
    normalize_bool_string(&mut req.fetch_enable_ssrf_protection);
    normalize_bool_string(&mut req.fetch_allow_private_ip);
    normalize_bool_string(&mut req.fetch_domain_filter_mode);
    normalize_bool_string(&mut req.fetch_ip_filter_mode);
    normalize_bool_string(&mut req.fetch_apply_ip_filter_for_domain);
    normalize_bool_string(&mut req.oidc_enabled);

    macro_rules! save_opt {
        ($field:expr, $key:expr, $desc:expr) => {
            if let Some(v) = $field {
                if let Err(e) = KeyValueConfig::set_value(&mut conn, $key, v.trim(), $desc).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<serde_json::Value>::error(
                            "CONFIG_SAVE_ERROR",
                            &format!("保存配置 {} 失败: {}", $key, e),
                        )),
                    )
                        .into_response();
                }
            }
        };
    }

    save_opt!(req.site_name, "system.site_name", Some("站点名称"));
    save_opt!(req.panel_title, "system.panel_title", Some("面板标题"));
    save_opt!(req.logo_url, "system.logo_url", Some("站点logo"));
    save_opt!(req.logo_dark_url, "system.logo_dark_url", Some("深色logo"));
    save_opt!(
        req.official_about_title,
        "system.official_about_title",
        Some("官网关于标题")
    );
    save_opt!(
        req.official_about_content,
        "system.official_about_content",
        Some("官网关于内容")
    );
    save_opt!(
        req.official_about_highlights_title,
        "system.official_about_highlights_title",
        Some("官网关于亮点标题")
    );
    save_opt!(
        req.official_about_highlights_items,
        "system.official_about_highlights_items",
        Some("官网关于亮点项")
    );
    save_opt!(
        req.official_about_timeline_title,
        "system.official_about_timeline_title",
        Some("官网时间线标题")
    );
    save_opt!(
        req.official_about_timeline_items,
        "system.official_about_timeline_items",
        Some("官网时间线项")
    );
    save_opt!(
        req.official_announcements_enabled,
        "system.official_announcements_enabled",
        Some("官网公告开关")
    );
    save_opt!(
        req.official_announcements_popup_enabled,
        "system.official_announcements_popup_enabled",
        Some("官网公告弹窗开关")
    );
    save_opt!(
        req.official_announcements_popup_once,
        "system.official_announcements_popup_once",
        Some("官网公告弹窗仅一次")
    );
    save_opt!(
        req.official_announcements_title,
        "system.official_announcements_title",
        Some("官网公告标题")
    );
    save_opt!(
        req.official_announcements_items,
        "system.official_announcements_items",
        Some("官网公告列表")
    );
    save_opt!(
        req.official_home_page_enabled,
        "system.official_home_page_enabled",
        Some("官网首页启用")
    );
    save_opt!(
        req.official_home_page_title,
        "system.official_home_page_title",
        Some("官网首页标题")
    );
    save_opt!(
        req.official_home_page_content,
        "system.official_home_page_content",
        Some("官网首页内容")
    );
    save_opt!(req.smtp_host, "system.smtp_host", Some("SMTP主机"));
    save_opt!(req.smtp_port, "system.smtp_port", Some("SMTP端口"));
    save_opt!(
        req.smtp_username,
        "system.smtp_username",
        Some("SMTP用户名")
    );
    save_opt!(req.smtp_password, "system.smtp_password", Some("SMTP密码"));
    save_opt!(req.smtp_from, "system.smtp_from", Some("发件邮箱"));
    save_opt!(
        req.smtp_from_name,
        "system.smtp_from_name",
        Some("发件人名称")
    );
    save_opt!(
        req.smtp_starttls,
        "system.smtp_starttls",
        Some("SMTP STARTTLS")
    );
    save_opt!(
        req.subscription_payment_allow_combined,
        "system.subscription_payment_allow_combined",
        Some("订阅允许组合支付")
    );
    save_opt!(
        req.register_email_verify_enabled,
        "system.register_email_verify_enabled",
        Some("注册邮箱验证开关")
    );
    save_opt!(
        req.register_captcha_enabled,
        "system.register_captcha_enabled",
        Some("注册图形验证码开关")
    );
    save_opt!(
        req.allow_register,
        "system.allow_register",
        Some("允许用户注册")
    );
    save_opt!(
        req.invite_code_required,
        "system.invite_code_required",
        Some("注册需邀请码")
    );
    save_opt!(
        req.register_review_enabled,
        "system.register_review_enabled",
        Some("注册审核开关")
    );
    save_opt!(
        req.invite_code_bypass_review,
        "system.invite_code_bypass_review",
        Some("邀请码免审核")
    );
    save_opt!(
        req.register_review_reject_notify_enabled,
        "system.register_review_reject_notify_enabled",
        Some("审核拒绝通知")
    );

    save_opt!(
        req.legal_user_agreement,
        "system.legal.user_agreement",
        Some("用户协议")
    );
    save_opt!(
        req.legal_privacy_policy,
        "system.legal.privacy_policy",
        Some("隐私政策")
    );

    save_opt!(
        req.fetch_enable_ssrf_protection,
        "system.fetch.enable_ssrf_protection",
        Some("启用SSRF防护")
    );
    save_opt!(
        req.fetch_allow_private_ip,
        "system.fetch.allow_private_ip",
        Some("允许私网IP")
    );
    save_opt!(
        req.fetch_domain_filter_mode,
        "system.fetch.domain_filter_mode",
        Some("域名过滤模式")
    );
    save_opt!(
        req.fetch_ip_filter_mode,
        "system.fetch.ip_filter_mode",
        Some("IP过滤模式")
    );
    save_opt!(
        req.fetch_domain_list,
        "system.fetch.domain_list",
        Some("域名过滤列表")
    );
    save_opt!(
        req.fetch_ip_list,
        "system.fetch.ip_list",
        Some("IP过滤列表")
    );
    save_opt!(
        req.fetch_allowed_ports,
        "system.fetch.allowed_ports",
        Some("允许端口列表")
    );
    save_opt!(
        req.fetch_apply_ip_filter_for_domain,
        "system.fetch.apply_ip_filter_for_domain",
        Some("域名启用IP过滤")
    );

    save_opt!(
        req.checkin_enabled,
        "system.checkin.enabled",
        Some("签到开关")
    );
    save_opt!(
        req.checkin_min_quota,
        "system.checkin.min_quota",
        Some("签到最小额度")
    );
    save_opt!(
        req.checkin_max_quota,
        "system.checkin.max_quota",
        Some("签到最大额度")
    );

    save_opt!(req.oidc_enabled, "system.oidc.enabled", Some("OIDC启用"));
    save_opt!(
        req.oidc_client_id,
        "system.oidc.client_id",
        Some("OIDC客户端ID")
    );
    save_opt!(
        req.oidc_client_secret,
        "system.oidc.client_secret",
        Some("OIDC客户端密钥")
    );
    save_opt!(
        req.oidc_well_known,
        "system.oidc.well_known",
        Some("OIDC Well-Known地址")
    );
    save_opt!(
        req.oidc_authorization_endpoint,
        "system.oidc.authorization_endpoint",
        Some("OIDC授权端点")
    );
    save_opt!(
        req.oidc_token_endpoint,
        "system.oidc.token_endpoint",
        Some("OIDC令牌端点")
    );
    save_opt!(
        req.oidc_user_info_endpoint,
        "system.oidc.user_info_endpoint",
        Some("OIDC用户信息端点")
    );

    // 运行时生效配置（迁移自 application.json5 -> KeyValueConfig）
    save_opt!(
        req.login_fail_retry,
        "system.login_fail_retry",
        Some("登录失败重试次数")
    );
    save_opt!(
        req.login_fail_retry_wait_sec,
        "system.login_fail_retry_wait_sec",
        Some("登录失败重试等待秒数")
    );
    save_opt!(
        req.trash_recycle_days,
        "system.trash_recycle_days",
        Some("回收站回收天数")
    );
    save_opt!(
        req.datetime_format,
        "system.datetime_format",
        Some("日期时间格式")
    );
    save_opt!(
        req.sms_cache_send_key_prefix,
        "system.sms_cache_send_key_prefix",
        Some("短信发送缓存键前缀")
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "message": "系统配置已更新"
        }))),
    )
        .into_response()
}

pub async fn test_system_smtp(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<TestSmtpRequest>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let smtp_host = KeyValueConfig::get_value(&mut conn, "system.smtp_host", "")
        .await
        .unwrap_or_default();
    let smtp_port = KeyValueConfig::get_value(&mut conn, "system.smtp_port", "465")
        .await
        .unwrap_or_else(|_| "465".to_string());
    let smtp_username = KeyValueConfig::get_value(&mut conn, "system.smtp_username", "")
        .await
        .unwrap_or_default();
    let smtp_password = KeyValueConfig::get_value(&mut conn, "system.smtp_password", "")
        .await
        .unwrap_or_default();
    let smtp_from = KeyValueConfig::get_value(&mut conn, "system.smtp_from", "")
        .await
        .unwrap_or_default();
    let smtp_from_name = KeyValueConfig::get_value(&mut conn, "system.smtp_from_name", "RSLLM")
        .await
        .unwrap_or_else(|_| "RSLLM".to_string());
    let smtp_starttls = KeyValueConfig::get_value(&mut conn, "system.smtp_starttls", "true")
        .await
        .unwrap_or_else(|_| "true".to_string());

    if smtp_host.is_empty()
        || smtp_username.is_empty()
        || smtp_password.is_empty()
        || smtp_from.is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "SMTP_NOT_CONFIGURED",
                "系统未配置完整发信服务",
            )),
        )
            .into_response();
    }

    let to_email = req
        .to_email
        .unwrap_or_else(|| smtp_from.clone())
        .trim()
        .to_string();
    if to_email.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "INVALID_TO_EMAIL",
                "请提供测试收件邮箱",
            )),
        )
            .into_response();
    }

    let port = smtp_port.parse::<u16>().unwrap_or(465);
    let starttls = !smtp_starttls.eq_ignore_ascii_case("false");

    let from_address = match smtp_from.parse() {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "SMTP_FROM_INVALID",
                    "发件人邮箱配置无效",
                )),
            )
                .into_response();
        }
    };
    let to_address = match to_email.parse() {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "INVALID_TO_EMAIL",
                    "收件人邮箱格式无效",
                )),
            )
                .into_response();
        }
    };

    let message = match Message::builder()
        .from(Mailbox::new(Some(smtp_from_name), from_address))
        .to(to_address)
        .subject("RSLLM SMTP 测试邮件")
        .body(
            "这是一封来自 RSLLM 系统配置页面的测试邮件，若您收到此邮件，表示SMTP配置可用。"
                .to_string(),
        ) {
        Ok(v) => v,
        Err(_e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "EMAIL_BUILD_FAILED",
                    "测试邮件构造失败",
                )),
            )
                .into_response();
        }
    };

    let credentials = Credentials::new(smtp_username, smtp_password);
    let mailer = if starttls {
        match AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host) {
            Ok(builder) => builder.port(port).credentials(credentials).build(),
            Err(_e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<serde_json::Value>::error(
                        "SMTP_RELAY_FAILED",
                        "SMTP配置错误",
                    )),
                )
                    .into_response();
            }
        }
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_host)
            .port(port)
            .credentials(credentials)
            .build()
    };

    if let Err(e) = mailer.send(message).await {
        log::warn!("[smtp-test] send failed: {}", e);
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "EMAIL_SEND_FAILED",
                "测试邮件发送失败",
            )),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "message": format!("测试邮件发送成功，收件人：{}", to_email)
        }))),
    )
        .into_response()
}

pub async fn get_public_notice(State(context): State<Arc<ServiceContext>>) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let title =
        KeyValueConfig::get_value(&mut conn, "system.official_announcements_title", "平台公告")
            .await
            .unwrap_or_else(|_| "平台公告".to_string());
    let popup_enabled = KeyValueConfig::get_value(
        &mut conn,
        "system.official_announcements_popup_enabled",
        "true",
    )
    .await
    .unwrap_or_else(|_| "true".to_string())
    .parse::<bool>()
    .unwrap_or(true);

    let content = {
        let list: Vec<OfficialAnnouncement> = conn
            .query_decode(
                "SELECT * FROM official_announcement WHERE enabled = 1 ORDER BY sort_order DESC, updated_at DESC, created_at DESC",
                vec![],
            )
            .await
            .unwrap_or_default();

        if !list.is_empty() {
            let items: Vec<serde_json::Value> = list
                .into_iter()
                .map(|a| {
                    let tabs: serde_json::Value =
                        serde_json::from_str(&a.content).unwrap_or_else(|_| serde_json::json!([]));
                    let updated_at = a.updated_at.map(|dt| dt.to_string()).unwrap_or_default();
                    let updated_at = updated_at.split(' ').next().unwrap_or("").to_string();
                    serde_json::json!({
                        "id": a.id,
                        "title": a.title,
                        "summary": a.summary,
                        "enabled": a.enabled != 0,
                        "popup": a.popup != 0,
                        "updatedAt": updated_at,
                        "tabs": tabs
                    })
                })
                .collect();
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
        } else {
            KeyValueConfig::get_value(&mut conn, "system.official_announcements_items", "[]")
                .await
                .unwrap_or_else(|_| "[]".to_string())
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(PublicNoticeResponse {
            title,
            content,
            popup_enabled,
        })),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/user-agreement",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PublicLegalResponse>),
        (status = 500, description = "查询失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "service"
)]
pub async fn get_public_user_agreement(
    State(context): State<Arc<ServiceContext>>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let content = KeyValueConfig::get_value(&mut conn, "system.legal.user_agreement", "")
        .await
        .unwrap_or_else(|_| "".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(PublicLegalResponse { content })),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/privacy-policy",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PublicLegalResponse>),
        (status = 500, description = "查询失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "service"
)]
pub async fn get_public_privacy_policy(
    State(context): State<Arc<ServiceContext>>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let content = KeyValueConfig::get_value(&mut conn, "system.legal.privacy_policy", "")
        .await
        .unwrap_or_else(|_| "".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(PublicLegalResponse { content })),
    )
        .into_response()
}

pub async fn get_public_about(State(context): State<Arc<ServiceContext>>) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let content = KeyValueConfig::get_value(
        &mut conn,
        "system.official_about_content",
        "RSLLM 致力于为团队提供稳定、高效、可治理的大模型中台能力。",
    )
    .await
    .unwrap_or_else(|_| "RSLLM 致力于为团队提供稳定、高效、可治理的大模型中台能力。".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(PublicContentResponse { content })),
    )
        .into_response()
}

pub async fn get_public_home_page_content(
    State(context): State<Arc<ServiceContext>>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let content = KeyValueConfig::get_value(
        &mut conn,
        "system.official_home_page_content",
        "# 欢迎使用 RSLLM\n\n请在系统配置中自定义首页内容。",
    )
    .await
    .unwrap_or_else(|_| "# 欢迎使用 RSLLM\n\n请在系统配置中自定义首页内容。".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(PublicContentResponse { content })),
    )
        .into_response()
}

pub async fn get_public_pricing(State(context): State<Arc<ServiceContext>>) -> impl IntoResponse {
    let conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let mut models = ModelBase::select_active_ratio_page(&conn, 1, 2000)
        .await
        .unwrap_or_default();
    if models.is_empty() {
        models = ModelBase::select_ratio_page(&conn, 1, 2000)
            .await
            .unwrap_or_default();
    }

    let mut pricing_models: Vec<PublicPricingItem> = models
        .into_iter()
        .map(|m| PublicPricingItem {
            model_code: m.model_code,
            name: m.name,
            model_type: m.model_type,
            currency: m.currency.unwrap_or_else(|| "USD".to_string()),
            input_price: m.input_price,
            output_price: m.output_price,
            price_unit: m.price_unit,
            patch_multiplier: m.patch_multiplier.unwrap_or(1.0),
            status: m.status.unwrap_or_else(|| "active".to_string()),
        })
        .collect();

    pricing_models.sort_by(|a, b| a.model_code.cmp(&b.model_code));

    (
        StatusCode::OK,
        Json(ApiResponse::success(PublicPricingResponse {
            currency: "USD".to_string(),
            models: pricing_models,
        })),
    )
        .into_response()
}

pub async fn get_public_ratio_config(
    State(context): State<Arc<ServiceContext>>,
) -> impl IntoResponse {
    let conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let mut levels = UserLevelConfig::select_all(&conn).await.unwrap_or_default();
    levels.retain(|item| item.enabled == 1);

    let mut level_items: Vec<PublicLevelRatioItem> = levels
        .into_iter()
        .map(|item| PublicLevelRatioItem {
            level: item.level,
            level_name: item.level_name,
            rpm_limit: item.rpm_limit,
            tpm_limit: item.tpm_limit,
        })
        .collect();
    level_items.sort_by(|a, b| a.level.cmp(&b.level));

    (
        StatusCode::OK,
        Json(ApiResponse::success(PublicRatioConfigResponse {
            default_patch_multiplier: 1.0,
            levels: level_items,
        })),
    )
        .into_response()
}

pub async fn get_admin_ratio_config(
    State(context): State<Arc<ServiceContext>>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let default_patch_multiplier =
        KeyValueConfig::get_value(&mut conn, DEFAULT_PATCH_MULTIPLIER_CONFIG_KEY, "1.0")
            .await
            .unwrap_or_else(|_| "1.0".to_string())
            .parse::<f64>()
            .ok()
            .filter(|v| *v > 0.0)
            .unwrap_or(1.0);

    let mut levels = UserLevelConfig::select_all(&conn).await.unwrap_or_default();
    levels.retain(|item| item.enabled == 1);
    let mut level_items: Vec<PublicLevelRatioItem> = levels
        .into_iter()
        .map(|item| PublicLevelRatioItem {
            level: item.level,
            level_name: item.level_name,
            rpm_limit: item.rpm_limit,
            tpm_limit: item.tpm_limit,
        })
        .collect();
    level_items.sort_by(|a, b| a.level.cmp(&b.level));

    let models = ModelBase::select_ratio_page(&conn, 1, 5000)
        .await
        .unwrap_or_default();
    let model_items: Vec<AdminModelRatioItem> = models
        .into_iter()
        .map(|item| AdminModelRatioItem {
            model_code: item.model_code,
            name: item.name,
            model_type: item.model_type,
            patch_multiplier: item.patch_multiplier.unwrap_or(default_patch_multiplier),
            input_price: item.input_price,
            output_price: item.output_price,
            status: item.status.unwrap_or_else(|| "active".to_string()),
        })
        .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminRatioConfigResponse {
            default_patch_multiplier,
            levels: level_items,
            models: model_items,
        })),
    )
        .into_response()
}

pub async fn update_admin_ratio_config(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<UpdateAdminRatioConfigRequest>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let mut updated_models = 0u64;
    let mut updated_default = false;

    if let Some(default_ratio) = req.default_patch_multiplier {
        if !default_ratio.is_finite() || default_ratio <= 0.0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "INVALID_RATIO",
                    "default_patch_multiplier 必须是大于 0 的数字",
                )),
            )
                .into_response();
        }

        if let Err(e) = KeyValueConfig::set_value(
            &mut conn,
            DEFAULT_PATCH_MULTIPLIER_CONFIG_KEY,
            &default_ratio.to_string(),
            Some("默认倍率配置"),
        )
        .await
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "CONFIG_SAVE_ERROR",
                    &format!("保存默认倍率失败: {}", e),
                )),
            )
                .into_response();
        }
        updated_default = true;
    }

    if let Some(model_ratios) = req.model_patch_multipliers {
        let now = Utc::now().to_rfc3339();
        for item in model_ratios {
            if item.model_code.trim().is_empty()
                || !item.patch_multiplier.is_finite()
                || item.patch_multiplier <= 0.0
            {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<serde_json::Value>::error(
                        "INVALID_RATIO",
                        "model_code 不能为空，patch_multiplier 必须大于 0",
                    )),
                )
                    .into_response();
            }

            match conn
                .exec(
                    "UPDATE model_base SET patch_multiplier = ?, updated_at = ? WHERE model_code = ?",
                    vec![
                        rbs::value!(item.patch_multiplier),
                        rbs::value!(now.clone()),
                        rbs::value!(item.model_code.trim()),
                    ],
                )
                .await
            {
                Ok(result) => {
                    updated_models += result.rows_affected;
                }
                Err(e) => {
                    log::warn!("[ratio-config] update model ratio failed: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<serde_json::Value>::error(
                            "MODEL_RATIO_SAVE_ERROR",
                            "更新模型倍率失败",
                        )),
                    )
                        .into_response();
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "updated_default": updated_default,
            "updated_models": updated_models
        }))),
    )
        .into_response()
}

pub async fn sync_admin_ratio_config(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<RatioSyncRequest>,
) -> impl IntoResponse {
    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "DB_ERROR",
                    "数据库连接失败",
                )),
            )
                .into_response();
        }
    };

    let default_patch_multiplier =
        KeyValueConfig::get_value(&mut conn, DEFAULT_PATCH_MULTIPLIER_CONFIG_KEY, "1.0")
            .await
            .unwrap_or_else(|_| "1.0".to_string())
            .parse::<f64>()
            .ok()
            .filter(|v| *v > 0.0)
            .unwrap_or(1.0);

    let apply_default_to_null_only = req.apply_default_to_null_only.unwrap_or(true);
    let normalize_non_positive = req.normalize_non_positive.unwrap_or(true);

    let ratio_sync_enabled_raw =
        KeyValueConfig::get_value(&mut conn, RATIO_SYNC_ENABLED_CONFIG_KEY, "false")
            .await
            .unwrap_or_else(|_| "false".to_string());
    let ratio_sync_enabled = parse_config_bool(&ratio_sync_enabled_raw);

    let ratio_sync_source_url =
        KeyValueConfig::get_value(&mut conn, RATIO_SYNC_SOURCE_URL_CONFIG_KEY, "")
            .await
            .unwrap_or_default()
            .trim()
            .to_string();
    let ratio_sync_api_key =
        KeyValueConfig::get_value(&mut conn, RATIO_SYNC_API_KEY_CONFIG_KEY, "")
            .await
            .unwrap_or_default()
            .trim()
            .to_string();

    let models = ModelBase::select_ratio_page(&conn, 1, 5000)
        .await
        .unwrap_or_default();
    let scanned_models = models.len() as i32;
    let mut updated_models = 0i32;
    let mut success_models = 0i32;
    let mut failed_models = 0i32;
    let mut skipped_models = 0i32;
    let now = Utc::now().to_rfc3339();

    let should_sync_external = ratio_sync_enabled && !ratio_sync_source_url.is_empty();
    let mut synchronized_from_external = false;
    let mut sync_message: Option<String> = None;
    let external_ratio_map = if should_sync_external {
        match fetch_external_ratio_map(&ratio_sync_source_url, &ratio_sync_api_key).await {
            Ok(map) => {
                synchronized_from_external = true;
                if map.is_empty() {
                    sync_message = Some("外部源返回空倍率列表，已按默认倍率规则补齐".to_string());
                }
                map
            }
            Err(e) => {
                log::warn!("[ratio-sync] external sync failed: {}", e);
                sync_message = Some("外部源同步失败，已按默认倍率规则补齐".to_string());
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    for model in models {
        let target_multiplier = if synchronized_from_external {
            external_ratio_map
                .get(&model.model_code)
                .copied()
                .filter(|v| *v > 0.0)
        } else {
            None
        };

        let fallback_multiplier = if apply_default_to_null_only {
            if model.patch_multiplier.is_none() {
                Some(default_patch_multiplier)
            } else {
                None
            }
        } else {
            match model.patch_multiplier {
                None => Some(default_patch_multiplier),
                Some(v) if normalize_non_positive && v <= 0.0 => Some(default_patch_multiplier),
                _ => None,
            }
        };

        let final_multiplier = target_multiplier.or(fallback_multiplier);

        let Some(next_multiplier) = final_multiplier else {
            skipped_models += 1;
            continue;
        };

        if model
            .patch_multiplier
            .map(|current| (current - next_multiplier).abs() < f64::EPSILON)
            .unwrap_or(false)
        {
            skipped_models += 1;
            continue;
        }

        let model_code = model.model_code;
        match conn
            .exec(
                "UPDATE model_base SET patch_multiplier = ?, updated_at = ? WHERE model_code = ?",
                vec![
                    rbs::value!(next_multiplier),
                    rbs::value!(now.clone()),
                    rbs::value!(model_code),
                ],
            )
            .await
        {
            Ok(result) if result.rows_affected > 0 => {
                updated_models += 1;
                success_models += 1;
            }
            Ok(_) => {
                failed_models += 1;
            }
            Err(_) => {
                failed_models += 1;
            }
        }
    }

    if should_sync_external
        && synchronized_from_external
        && sync_message.is_none()
        && !external_ratio_map.is_empty()
    {
        sync_message = Some(format!(
            "外部源同步完成，匹配 {} 个模型倍率",
            external_ratio_map.len()
        ));
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(RatioSyncResponse {
            scanned_models,
            updated_models,
            success_models,
            failed_models,
            skipped_models,
            synchronized_from_external,
            default_patch_multiplier,
            sync_message,
        })),
    )
        .into_response()
}

fn parse_config_bool(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn normalize_bearer_auth(raw_token: &str) -> Option<String> {
    let token = raw_token.trim();
    if token.is_empty() {
        return None;
    }
    if token.to_ascii_lowercase().starts_with("bearer ") {
        Some(token.to_string())
    } else {
        Some(format!("Bearer {}", token))
    }
}

async fn fetch_external_ratio_map(
    source_url: &str,
    api_key: &str,
) -> Result<HashMap<String, f64>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {}", e))?;

    let mut request = client.get(source_url);
    if let Some(auth_value) = normalize_bearer_auth(api_key) {
        request = request.header(AUTHORIZATION, auth_value);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("请求外部源失败: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let body_preview = if body.len() > 300 {
            format!("{}...", &body[..300])
        } else {
            body
        };
        return Err(format!(
            "外部源返回非成功状态码({})，响应: {}",
            status, body_preview
        ));
    }

    let payload: JsonValue = response
        .json()
        .await
        .map_err(|e| format!("解析外部源响应失败: {}", e))?;

    Ok(parse_external_ratio_payload(&payload))
}

fn parse_external_ratio_payload(payload: &JsonValue) -> HashMap<String, f64> {
    let mut ratio_map = HashMap::new();

    if let Some(obj) = payload.as_object() {
        for value in obj.values() {
            if let Some(entries) = value.as_array() {
                merge_ratio_entries(entries, &mut ratio_map);
            }
        }

        for (k, v) in obj {
            if let Some(multiplier) = parse_ratio_value(v) {
                ratio_map.insert(k.trim().to_string(), multiplier);
            }
        }
    }

    if let Some(entries) = payload.as_array() {
        merge_ratio_entries(entries, &mut ratio_map);
    }

    ratio_map
}

fn merge_ratio_entries(entries: &[JsonValue], ratio_map: &mut HashMap<String, f64>) {
    for item in entries {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let model_code = obj
            .get("model_code")
            .or_else(|| obj.get("model"))
            .or_else(|| obj.get("id"))
            .or_else(|| obj.get("code"))
            .or_else(|| obj.get("name"))
            .and_then(|v| v.as_str())
            .map(|v| v.trim())
            .filter(|v| !v.is_empty());

        let Some(model_code) = model_code else {
            continue;
        };

        let multiplier = obj
            .get("patch_multiplier")
            .or_else(|| obj.get("multiplier"))
            .or_else(|| obj.get("ratio"))
            .or_else(|| obj.get("price_ratio"))
            .and_then(parse_ratio_value)
            .filter(|v| *v > 0.0);

        if let Some(multiplier) = multiplier {
            ratio_map.insert(model_code.to_string(), multiplier);
        }
    }
}

fn parse_ratio_value(value: &JsonValue) -> Option<f64> {
    if let Some(v) = value.as_f64() {
        return Some(v);
    }
    value.as_str().and_then(|v| v.trim().parse::<f64>().ok())
}

async fn check_database_health() -> bool {
    CONTEXT.rbac_permission_service.finds_all().await.is_ok()
}

async fn check_cache_health() -> bool {
    true
}

async fn check_auth_health() -> bool {
    true
}

async fn check_rbac_health() -> bool {
    (CONTEXT.rbac_permission_service.finds_all().await).is_ok()
}
