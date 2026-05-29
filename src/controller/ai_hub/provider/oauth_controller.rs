use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Redirect, Response},
};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};
use crate::service::ai_hub::provider::oauth::provider_accounts_from_auth_config;
use crate::service::ai_hub::{
    ProviderConfigService, ProviderOAuthAccount, ProviderOAuthAccountImportItem,
    ProviderOAuthAccountsImportRequest, ProviderOAuthAccountsUpdateRequest,
    ProviderOAuthCallbackRequest, ProviderOAuthDevicePollRequest, ProviderOAuthDevicePollResponse,
    ProviderOAuthDisconnectRequest, ProviderOAuthStartRequest, ProviderOAuthStatusResponse,
};

fn fallback_account_key_from_file_name(entry_name: &str) -> String {
    let file_name = std::path::Path::new(entry_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .trim();
    file_name.to_string()
}

fn sanitize_export_file_stem(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "account".to_string();
    }

    let sanitized: String = trimmed
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    let candidate = sanitized.trim_matches('.').trim();
    if candidate.is_empty() {
        "account".to_string()
    } else {
        candidate.to_string()
    }
}

fn build_export_item(account: &ProviderOAuthAccount) -> ProviderOAuthAccountImportItem {
    ProviderOAuthAccountImportItem {
        account_key: account.account_key.clone(),
        access_token: account.access_token.clone(),
        refresh_token: account.refresh_token.clone(),
        id_token: account.id_token.clone(),
        token_type: account.token_type.clone(),
        email: account.email.clone(),
        account_id: account.account_id.clone(),
        access_token_expires_at: account.access_token_expires_at.clone(),
        enabled: account.enabled,
        weight: account.weight,
    }
}

#[derive(Debug, serde::Serialize)]
struct ProviderOAuthAccountExportFile {
    #[serde(rename = "type")]
    provider_type: String,
    email: String,
    expired: String,
    id_token: String,
    account_id: String,
    access_token: String,
    last_refresh: String,
    refresh_token: String,
    token_type: String,
}

fn build_export_file_item(
    account: &ProviderOAuthAccount,
    provider_type: &str,
) -> ProviderOAuthAccountExportFile {
    ProviderOAuthAccountExportFile {
        provider_type: provider_type.to_string(),
        email: account.email.clone(),
        expired: account.access_token_expires_at.clone(),
        id_token: account.id_token.clone(),
        account_id: account.account_id.clone(),
        access_token: account.access_token.clone(),
        last_refresh: account.updated_at.clone(),
        refresh_token: account.refresh_token.clone(),
        token_type: account.token_type.clone(),
    }
}

const MAX_EXPORT_ACCOUNTS_PER_PACKAGE: usize = 5000;

fn build_accounts_zip_bytes(
    accounts: &[ProviderOAuthAccount],
    provider_type: &str,
) -> Result<Vec<u8>> {
    let mut zip_writer = zip::ZipWriter::new(Cursor::new(Vec::<u8>::new()));
    let file_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut name_count_map: HashMap<String, usize> = HashMap::new();
    for account in accounts {
        let preferred = if !account.email.trim().is_empty() {
            account.email.as_str()
        } else if !account.account_key.trim().is_empty() {
            account.account_key.as_str()
        } else {
            "account"
        };

        let stem = sanitize_export_file_stem(preferred);
        let count = name_count_map.entry(stem.clone()).or_insert(0);
        let file_name = if *count == 0 {
            format!("{}.json", stem)
        } else {
            format!("{}_{}.json", stem, count)
        };
        *count += 1;

        let export_item = build_export_file_item(account, provider_type);
        let payload = serde_json::to_vec_pretty(&export_item).map_err(|e| {
            Error::BusinessError(format!("serialize oauth account export failed: {}", e))
        })?;

        zip_writer
            .start_file(file_name, file_options)
            .map_err(|e| Error::BusinessError(format!("zip start file failed: {}", e)))?;
        zip_writer
            .write_all(&payload)
            .map_err(|e| Error::BusinessError(format!("zip write file failed: {}", e)))?;
    }

    let zip_cursor = zip_writer
        .finish()
        .map_err(|e| Error::BusinessError(format!("zip finalize failed: {}", e)))?;
    Ok(zip_cursor.into_inner())
}

