use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

use crate::context::ServiceContext;
use crate::domain::dto::register::UserRegisterDTO;
use crate::domain::table::sys_user::SysUser;
use crate::domain::vo::basic::LoginVO;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;
use crate::pool;

const OAUTH_STATE_TTL_SECONDS: u64 = 600;
const OAUTH_STATE_KEY_PREFIX: &str = "oauth:user:state:";
const OAUTH_SUB_BIND_KEY_PREFIX: &str = "system.oidc.sub.";
const OAUTH_MAX_PARAM_LEN: usize = 2048;
const OAUTH_MAX_ERROR_DESC_LEN: usize = 256;
const OAUTH_MAX_EMAIL_LEN: usize = 320;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthStateResponse {
    pub state: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthStateQuery {
    #[serde(default)]
    pub aff: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OAuthCallbackQuery {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OidcWellKnown {
    #[serde(default)]
    authorization_endpoint: String,
    #[serde(default)]
    token_endpoint: String,
    #[serde(default)]
    userinfo_endpoint: String,
}

#[derive(Debug, Clone)]
struct OidcRuntimeConfig {
    token_endpoint: String,
    userinfo_endpoint: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthStatePayload {
    provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OidcTokenResponse {
    #[serde(default)]
    access_token: String,
}

#[derive(Debug, Clone)]
struct OidcUserIdentity {
    sub: String,
    email: String,
    email_verified: bool,
}

#[utoipa::path(
    get,
    path = "/oauth/state",
    params(
        ("aff" = Option<String>, Query, description = "aff code")
    ),
    responses(
        (status = 200, description = "获取 OAuth state 成功", body = ApiResponse<OAuthStateResponse>),
        (status = 400, description = "参数不合法", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn generate_oauth_state(
    State(context): State<Arc<ServiceContext>>,
    Query(_query): Query<OAuthStateQuery>,
) -> impl IntoResponse {
    let state = generate_url_safe_random(24);
    let payload = OAuthStatePayload {
        provider: "oidc".to_string(),
    };

    let payload_str = match serde_json::to_string(&payload) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<serde_json::Value>::error(
                    "INTERNAL_ERROR",
                    "生成state失败",
                )),
            )
                .into_response();
        }
    };

    let key = oauth_state_key(&state);
    if context
        .cache_service
        .set_string_ex(
            &key,
            &payload_str,
            Some(Duration::from_secs(OAUTH_STATE_TTL_SECONDS)),
        )
        .await
        .is_err()
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::error(
                "INTERNAL_ERROR",
                "保存state失败",
            )),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(OAuthStateResponse {
            state,
            expires_in: OAUTH_STATE_TTL_SECONDS,
        })),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/oauth/{provider}",
    params(
        ("provider" = String, Path, description = "oauth provider name，当前仅支持 oidc"),
        ("code" = Option<String>, Query, description = "OAuth code"),
        ("state" = Option<String>, Query, description = "OAuth state"),
        ("error" = Option<String>, Query, description = "OAuth error code"),
        ("error_description" = Option<String>, Query, description = "OAuth error description")
    ),
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<LoginVO>),
        (status = 400, description = "参数不合法", body = ApiResponse<serde_json::Value>),
        (status = 401, description = "登录失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn handle_oauth_callback(
    State(context): State<Arc<ServiceContext>>,
    Path(provider): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    let provider = provider.trim().to_lowercase();
    if provider != "oidc" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "UNSUPPORTED_PROVIDER",
                "当前仅支持 oidc provider",
            )),
        )
            .into_response();
    }

    if let Some(err_code) =
        sanitize_query_param(query.error.as_deref(), OAUTH_MAX_PARAM_LEN).filter(|v| !v.is_empty())
    {
        let desc =
            sanitize_query_param(query.error_description.as_deref(), OAUTH_MAX_ERROR_DESC_LEN)
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "OAuth provider 返回错误".to_string());

        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "OAUTH_PROVIDER_ERROR",
                &format!("{}: {}", err_code, desc),
            )),
        )
            .into_response();
    }

    let code = match sanitize_query_param(query.code.as_deref(), OAUTH_MAX_PARAM_LEN)
        .filter(|v| !v.is_empty())
    {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "INVALID_PARAMS",
                    "缺少 code 参数",
                )),
            )
                .into_response();
        }
    };

    let state = match sanitize_query_param(query.state.as_deref(), OAUTH_MAX_PARAM_LEN)
        .filter(|v| !v.is_empty())
    {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "INVALID_PARAMS",
                    "缺少 state 参数",
                )),
            )
                .into_response();
        }
    };

    let state_key = oauth_state_key(&state);
    let state_payload = match context.cache_service.get_string(&state_key).await {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "STATE_INVALID",
                    "state 无效或已过期",
                )),
            )
                .into_response();
        }
    };

    let _ = context.cache_service.del(&state_key).await;

    let payload: OAuthStatePayload = match serde_json::from_str(&state_payload) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "STATE_INVALID",
                    "state 数据格式错误",
                )),
            )
                .into_response();
        }
    };

    if payload.provider != "oidc" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "STATE_INVALID",
                "state provider 不匹配",
            )),
        )
            .into_response();
    }

    let oidc_config = match load_oidc_runtime_config().await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[oauth] oidc config invalid: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<serde_json::Value>::error(
                    "INVALID_CONFIG",
                    "OIDC 配置无效",
                )),
            )
                .into_response();
        }
    };

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| Client::new());

    let token = match exchange_token(&client, &oidc_config, &code, &context.config.server_url).await
    {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value>::error(
                    "OAUTH_EXCHANGE_FAILED",
                    "第三方认证失败",
                )),
            )
                .into_response();
        }
    };

    let identity = match fetch_user_identity(&client, &oidc_config, &token.access_token).await {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value>::error(
                    "OAUTH_USERINFO_FAILED",
                    "获取用户信息失败",
                )),
            )
                .into_response();
        }
    };

    if identity.sub.trim().is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<serde_json::Value>::error(
                "OAUTH_USERINFO_INVALID",
                "userinfo 缺少 sub",
            )),
        )
            .into_response();
    }

    if !identity.email_verified {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<serde_json::Value>::error(
                "OAUTH_USERINFO_INVALID",
                "userinfo 邮箱未验证",
            )),
        )
            .into_response();
    }

    let user = match find_or_create_oidc_user(&context, &identity).await {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[oauth] find/create user failed: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value>::error(
                    "OAUTH_LOGIN_FAILED",
                    "OAuth 登录失败",
                )),
            )
                .into_response();
        }
    };

    if user.state == Some(0) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<serde_json::Value>::error(
                "ACCOUNT_DISABLED",
                "账户已被禁用",
            )),
        )
            .into_response();
    }

    let account = user.account.as_deref().unwrap_or_default().trim();
    if account.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<serde_json::Value>::error(
                "ACCOUNT_INVALID",
                "用户账户信息异常",
            )),
        )
            .into_response();
    }

    let sign_vo = match context.sys_user_service.get_user_info(&user).await {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<serde_json::Value>::error(
                    "OAUTH_LOGIN_FAILED",
                    "登录态创建失败",
                )),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(LoginVO {
            access_token: sign_vo.access_token,
        })),
    )
        .into_response()
}