fn parse_jwt_exp_rfc3339(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload.as_bytes()).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let exp = value.get("exp")?.as_i64()?;
    chrono::DateTime::<chrono::Utc>::from_timestamp(exp, 0).map(|ts| ts.to_rfc3339())
}

fn refresh_account_expiry(account: &mut ProviderOAuthAccount) -> bool {
    let next_expiry = parse_jwt_exp_rfc3339(account.access_token.trim())
        .or_else(|| parse_jwt_exp_rfc3339(account.id_token.trim()));

    let mut changed = false;
    if let Some(expiry) = next_expiry
        && account.access_token_expires_at != expiry
    {
        account.access_token_expires_at = expiry;
        changed = true;
    }

    if changed {
        account.updated_at = chrono::Utc::now().to_rfc3339();
    }
    changed
}

#[derive(Debug, serde::Deserialize)]
pub struct ProviderOAuthPublicCallbackQuery {
    pub code: Option<String>,
    pub state: String,
    pub error: Option<String>,
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/oauth/start",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    request_body = ProviderOAuthStartRequest,
    responses(
        (status = 200, description = "启动成功", body = ApiResponse<crate::service::ai_hub::ProviderOAuthStartResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<crate::service::ai_hub::ProviderOAuthStartResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::service::ai_hub::ProviderOAuthStartResponse>)
    ),
    tag = "providers"
)]
pub async fn start_provider_oauth(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<ProviderOAuthStartRequest>,
) -> Result<Json<ApiResponse<crate::service::ai_hub::ProviderOAuthStartResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let resp = service.start_provider_oauth(&id, req).await?;
    Ok(Json(ApiResponse::success(resp)))
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/oauth/callback",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    request_body = ProviderOAuthCallbackRequest,
    responses(
        (status = 200, description = "回调处理成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn complete_provider_oauth(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<ProviderOAuthCallbackRequest>,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let status = service.complete_provider_oauth(&id, req).await?;
    Ok(Json(ApiResponse::success(status)))
}

pub async fn poll_provider_oauth_device(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<ProviderOAuthDevicePollRequest>,
) -> Result<Json<ApiResponse<ProviderOAuthDevicePollResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let status = service.poll_provider_oauth_device(&id, req).await?;
    Ok(Json(ApiResponse::success(status)))
}