async fn load_oidc_runtime_config() -> ApplicationResult<OidcRuntimeConfig> {
    let conn = pool!();

    let enabled = crate::domain::table::key_value_config::KeyValueConfig::get_value(
        conn,
        "system.oidc.enabled",
        "false",
    )
    .await
    .unwrap_or_else(|_| "false".to_string())
    .to_lowercase();

    if enabled != "true" {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 登录未开启".to_string(),
            field: Some("system.oidc.enabled".to_string()),
            value: Some(enabled),
        });
    }

    let client_id = crate::domain::table::key_value_config::KeyValueConfig::get_value(
        conn,
        "system.oidc.client_id",
        "",
    )
    .await
    .unwrap_or_default()
    .trim()
    .to_string();

    let client_secret = crate::domain::table::key_value_config::KeyValueConfig::get_value(
        conn,
        "system.oidc.client_secret",
        "",
    )
    .await
    .unwrap_or_default()
    .trim()
    .to_string();

    let mut authorization_endpoint =
        crate::domain::table::key_value_config::KeyValueConfig::get_value(
            conn,
            "system.oidc.authorization_endpoint",
            "",
        )
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

    let mut token_endpoint = crate::domain::table::key_value_config::KeyValueConfig::get_value(
        conn,
        "system.oidc.token_endpoint",
        "",
    )
    .await
    .unwrap_or_default()
    .trim()
    .to_string();

    let mut userinfo_endpoint = crate::domain::table::key_value_config::KeyValueConfig::get_value(
        conn,
        "system.oidc.user_info_endpoint",
        "",
    )
    .await
    .unwrap_or_default()
    .trim()
    .to_string();

    let well_known = crate::domain::table::key_value_config::KeyValueConfig::get_value(
        conn,
        "system.oidc.well_known",
        "",
    )
    .await
    .unwrap_or_default()
    .trim()
    .to_string();

    if (!authorization_endpoint.is_empty() && token_endpoint.is_empty())
        || (!token_endpoint.is_empty() && authorization_endpoint.is_empty())
    {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 配置不完整：authorization_endpoint 与 token_endpoint 需同时配置"
                .to_string(),
            field: Some("system.oidc.authorization_endpoint".to_string()),
            value: None,
        });
    }

    if (authorization_endpoint.is_empty()
        || token_endpoint.is_empty()
        || userinfo_endpoint.is_empty())
        && !well_known.is_empty()
    {
        let well_known_url = validate_oidc_url(&well_known, "system.oidc.well_known")?;
        let wk = fetch_well_known(well_known_url.as_str()).await?;
        if authorization_endpoint.is_empty() {
            authorization_endpoint = wk.authorization_endpoint;
        }
        if token_endpoint.is_empty() {
            token_endpoint = wk.token_endpoint;
        }
        if userinfo_endpoint.is_empty() {
            userinfo_endpoint = wk.userinfo_endpoint;
        }
    }

    if client_id.is_empty() {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 配置缺少 client_id".to_string(),
            field: Some("system.oidc.client_id".to_string()),
            value: None,
        });
    }

    if client_secret.is_empty() {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 配置缺少 client_secret".to_string(),
            field: Some("system.oidc.client_secret".to_string()),
            value: None,
        });
    }

    if authorization_endpoint.is_empty() {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 配置缺少 authorization_endpoint".to_string(),
            field: Some("system.oidc.authorization_endpoint".to_string()),
            value: None,
        });
    }

    if token_endpoint.is_empty() {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 配置缺少 token_endpoint".to_string(),
            field: Some("system.oidc.token_endpoint".to_string()),
            value: None,
        });
    }

    if userinfo_endpoint.is_empty() {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 配置缺少 user_info_endpoint".to_string(),
            field: Some("system.oidc.user_info_endpoint".to_string()),
            value: None,
        });
    }

    let auth_url = validate_oidc_url(
        &authorization_endpoint,
        "system.oidc.authorization_endpoint",
    )?;
    let token_url = validate_oidc_url(&token_endpoint, "system.oidc.token_endpoint")?;
    let userinfo_url = validate_oidc_url(&userinfo_endpoint, "system.oidc.user_info_endpoint")?;

    if !same_origin(&auth_url, &token_url) || !same_origin(&auth_url, &userinfo_url) {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC 配置不合法：端点必须属于同一来源".to_string(),
            field: Some("system.oidc.authorization_endpoint".to_string()),
            value: None,
        });
    }

    Ok(OidcRuntimeConfig {
        token_endpoint: token_url.to_string(),
        userinfo_endpoint: userinfo_url.to_string(),
        client_id,
        client_secret,
    })
}

async fn fetch_well_known(url: &str) -> ApplicationResult<OidcWellKnown> {
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .unwrap_or_else(|_| Client::new());

    let resp = client.get(url).send().await.map_err(|e| {
        crate::error::ApplicationError::ExternalServiceError {
            message: format!("拉取 OIDC well-known 失败: {}", e),
            service: Some("oidc".to_string()),
            endpoint: Some(url.to_string()),
        }
    })?;

    if !resp.status().is_success() {
        return Err(crate::error::ApplicationError::ExternalServiceError {
            message: format!("拉取 OIDC well-known 失败，状态码 {}", resp.status()),
            service: Some("oidc".to_string()),
            endpoint: Some(url.to_string()),
        });
    }

    let wk = resp.json::<OidcWellKnown>().await.map_err(|e| {
        crate::error::ApplicationError::ExternalServiceError {
            message: format!("解析 OIDC well-known 失败: {}", e),
            service: Some("oidc".to_string()),
            endpoint: Some(url.to_string()),
        }
    })?;

    Ok(wk)
}

async fn exchange_token(
    client: &Client,
    config: &OidcRuntimeConfig,
    code: &str,
    server_url: &str,
) -> ApplicationResult<OidcTokenResponse> {
    let redirect_uri = build_oauth_callback_url(server_url, "oidc")?;

    let params = [
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("client_id", config.client_id.clone()),
        ("client_secret", config.client_secret.clone()),
        ("redirect_uri", redirect_uri),
    ];

    let resp = client
        .post(&config.token_endpoint)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| crate::error::ApplicationError::ExternalServiceError {
            message: format!("OIDC token 交换失败: {}", e),
            service: Some("oidc".to_string()),
            endpoint: Some(config.token_endpoint.clone()),
        })?;

    if !resp.status().is_success() {
        return Err(crate::error::ApplicationError::ExternalServiceError {
            message: format!("OIDC token 交换失败，状态码 {}", resp.status()),
            service: Some("oidc".to_string()),
            endpoint: Some(config.token_endpoint.clone()),
        });
    }

    let token = resp.json::<OidcTokenResponse>().await.map_err(|e| {
        crate::error::ApplicationError::ExternalServiceError {
            message: format!("解析 OIDC token 响应失败: {}", e),
            service: Some("oidc".to_string()),
            endpoint: Some(config.token_endpoint.clone()),
        }
    })?;

    if token.access_token.trim().is_empty() {
        return Err(crate::error::ApplicationError::ExternalServiceError {
            message: "OIDC token 响应缺少 access_token".to_string(),
            service: Some("oidc".to_string()),
            endpoint: Some(config.token_endpoint.clone()),
        });
    }

    Ok(token)
}