#[utoipa::path(
    get,
    path = "/admin/providers/{id}/oauth/status",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn get_provider_oauth_status(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let status = service.get_provider_oauth_status(&id).await?;
    Ok(Json(ApiResponse::success(status)))
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/oauth/disconnect",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "断开成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn disconnect_provider_oauth(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    req: Option<Json<ProviderOAuthDisconnectRequest>>,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let req = req.map(|Json(payload)| payload).unwrap_or_default();
    let status = service.disconnect_provider_oauth(&id, req).await?;
    Ok(Json(ApiResponse::success(status)))
}

#[utoipa::path(
    put,
    path = "/admin/providers/{id}/oauth/accounts",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    request_body = ProviderOAuthAccountsUpdateRequest,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn update_provider_oauth_accounts(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<ProviderOAuthAccountsUpdateRequest>,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let status = service.update_provider_oauth_accounts(&id, req).await?;
    Ok(Json(ApiResponse::success(status)))
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/oauth/accounts/import",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    request_body = ProviderOAuthAccountsImportRequest,
    responses(
        (status = 200, description = "导入成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn import_provider_oauth_accounts(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<ProviderOAuthAccountsImportRequest>,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let status = service.import_provider_oauth_accounts(&id, req).await?;
    Ok(Json(ApiResponse::success(status)))
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/oauth/accounts/import/zip",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "ZIP导入成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn import_provider_oauth_accounts_zip(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let mut zip_bytes: Option<Vec<u8>> = None;
    let mut zip_file_name = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::ValidationError(format!("读取上传字段失败: {}", e)))?
    {
        let field_name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().unwrap_or_default().to_string();
        if field_name == "file" || file_name.to_lowercase().ends_with(".zip") {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| Error::ValidationError(format!("读取ZIP文件失败: {}", e)))?;
            zip_bytes = Some(bytes.to_vec());
            zip_file_name = file_name;
            break;
        }
    }

    let zip_bytes = zip_bytes.ok_or_else(|| {
        Error::ValidationError(
            "缺少 ZIP 文件字段，请使用 multipart form-data 并传 file".to_string(),
        )
    })?;

    if !zip_file_name.is_empty() && !zip_file_name.to_lowercase().ends_with(".zip") {
        return Err(Error::ValidationError(
            "上传文件必须是 .zip 格式".to_string(),
        ));
    }

    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes.as_slice()))
        .map_err(|e| Error::ValidationError(format!("ZIP 文件解析失败: {}", e)))?;

    let mut accounts: Vec<ProviderOAuthAccountImportItem> = Vec::new();

    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx).map_err(|e| {
            Error::ValidationError(format!("读取 ZIP 条目失败(index={}): {}", idx, e))
        })?;
        if file.is_dir() {
            continue;
        }

        let entry_name = file.name().to_string();

        if entry_name.starts_with("__MACOSX/") {
            continue;
        }

        let entry_path = std::path::Path::new(&entry_name);
        let component_count = entry_path.components().count();
        if component_count != 1 {
            continue;
        }

        let file_name = entry_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if file_name.starts_with("._") {
            continue;
        }

        if !file_name.to_lowercase().ends_with(".json") {
            continue;
        }

        let mut raw = Vec::new();
        file.read_to_end(&mut raw).map_err(|e| {
            Error::ValidationError(format!("读取 ZIP 文件条目失败({}): {}", entry_name, e))
        })?;

        let mut item: ProviderOAuthAccountImportItem =
            serde_json::from_slice(&raw).map_err(|e| {
                Error::ValidationError(format!("解析账号 JSON 失败({}): {}", entry_name, e))
            })?;

        if item.account_key.trim().is_empty() {
            item.account_key = fallback_account_key_from_file_name(&entry_name);
        }

        accounts.push(item);
    }

    if accounts.is_empty() {
        return Err(Error::ValidationError(
            "ZIP 内未找到可导入的账号 JSON 文件".to_string(),
        ));
    }

    let status = service
        .import_provider_oauth_accounts(&id, ProviderOAuthAccountsImportRequest { accounts })
        .await?;
    Ok(Json(ApiResponse::success(status)))
}