async fn fetch_user_identity(
    client: &Client,
    config: &OidcRuntimeConfig,
    access_token: &str,
) -> ApplicationResult<OidcUserIdentity> {
    let resp = client
        .get(&config.userinfo_endpoint)
        .header("Accept", "application/json")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| crate::error::ApplicationError::ExternalServiceError {
            message: format!("请求 OIDC userinfo 失败: {}", e),
            service: Some("oidc".to_string()),
            endpoint: Some(config.userinfo_endpoint.clone()),
        })?;

    if !resp.status().is_success() {
        return Err(crate::error::ApplicationError::ExternalServiceError {
            message: format!("请求 OIDC userinfo 失败，状态码 {}", resp.status()),
            service: Some("oidc".to_string()),
            endpoint: Some(config.userinfo_endpoint.clone()),
        });
    }

    let value = resp.json::<JsonValue>().await.map_err(|e| {
        crate::error::ApplicationError::ExternalServiceError {
            message: format!("解析 OIDC userinfo 失败: {}", e),
            service: Some("oidc".to_string()),
            endpoint: Some(config.userinfo_endpoint.clone()),
        }
    })?;

    let email = extract_string(&value, &["email", "upn", "preferred_email"]).to_lowercase();
    let sub = extract_string(&value, &["sub", "id", "user_id"]);
    let email_verified = extract_bool(&value, &["email_verified"]);

    Ok(OidcUserIdentity {
        sub,
        email,
        email_verified,
    })
}

fn extract_string(value: &JsonValue, keys: &[&str]) -> String {
    for key in keys {
        if let Some(v) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    String::new()
}

fn extract_bool(value: &JsonValue, keys: &[&str]) -> bool {
    for key in keys {
        if let Some(v) = value.get(*key) {
            if let Some(b) = v.as_bool() {
                return b;
            }
            if let Some(s) = v.as_str() {
                let lowered = s.trim().to_lowercase();
                if lowered == "true" || lowered == "1" || lowered == "yes" {
                    return true;
                }
                if lowered == "false" || lowered == "0" || lowered == "no" {
                    return false;
                }
            }
            if let Some(n) = v.as_i64() {
                return n != 0;
            }
        }
    }
    false
}

async fn find_or_create_oidc_user(
    context: &Arc<ServiceContext>,
    identity: &OidcUserIdentity,
) -> ApplicationResult<SysUser> {
    let email = identity.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "userinfo 缺少 email".to_string(),
            field: Some("email".to_string()),
            value: None,
        });
    }
    if email.len() > OAUTH_MAX_EMAIL_LEN {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "userinfo email 长度超限".to_string(),
            field: Some("email".to_string()),
            value: None,
        });
    }

    let sub = identity.sub.trim();
    if !sub.is_empty() {
        let bind_key = oauth_sub_bind_key(sub);
        if let Some(user_id) = get_oidc_subject_binding(pool!(), &bind_key).await? {
            if let Some(user) = load_user_by_id(&user_id).await? {
                if user.state == Some(0) {
                    return Err(crate::error::ApplicationError::BusinessError {
                        message: "账户已被禁用".to_string(),
                        code: Some("ACCOUNT_DISABLED".to_string()),
                        context: Some("oauth_login".to_string()),
                    });
                }
                return Ok(user);
            }
        }
    }

    if let Some(existing) = SysUser::select_by_map(pool!(), rbs::value! {"email": &email})
        .await
        .map_err(|e| crate::error::ApplicationError::DatabaseError {
            message: e.to_string(),
            operation: Some("select_by_email".to_string()),
            table: Some("sys_user".to_string()),
        })?
        .into_iter()
        .next()
    {
        if !sub.is_empty() {
            persist_oidc_subject_binding(
                pool!(),
                &oauth_sub_bind_key(sub),
                existing.id.as_deref().unwrap_or_default(),
            )
            .await?;
        }
        if existing.state == Some(0) {
            return Err(crate::error::ApplicationError::BusinessError {
                message: "账户已被禁用".to_string(),
                code: Some("ACCOUNT_DISABLED".to_string()),
                context: Some("oauth_login".to_string()),
            });
        }
        return Ok(existing);
    }

    let policy = crate::service::RegisterPolicyService::get_policy(&context.rb).await;
    if !policy.allow_register {
        return Err(crate::error::ApplicationError::BusinessError {
            message: "当前站点已关闭用户注册".to_string(),
            code: Some("REGISTER_DISABLED".to_string()),
            context: Some("oauth_login".to_string()),
        });
    }

    let account = build_unique_account(context, &email).await?;
    let random_password = generate_url_safe_random(18);

    let register_dto = UserRegisterDTO {
        name: account,
        email: email.clone(),
        password: random_password,
        user_level: None,
        agree_terms: true,
    };

    context
        .sys_user_service
        .register(&register_dto)
        .await
        .map_err(|e| crate::error::ApplicationError::BusinessError {
            message: e.to_string(),
            code: Some("REGISTER_FAILED".to_string()),
            context: Some("oauth_login".to_string()),
        })?;

    let created = SysUser::select_by_map(pool!(), rbs::value! {"email": &email})
        .await
        .map_err(|e| crate::error::ApplicationError::DatabaseError {
            message: e.to_string(),
            operation: Some("select_created_user".to_string()),
            table: Some("sys_user".to_string()),
        })?
        .into_iter()
        .next()
        .ok_or_else(|| crate::error::ApplicationError::DatabaseError {
            message: "创建用户后未找到记录".to_string(),
            operation: Some("select_created_user".to_string()),
            table: Some("sys_user".to_string()),
        })?;

    if !sub.is_empty() {
        persist_oidc_subject_binding(
            pool!(),
            &oauth_sub_bind_key(sub),
            created.id.as_deref().unwrap_or_default(),
        )
        .await?;
    }

    Ok(created)
}

async fn build_unique_account(
    context: &Arc<ServiceContext>,
    email: &str,
) -> ApplicationResult<String> {
    let base_local = email
        .split('@')
        .next()
        .unwrap_or("oidc")
        .trim()
        .to_lowercase();
    let mut base = sanitize_account_base(&base_local);
    if base.is_empty() {
        base = "oidc".to_string();
    }

    let mut candidate = format!("oidc_{}", base);
    for idx in 0..50_u32 {
        let exists = context
            .sys_user_service
            .find_by_account(&candidate)
            .await
            .map_err(|e| crate::error::ApplicationError::DatabaseError {
                message: e.to_string(),
                operation: Some("find_by_account".to_string()),
                table: Some("sys_user".to_string()),
            })?;
        if exists.is_none() {
            return Ok(candidate);
        }
        candidate = format!("oidc_{}_{:02}", base, idx + 1);
    }

    Ok(format!(
        "oidc_{}",
        generate_url_safe_random(6).to_lowercase()
    ))
}

fn sanitize_account_base(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        }
        if out.len() >= 32 {
            break;
        }
    }
    out
}

fn oauth_state_key(state: &str) -> String {
    format!("{}{}", OAUTH_STATE_KEY_PREFIX, state)
}

fn oauth_sub_bind_key(sub: &str) -> String {
    format!("{}{}", OAUTH_SUB_BIND_KEY_PREFIX, sub)
}