#[utoipa::path(
    get,
    path = "/admin/providers/{id}/oauth/accounts/export",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "导出成功，返回ZIP文件"),
        (status = 400, description = "参数错误", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn export_provider_oauth_accounts(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Response> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let provider = service.get_provider(&id).await?;
    let provider_type = provider.provider_type.trim().to_lowercase();
    if provider_type.is_empty() {
        return Err(Error::ValidationError(
            "provider_type cannot be empty".to_string(),
        ));
    }

    let accounts = provider_accounts_from_auth_config(&provider.auth_config, &provider_type);
    if accounts.is_empty() {
        return Err(Error::ValidationError(
            "no oauth accounts found for provider".to_string(),
        ));
    }

    let zip_bytes = if accounts.len() <= MAX_EXPORT_ACCOUNTS_PER_PACKAGE {
        build_accounts_zip_bytes(&accounts, &provider_type)?
    } else {
        let mut root_zip = zip::ZipWriter::new(Cursor::new(Vec::<u8>::new()));
        let root_file_options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for (index, chunk) in accounts.chunks(MAX_EXPORT_ACCOUNTS_PER_PACKAGE).enumerate() {
            let package_bytes = build_accounts_zip_bytes(chunk, &provider_type)?;
            let package_name = format!("oauth-accounts-part-{:04}.zip", index + 1);
            root_zip
                .start_file(package_name, root_file_options)
                .map_err(|e| Error::BusinessError(format!("zip start package failed: {}", e)))?;
            root_zip
                .write_all(&package_bytes)
                .map_err(|e| Error::BusinessError(format!("zip write package failed: {}", e)))?;
        }

        let root_cursor = root_zip
            .finish()
            .map_err(|e| Error::BusinessError(format!("zip finalize root failed: {}", e)))?;
        root_cursor.into_inner()
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"{}-oauth-accounts.zip\"",
            id
        ))
        .map_err(|e| Error::BusinessError(format!("invalid content-disposition: {}", e)))?,
    );

    Ok((headers, zip_bytes).into_response())
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/oauth/accounts/refresh-expiry",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "刷新成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn refresh_provider_oauth_accounts_expiry(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let provider = service.get_provider(&id).await?;
    let provider_type = provider.provider_type.trim().to_lowercase();
    if provider_type.is_empty() {
        return Err(Error::ValidationError(
            "provider_type cannot be empty".to_string(),
        ));
    }

    let mut accounts = provider_accounts_from_auth_config(&provider.auth_config, &provider_type);
    if accounts.is_empty() {
        return Err(Error::ValidationError(
            "no oauth accounts found for provider".to_string(),
        ));
    }

    let mut changed = false;
    for account in &mut accounts {
        if refresh_account_expiry(account) {
            changed = true;
        }
    }

    if changed {
        let status = service
            .import_provider_oauth_accounts(
                &id,
                ProviderOAuthAccountsImportRequest {
                    accounts: accounts.iter().map(build_export_item).collect(),
                },
            )
            .await?;
        return Ok(Json(ApiResponse::success(status)));
    }

    let status = service.get_provider_oauth_status(&id).await?;
    Ok(Json(ApiResponse::success(status)))
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/oauth/accounts/{account_key}/refresh-expiry",
    params(
        ("id" = String, Path, description = "供应商ID"),
        ("account_key" = String, Path, description = "账号key")
    ),
    responses(
        (status = 200, description = "刷新成功", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 404, description = "供应商或账号不存在", body = ApiResponse<ProviderOAuthStatusResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ProviderOAuthStatusResponse>)
    ),
    tag = "providers"
)]
pub async fn refresh_provider_oauth_account_expiry(
    State(ctx): State<Arc<ServiceContext>>,
    Path((id, account_key)): Path<(String, String)>,
) -> Result<Json<ApiResponse<ProviderOAuthStatusResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let provider = service.get_provider(&id).await?;
    let provider_type = provider.provider_type.trim().to_lowercase();
    if provider_type.is_empty() {
        return Err(Error::ValidationError(
            "provider_type cannot be empty".to_string(),
        ));
    }

    let mut accounts = provider_accounts_from_auth_config(&provider.auth_config, &provider_type);
    if accounts.is_empty() {
        return Err(Error::ValidationError(
            "no oauth accounts found for provider".to_string(),
        ));
    }

    let target_key = account_key.trim();
    if target_key.is_empty() {
        return Err(Error::ValidationError(
            "account_key cannot be empty".to_string(),
        ));
    }

    let target = accounts
        .iter_mut()
        .find(|a| a.account_key == target_key)
        .ok_or_else(|| Error::NotFound(format!("oauth account not found: {}", target_key)))?;

    let changed = refresh_account_expiry(target);
    if changed {
        let status = service
            .import_provider_oauth_accounts(
                &id,
                ProviderOAuthAccountsImportRequest {
                    accounts: accounts.iter().map(build_export_item).collect(),
                },
            )
            .await?;
        return Ok(Json(ApiResponse::success(status)));
    }

    let status = service.get_provider_oauth_status(&id).await?;
    Ok(Json(ApiResponse::success(status)))
}

pub async fn provider_oauth_public_callback(
    State(ctx): State<Arc<ServiceContext>>,
    axum::extract::Query(query): axum::extract::Query<ProviderOAuthPublicCallbackQuery>,
) -> Result<Redirect> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let redirect_url = service
        .complete_provider_oauth_public_callback(
            &query.state,
            query.code.as_deref(),
            query.error.as_deref(),
            None,
        )
        .await?;

    Ok(Redirect::temporary(&redirect_url))
}

pub async fn provider_oauth_public_callback_by_type(
    State(ctx): State<Arc<ServiceContext>>,
    Path(provider_type): Path<String>,
    axum::extract::Query(query): axum::extract::Query<ProviderOAuthPublicCallbackQuery>,
) -> Result<Redirect> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;

    let redirect_url = service
        .complete_provider_oauth_public_callback(
            &query.state,
            query.code.as_deref(),
            query.error.as_deref(),
            Some(&provider_type),
        )
        .await?;

    Ok(Redirect::temporary(&redirect_url))
}