fn generate_url_safe_random(bytes_len: usize) -> String {
    let mut bytes = vec![0_u8; bytes_len];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn build_oauth_callback_url(server_url: &str, provider: &str) -> ApplicationResult<String> {
    let mut url = reqwest::Url::parse(server_url).map_err(|e| {
        crate::error::ApplicationError::ValidationError {
            message: format!("server_url 不合法: {}", e),
            field: Some("server_url".to_string()),
            value: Some(server_url.to_string()),
        }
    })?;

    if url.host_str() == Some("0.0.0.0") {
        url.set_host(Some("127.0.0.1")).map_err(|_| {
            crate::error::ApplicationError::ValidationError {
                message: "server_url host 非法".to_string(),
                field: Some("server_url".to_string()),
                value: Some(server_url.to_string()),
            }
        })?;
    }

    url.set_path(&format!("/rsllm/api/oauth/{}", provider));
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

fn sanitize_query_param(input: Option<&str>, max_len: usize) -> Option<String> {
    let raw = input?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Some(String::new());
    }
    if trimmed.len() > max_len {
        return None;
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return None;
    }
    Some(trimmed.to_string())
}

fn validate_oidc_url(url: &str, field: &str) -> ApplicationResult<reqwest::Url> {
    let parsed =
        reqwest::Url::parse(url).map_err(|e| crate::error::ApplicationError::ValidationError {
            message: format!("OIDC URL 不合法: {}", e),
            field: Some(field.to_string()),
            value: Some(url.to_string()),
        })?;

    let scheme = parsed.scheme().to_lowercase();
    if scheme != "https" {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC URL 必须使用 https".to_string(),
            field: Some(field.to_string()),
            value: Some(url.to_string()),
        });
    }

    if let Some(host) = parsed.host_str() {
        let lowered = host.to_lowercase();
        if lowered == "localhost" || lowered == "0.0.0.0" || lowered == "127.0.0.1" {
            return Err(crate::error::ApplicationError::ValidationError {
                message: "OIDC URL 不允许使用本地地址".to_string(),
                field: Some(field.to_string()),
                value: Some(url.to_string()),
            });
        }

        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            if is_disallowed_ip(&ip) {
                return Err(crate::error::ApplicationError::ValidationError {
                    message: "OIDC URL 不允许使用内网地址".to_string(),
                    field: Some(field.to_string()),
                    value: Some(url.to_string()),
                });
            }
        }
    } else {
        return Err(crate::error::ApplicationError::ValidationError {
            message: "OIDC URL 缺少 host".to_string(),
            field: Some(field.to_string()),
            value: Some(url.to_string()),
        });
    }

    Ok(parsed)
}

fn same_origin(a: &reqwest::Url, b: &reqwest::Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

fn is_disallowed_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_unspecified()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_multicast()
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.is_multicast()
        }
    }
}

async fn get_oidc_subject_binding(
    conn: &impl rbatis::executor::Executor,
    key: &str,
) -> ApplicationResult<Option<String>> {
    let value = crate::domain::table::key_value_config::KeyValueConfig::get_value(conn, key, "")
        .await
        .map_err(|e| crate::error::ApplicationError::DatabaseError {
            message: e.to_string(),
            operation: Some("get_oidc_subject_binding".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

async fn persist_oidc_subject_binding(
    conn: &impl rbatis::executor::Executor,
    key: &str,
    user_id: &str,
) -> ApplicationResult<()> {
    if user_id.trim().is_empty() {
        return Ok(());
    }

    crate::domain::table::key_value_config::KeyValueConfig::set_value(
        conn,
        key,
        user_id,
        Some("OIDC Subject 绑定用户"),
    )
    .await
    .map_err(|e| crate::error::ApplicationError::DatabaseError {
        message: e.to_string(),
        operation: Some("persist_oidc_subject_binding".to_string()),
        table: Some("key_value_config".to_string()),
    })?;

    Ok(())
}

async fn load_user_by_id(user_id: &str) -> ApplicationResult<Option<SysUser>> {
    let users = SysUser::select_by_map(pool!(), rbs::value! {"id": user_id})
        .await
        .map_err(|e| crate::error::ApplicationError::DatabaseError {
            message: e.to_string(),
            operation: Some("select_by_id".to_string()),
            table: Some("sys_user".to_string()),
        })?;

    Ok(users.into_iter().next())
}
