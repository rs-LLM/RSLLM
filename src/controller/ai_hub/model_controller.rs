//! 模型管理控制器模块
//! 提供模型管理的RESTful API接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::{collections::HashSet, time::Duration};

use serde::{Deserialize, Serialize};

use crate::context::ServiceContext;
use crate::domain::table::ai_hub::model_base::ModelBase;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::table::ai_hub::provider_config::ProviderConfig;
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};
use crate::service::ai_hub::ProviderConfigService;

fn resolve_page(page: Option<i64>) -> Result<u64> {
    let page = page.unwrap_or(1);
    if page < 1 {
        return Err(Error::ValidationError(format!(
            "page must be >= 1, got {}",
            page
        )));
    }

    u64::try_from(page).map_err(|_| Error::ValidationError("page is too large".to_string()))
}

fn resolve_page_size(page_size: Option<i64>) -> Result<u64> {
    let page_size = page_size.unwrap_or(10);
    if page_size < 1 {
        return Err(Error::ValidationError(format!(
            "page_size must be >= 1, got {}",
            page_size
        )));
    }

    u64::try_from(page_size)
        .map_err(|_| Error::ValidationError("page_size is too large".to_string()))
}

fn validate_required_text(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(Error::ValidationError(format!(
            "Missing required field: {}",
            field
        )));
    }

    Ok(())
}

const SUPPORTED_MODEL_TYPES: &[&str] = &["chat", "video", "embedding", "image", "audio", "rerank"];

fn normalize_model_type(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == "completion" {
        return "chat".to_string();
    }
    lowered
}

fn validate_model_type(model_type: &str) -> Result<String> {
    let normalized = normalize_model_type(model_type);
    if SUPPORTED_MODEL_TYPES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(Error::ValidationError(format!(
            "Unsupported model_type: {}. allowed: {}",
            model_type,
            SUPPORTED_MODEL_TYPES.join(", ")
        )))
    }
}

fn normalize_model_code(raw: &str) -> Option<String> {
    let code = raw.trim();
    if code.is_empty() {
        None
    } else {
        Some(code.to_string())
    }
}

fn parse_provider_model_codes(payload: &JsonValue) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    let mut push = |value: &str| {
        if let Some(code) = normalize_model_code(value)
            && seen.insert(code.clone())
        {
            result.push(code);
        }
    };

    if let Some(arr) = payload.get("data").and_then(JsonValue::as_array) {
        for item in arr {
            if let Some(code) = item.get("id").and_then(JsonValue::as_str) {
                push(code);
            } else if let Some(code) = item.get("key").and_then(JsonValue::as_str) {
                push(code);
            } else if let Some(code) = item.get("name").and_then(JsonValue::as_str) {
                push(code);
            }
        }
    }

    if let Some(arr) = payload.get("models").and_then(JsonValue::as_array) {
        for item in arr {
            if let Some(code) = item.as_str() {
                push(code);
            } else if let Some(code) = item.get("id").and_then(JsonValue::as_str) {
                push(code);
            } else if let Some(code) = item.get("name").and_then(JsonValue::as_str) {
                push(code);
            }
        }
    }

    if let Some(arr) = payload.as_array() {
        for item in arr {
            if let Some(code) = item.as_str() {
                push(code);
            } else if let Some(code) = item.get("id").and_then(JsonValue::as_str) {
                push(code);
            }
        }
    }

    result
}

fn normalize_model_scan_endpoints(api_base: &str) -> Vec<String> {
    let raw = api_base.trim();
    if raw.is_empty() {
        return Vec::new();
    }

    let trimmed = raw.trim_end_matches('/');
    let lowered = trimmed.to_ascii_lowercase();
    let is_openai_base = lowered.contains("api.openai.com");
    let base_without_models = trimmed.trim_end_matches("/models").trim_end_matches('/');

    let mut seen = HashSet::new();
    let mut candidates: Vec<String> = Vec::new();

    let push = |url: String, seen: &mut HashSet<String>, candidates: &mut Vec<String>| {
        if !url.trim().is_empty() && seen.insert(url.clone()) {
            candidates.push(url);
        }
    };

    if trimmed.ends_with("/models") && (!is_openai_base || trimmed.ends_with("/v1/models")) {
        push(trimmed.to_string(), &mut seen, &mut candidates);
    }

    if !base_without_models.is_empty() {
        if is_openai_base {
            if base_without_models.ends_with("/v1") {
                push(
                    format!("{base_without_models}/models"),
                    &mut seen,
                    &mut candidates,
                );
            } else {
                push(
                    format!("{base_without_models}/v1/models"),
                    &mut seen,
                    &mut candidates,
                );
            }

            return candidates;
        }

        push(
            format!("{base_without_models}/models"),
            &mut seen,
            &mut candidates,
        );

        if !base_without_models.ends_with("/v1") {
            push(
                format!("{base_without_models}/v1/models"),
                &mut seen,
                &mut candidates,
            );
        }

        if base_without_models.ends_with("/v1") {
            let root_base = base_without_models
                .trim_end_matches("/v1")
                .trim_end_matches('/');
            if !root_base.is_empty() {
                push(format!("{root_base}/models"), &mut seen, &mut candidates);
            }
        }
    }

    candidates
}

fn normalize_bearer_auth(raw_token: &str) -> String {
    let trimmed = raw_token.trim();
    if trimmed.len() >= 7 && trimmed[..7].eq_ignore_ascii_case("bearer ") {
        let token = trimmed[7..].trim();
        return format!("Bearer {}", token);
    }

    format!("Bearer {}", trimmed)
}

fn apply_authorization_header(
    req: reqwest::RequestBuilder,
    api_key: Option<&str>,
) -> Result<reqwest::RequestBuilder> {
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        let mut headers = HeaderMap::new();
        let auth = normalize_bearer_auth(key);
        let auth_value = HeaderValue::from_str(&auth)
            .map_err(|e| Error::BusinessError(format!("Invalid provider api key: {e}")))?;
        headers.insert(AUTHORIZATION, auth_value);
        return Ok(req.headers(headers));
    }

    Ok(req)
}

fn first_non_empty_json_string(value: &JsonValue, keys: &[&str]) -> Option<String> {
    match value {
        JsonValue::Object(map) => {
            for key in keys {
                if let Some(v) = map.get(*key).and_then(JsonValue::as_str) {
                    let token = v.trim();
                    if !token.is_empty() {
                        return Some(token.to_string());
                    }
                }
            }

            for child in map.values() {
                if let Some(found) = first_non_empty_json_string(child, keys) {
                    return Some(found);
                }
            }

            None
        }
        JsonValue::Array(arr) => {
            for item in arr {
                if let Some(found) = first_non_empty_json_string(item, keys) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn resolve_provider_scan_api_key(
    provider: &ProviderConfig,
    decrypted_api_key: Option<String>,
) -> Option<String> {
    if let Some(key) = decrypted_api_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    provider
        .auth_config
        .as_ref()
        .and_then(|cfg| first_non_empty_json_string(cfg, &["api_key", "access_token", "token"]))
}

fn provider_requires_strict_api_key(provider: &ProviderConfig) -> bool {
    let api_base = provider.api_base.to_lowercase();
    let provider_type = provider.provider_type.to_lowercase();
    api_base.contains("api.openai.com") || provider_type.contains("openai")
}

fn provider_uses_kimi_default_models(provider: &ProviderConfig) -> bool {
    let provider_type = provider.provider_type.trim().to_ascii_lowercase();
    let provider_code = provider.provider_code.trim().to_ascii_lowercase();
    provider_type == "kimi" || provider_code == "kimi"
}

fn kimi_default_model_codes() -> Vec<String> {
    [
        "kimi-k2",
        "kimi-k2-thinking",
        "kimi-k2.5",
        "kimi-k2-turbo-preview",
        "kimi-k2-thinking-turbo",
        "kimi-latest",
        "moonshot-v1-auto",
        "moonshot-v1-128k",
        "moonshot-v1-32k",
        "moonshot-v1-8k",
    ]
    .iter()
    .map(|item| item.to_string())
    .collect()
}

/// 列出所有模型
///
/// 支持分页和搜索过滤
#[utoipa::path(
    get,
    path = "/admin/models",
    params(
        ("page" = Option<i64>, Query, description = "页码"),
        ("page_size" = Option<i64>, Query, description = "每页数量"),
        ("search" = Option<String>, Query, description = "搜索关键词")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListModelsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ListModelsResponse>)
    ),
    tag = "models"
)]
pub async fn list_models(
    State(_ctx): State<Arc<ServiceContext>>,
    Query(params): Query<ListModelsParams>,
) -> Result<Json<ApiResponse<ListModelsResponse>>> {
    let rb = crate::pool!();

    let page = resolve_page(params.page)?;
    let page_size = resolve_page_size(params.page_size)?;
    let search = params.search.unwrap_or_default();

    let models = if search.is_empty() {
        ModelBase::select_page(rb, page, page_size)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    } else {
        ModelBase::search_page(rb, &search, page, page_size)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    };

    let total = if search.is_empty() {
        ModelBase::count_all(rb)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    } else {
        ModelBase::count_search(rb, &search)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    };

    let model_list: Vec<Model> = models
        .into_iter()
        .map(|model_base| Model {
            id: model_base.id.unwrap_or_default(),
            model_code: model_base.model_code,
            name: model_base.name,
            model_type: normalize_model_type(&model_base.model_type),
            input_price: model_base.input_price,
            output_price: model_base.output_price,
            currency: model_base.currency.unwrap_or_default(),
            max_tokens_per_request: model_base.max_tokens_per_request,
            max_requests_per_minute: model_base.max_requests_per_minute,
            description: model_base.description,
            model_category: model_base.model_category,
            documentation_md: model_base.documentation_md,
            documentation_options: model_base.documentation_options,
            capabilities: model_base.capabilities,
            status: model_base.status.unwrap_or_default(),
            created_at: model_base.created_at.unwrap_or_default(),
            updated_at: model_base.updated_at.unwrap_or_default(),
            provider: None,
        })
        .collect();

    let response = ListModelsResponse {
        items: model_list,
        total,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取模型详情
///
/// 根据ID获取模型详情
#[utoipa::path(
    get,
    path = "/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ModelBase>),
        (status = 404, description = "模型不存在", body = ApiResponse<ModelBase>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelBase>)
    ),
    tag = "models"
)]
pub async fn get_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ModelBase>>> {
    let rb = crate::pool!();

    let mut model = ModelBase::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop()
        .ok_or_else(|| Error::BusinessError("Model not found".to_string()))?;

    model.model_type = normalize_model_type(&model.model_type);

    Ok(Json(ApiResponse::success(model)))
}

/// 创建模型
///
/// 创建新的模型配置
#[utoipa::path(
    post,
    path = "/admin/models",
    request_body = CreateModelRequest,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<ModelBase>),
        (status = 400, description = "参数错误", body = ApiResponse<ModelBase>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelBase>)
    ),
    tag = "models"
)]
pub async fn create_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<ApiResponse<ModelBase>>> {
    let rb = crate::pool!();
    validate_required_text("model_code", &req.model_code)?;
    let model_code = req.model_code.trim().to_string();
    let model_type = validate_model_type(&req.model_type)?;

    let model = ModelBase {
        id: Some(ulid::Ulid::new().to_string()),
        model_code,
        name: req.name,
        model_type,
        input_price: req.input_price,
        output_price: req.output_price,
        price_unit: req.price_unit,
        currency: req.currency,
        max_tokens_per_request: req.max_tokens_per_request,
        max_requests_per_minute: req.max_requests_per_minute,
        description: req.description,
        model_category: req.model_category,
        documentation_md: req.documentation_md,
        documentation_options: req.documentation_options,
        capabilities: req.capabilities,
        status: Some("active".to_string()),
        image_token_calculation_type: req.image_token_calculation_type,
        patch_multiplier: req.patch_multiplier,
        tile_base_tokens: req.tile_base_tokens,
        tile_tokens_per_tile: req.tile_tokens_per_tile,
        audio_tokens_per_second: req.audio_tokens_per_second,
        created_at: Some(rbatis::rbdc::DateTime::now().to_string()),
        updated_at: Some(rbatis::rbdc::DateTime::now().to_string()),
    };

    ModelBase::insert(rb, &model)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success(model)))
}

/// 更新模型
///
/// 更新模型配置
#[utoipa::path(
    put,
    path = "/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    request_body = UpdateModelRequest,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<ModelBase>),
        (status = 404, description = "模型不存在", body = ApiResponse<ModelBase>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelBase>)
    ),
    tag = "models"
)]
pub async fn update_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<ApiResponse<ModelBase>>> {
    let rb = crate::pool!();

    let existing = ModelBase::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop()
        .ok_or_else(|| Error::BusinessError("Model not found".to_string()))?;

    let mut model = existing;
    model.model_type = normalize_model_type(&model.model_type);
    if let Some(name) = req.name {
        model.name = name;
    }
    if let Some(model_type) = req.model_type {
        model.model_type = validate_model_type(&model_type)?;
    }
    if let Some(input_price) = req.input_price {
        model.input_price = input_price;
    }
    if let Some(output_price) = req.output_price {
        model.output_price = output_price;
    }
    if let Some(price_unit) = req.price_unit {
        model.price_unit = Some(price_unit);
    }
    if let Some(currency) = req.currency {
        model.currency = Some(currency);
    }
    if let Some(max_tokens_per_request) = req.max_tokens_per_request {
        model.max_tokens_per_request = Some(max_tokens_per_request);
    }
    if let Some(max_requests_per_minute) = req.max_requests_per_minute {
        model.max_requests_per_minute = Some(max_requests_per_minute);
    }
    if let Some(description) = req.description {
        model.description = Some(description);
    }
    if let Some(model_category) = req.model_category {
        model.model_category = Some(model_category);
    }
    if let Some(documentation_md) = req.documentation_md {
        model.documentation_md = Some(documentation_md);
    }
    if let Some(documentation_options) = req.documentation_options {
        model.documentation_options = Some(documentation_options);
    }
    if let Some(capabilities) = req.capabilities {
        model.capabilities = Some(capabilities);
    }
    if let Some(status) = req.status {
        model.status = Some(status);
    }
    if let Some(image_token_calculation_type) = req.image_token_calculation_type {
        model.image_token_calculation_type = Some(image_token_calculation_type);
    }
    if let Some(patch_multiplier) = req.patch_multiplier {
        model.patch_multiplier = Some(patch_multiplier);
    }
    if let Some(tile_base_tokens) = req.tile_base_tokens {
        model.tile_base_tokens = Some(tile_base_tokens);
    }
    if let Some(tile_tokens_per_tile) = req.tile_tokens_per_tile {
        model.tile_tokens_per_tile = Some(tile_tokens_per_tile);
    }
    if let Some(audio_tokens_per_second) = req.audio_tokens_per_second {
        model.audio_tokens_per_second = Some(audio_tokens_per_second);
    }

    ModelBase::update_by_map(rb, &model, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success(model)))
}

/// 删除模型
///
/// 删除模型配置
#[utoipa::path(
    delete,
    path = "/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
        (status = 404, description = "模型不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器错误", body = ApiResponse<String>)
    ),
    tag = "models"
)]
pub async fn delete_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    let rb = crate::pool!();

    // 先删除对应的供应商模型关系
    ModelProviderMapping::delete_by_map(rb, rbs::value! { "model_id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    // 再删除模型本身
    ModelBase::delete_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

fn parse_model_code(raw: &str) -> Option<String> {
    let model_code = raw.trim();
    if model_code.is_empty() {
        return None;
    }
    Some(model_code.to_string())
}

fn build_provider_model_scan_urls(api_base: &str) -> Vec<String> {
    normalize_model_scan_endpoints(api_base)
}

async fn scan_models_from_provider(
    provider: &ProviderConfig,
    api_key: Option<&str>,
) -> Result<Vec<String>> {
    let scan_urls = build_provider_model_scan_urls(&provider.api_base);
    if scan_urls.is_empty() {
        return Err(Error::ValidationError(
            "Provider api_base cannot be empty".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| Error::BusinessError(format!("Failed to initialize HTTP client: {}", e)))?;

    let mut failure_reasons = Vec::new();
    for url in scan_urls {
        let req = apply_authorization_header(client.get(&url), api_key)?;
        let response = req.send().await;
        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                failure_reasons.push(format!("{} -> request failed: {}", url, e));
                continue;
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let body_lower = body.to_ascii_lowercase();
            if status == reqwest::StatusCode::UNAUTHORIZED
                && body_lower.contains("missing bearer authentication")
            {
                failure_reasons.push(format!(
                    "{} -> status={} body={} (hint: missing API key/Bearer token in provider config)",
                    url, status, body
                ));
                continue;
            }
            if status == reqwest::StatusCode::FORBIDDEN
                && body_lower.contains("api.model.read")
                && provider_requires_strict_api_key(provider)
            {
                failure_reasons.push(format!(
                    "{} -> status={} body={} (hint: current OpenAI project/key lacks model-read permission `api.model.read`; grant the required project role/scope or import models manually)",
                    url, status, body
                ));
                continue;
            }
            failure_reasons.push(format!("{} -> status={} body={}", url, status, body));
            continue;
        }

        let payload: JsonValue = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                failure_reasons.push(format!("{} -> response parse failed: {}", url, e));
                continue;
            }
        };

        let model_codes = parse_provider_model_codes(&payload);
        if !model_codes.is_empty() {
            return Ok(model_codes);
        }

        failure_reasons.push(format!("{} -> no model ids found in response", url));
    }

    Err(Error::BusinessError(format!(
        "Failed to scan models from provider {}: {}",
        provider.name,
        failure_reasons.join(" | ")
    )))
}

async fn resolve_provider_and_scan_context(
    ctx: &Arc<ServiceContext>,
    id: String,
) -> Result<(ProviderConfig, String, Vec<String>, Option<String>)> {
    let rb = crate::pool!();

    let provider = ProviderConfig::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop()
        .ok_or_else(|| Error::NotFound(format!("Provider with id '{}' not found", id)))?;

    let provider_id = provider.id.clone().unwrap_or(id);
    let decrypted_api_key = if let Some(encrypted_key) = provider
        .api_key_encrypted
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let service_guard = ctx.provider_config_service.read().await;
        let service = service_guard.as_ref().ok_or_else(|| {
            Error::BusinessError("Provider config service not initialized".to_string())
        })?;
        Some(service.get_decrypted_api_key(&provider_id, encrypted_key)?)
    } else {
        None
    };

    let api_key = resolve_provider_scan_api_key(&provider, decrypted_api_key);
    if api_key.is_none() && provider_requires_strict_api_key(&provider) {
        return Err(Error::ValidationError(format!(
            "Provider {} is missing API key. Please configure API Key before scanning models",
            provider.name
        )));
    }
    let (scanned_model_codes, warning) =
        match scan_models_from_provider(&provider, api_key.as_deref()).await {
            Ok(model_codes) => (model_codes, None),
            Err(scan_error) if provider_uses_kimi_default_models(&provider) => {
                let fallback_codes = kimi_default_model_codes();
                if fallback_codes.is_empty() {
                    return Err(scan_error);
                }

                let warning = format!(
                    "Kimi upstream model scan failed ({}); fallback to built-in model set.",
                    scan_error
                );
                (fallback_codes, Some(warning))
            }
            Err(scan_error) => return Err(scan_error),
        };

    Ok((provider, provider_id, scanned_model_codes, warning))
}

fn dedupe_selected_model_codes(scanned_model_codes: &[String], selected: &[String]) -> Vec<String> {
    let scanned_set: HashSet<&str> = scanned_model_codes.iter().map(|s| s.as_str()).collect();
    let mut selected_set = HashSet::new();
    let mut result = Vec::new();

    for raw in selected {
        if let Some(code) = parse_model_code(raw)
            && scanned_set.contains(code.as_str())
            && selected_set.insert(code.clone())
        {
            result.push(code);
        }
    }

    result
}

fn normalize_requested_model_codes(selected: &[String]) -> Vec<String> {
    let mut selected_set = HashSet::new();
    let mut result = Vec::new();

    for raw in selected {
        if let Some(code) = parse_model_code(raw)
            && selected_set.insert(code.clone())
        {
            result.push(code);
        }
    }

    result
}

async fn import_model_codes_for_provider(
    provider: &ProviderConfig,
    provider_id: &str,
    model_codes: &[String],
) -> Result<(usize, usize, usize, usize)> {
    let rb = crate::pool!();

    let mut created_model_count = 0usize;
    let mut created_mapping_count = 0usize;
    let mut updated_mapping_count = 0usize;
    let mut skipped_count = 0usize;

    let now = rbatis::rbdc::DateTime::now().to_string();
    for model_code in model_codes {
        let model = match ModelBase::select_by_model_code_or_name_case_insensitive(rb, model_code)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
        {
            Some(existing) => existing,
            None => {
                let new_model = ModelBase {
                    id: Some(ulid::Ulid::new().to_string()),
                    model_code: model_code.clone(),
                    name: model_code.clone(),
                    model_type: "chat".to_string(),
                    input_price: 0.0,
                    output_price: 0.0,
                    price_unit: None,
                    currency: Some("USD".to_string()),
                    max_tokens_per_request: None,
                    max_requests_per_minute: None,
                    description: Some(format!("Imported from provider {}", provider.name)),
                    model_category: None,
                    documentation_md: None,
                    documentation_options: None,
                    capabilities: None,
                    status: Some("active".to_string()),
                    image_token_calculation_type: None,
                    patch_multiplier: None,
                    tile_base_tokens: None,
                    tile_tokens_per_tile: None,
                    audio_tokens_per_second: None,
                    created_at: Some(now.clone()),
                    updated_at: Some(now.clone()),
                };

                ModelBase::insert(rb, &new_model)
                    .await
                    .map_err(|e| Error::DatabaseError(e.to_string()))?;
                created_model_count += 1;
                new_model
            }
        };

        let model_id = model.id.clone().unwrap_or_default();
        if model_id.is_empty() {
            skipped_count += 1;
            continue;
        }

        let existing_mapping =
            ModelProviderMapping::select_by_model_and_provider(rb, &model_id, provider_id)
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

        if let Some(mut mapping) = existing_mapping {
            let mut changed = false;

            if mapping.provider_model_name != *model_code {
                mapping.provider_model_name = model_code.clone();
                changed = true;
            }

            if mapping.api_endpoint.as_deref() != Some(provider.api_base.as_str()) {
                mapping.api_endpoint = Some(provider.api_base.clone());
                changed = true;
            }

            if mapping.status.as_deref() != Some("active") {
                mapping.status = Some("active".to_string());
                changed = true;
            }

            if changed {
                mapping.updated_at = Some(now.clone());
                if let Some(mapping_id) = mapping.id.clone() {
                    ModelProviderMapping::update_by_map(
                        rb,
                        &mapping,
                        rbs::value! { "id": mapping_id },
                    )
                    .await
                    .map_err(|e| Error::DatabaseError(e.to_string()))?;
                    updated_mapping_count += 1;
                } else {
                    skipped_count += 1;
                }
            } else {
                skipped_count += 1;
            }
        } else {
            let mapping = ModelProviderMapping {
                id: Some(ulid::Ulid::new().to_string()),
                model_id,
                provider_id: provider_id.to_string(),
                provider_model_name: model_code.clone(),
                api_endpoint: Some(provider.api_base.clone()),
                api_key_encrypted: None,
                priority: Some(10),
                status: Some("active".to_string()),
                created_at: Some(now.clone()),
                updated_at: Some(now.clone()),
            };

            ModelProviderMapping::insert(rb, &mapping)
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;
            created_mapping_count += 1;
        }
    }

    Ok((
        created_model_count,
        created_mapping_count,
        updated_mapping_count,
        skipped_count,
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScanProviderModelsResponse {
    pub provider_id: String,
    pub provider_name: String,
    pub model_codes: Vec<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ImportProviderModelsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_codes: Option<Vec<String>>,
}

/// 从供应商扫描并导入模型结果
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImportProviderModelsResponse {
    pub provider_id: String,
    pub provider_name: String,
    pub scanned_count: usize,
    pub created_model_count: usize,
    pub created_mapping_count: usize,
    pub updated_mapping_count: usize,
    pub skipped_count: usize,
    pub model_codes: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/admin/providers/{id}/models/scan",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "扫描成功", body = ApiResponse<ScanProviderModelsResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ScanProviderModelsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ScanProviderModelsResponse>)
    ),
    tag = "models"
)]
pub async fn scan_provider_models(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ScanProviderModelsResponse>>> {
    let fallback_provider = {
        let rb = crate::pool!();
        ProviderConfig::select_by_map(rb, rbs::value! { "id": &id })
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(format!("Provider {} not found", id)))?
    };

    let resolved = resolve_provider_and_scan_context(&ctx, id).await;
    let (provider, provider_id, scanned_model_codes, warning) = match resolved {
        Ok((provider, provider_id, scanned_model_codes, warning)) => {
            (provider, provider_id, scanned_model_codes, warning)
        }
        Err(Error::BusinessError(msg)) if msg.to_lowercase().contains("api.model.read") => (
            fallback_provider.clone(),
            fallback_provider
                .id
                .clone()
                .unwrap_or_else(|| fallback_provider.provider_code.clone()),
            Vec::new(),
            Some("Missing scope `api.model.read`; switched to manual import mode.".to_string()),
        ),
        Err(e) => return Err(e),
    };

    Ok(Json(ApiResponse::success(ScanProviderModelsResponse {
        provider_id,
        provider_name: provider.name,
        model_codes: scanned_model_codes,
        warning,
    })))
}

/// 从供应商扫描并导入模型
#[utoipa::path(
    post,
    path = "/admin/providers/{id}/models/import",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "导入成功", body = ApiResponse<ImportProviderModelsResponse>),
        (status = 404, description = "供应商不存在", body = ApiResponse<ImportProviderModelsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ImportProviderModelsResponse>)
    ),
    tag = "models"
)]
pub async fn import_provider_models(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    payload: Option<Json<ImportProviderModelsRequest>>,
) -> Result<Json<ApiResponse<ImportProviderModelsResponse>>> {
    let requested_codes = payload
        .as_ref()
        .and_then(|body| body.model_codes.clone())
        .unwrap_or_default();

    let (provider, provider_id, scanned_model_codes, target_model_codes) =
        if requested_codes.is_empty() {
            let (provider, provider_id, scanned_model_codes, _warning) =
                resolve_provider_and_scan_context(&ctx, id).await?;
            let target_model_codes = scanned_model_codes.clone();
            (
                provider,
                provider_id,
                scanned_model_codes,
                target_model_codes,
            )
        } else {
            let rb = crate::pool!();
            let provider = ProviderConfig::select_by_map(rb, rbs::value! { "id": &id })
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?
                .into_iter()
                .next()
                .ok_or_else(|| Error::NotFound(format!("Provider {} not found", id)))?;

            let provider_id = provider.id.clone().unwrap_or_else(|| id.to_string());
            let target_model_codes = normalize_requested_model_codes(&requested_codes);
            let scanned_model_codes =
                dedupe_selected_model_codes(&target_model_codes, &requested_codes);
            (
                provider,
                provider_id,
                scanned_model_codes,
                target_model_codes,
            )
        };

    if target_model_codes.is_empty() {
        return Err(Error::ValidationError(
            "No valid selected model codes to import".to_string(),
        ));
    }

    let (created_model_count, created_mapping_count, updated_mapping_count, skipped_count) =
        import_model_codes_for_provider(&provider, &provider_id, &target_model_codes).await?;

    Ok(Json(ApiResponse::success(ImportProviderModelsResponse {
        provider_id,
        provider_name: provider.name,
        scanned_count: scanned_model_codes.len(),
        created_model_count,
        created_mapping_count,
        updated_mapping_count,
        skipped_count,
        model_codes: target_model_codes,
    })))
}

/// 列出模型参数
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ListModelsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// 列出模型响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ListModelsResponse {
    pub items: Vec<Model>,
    pub total: i64,
}

/// 模型信息（适配前端）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Model {
    pub id: String,
    pub model_code: String,
    pub name: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_md: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderInfo>,
}

/// 供应商信息（适配前端）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderInfo {
    pub id: String,
    pub provider_code: String,
    pub provider_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// 创建模型请求
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateModelRequest {
    pub model_code: String,
    pub name: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_md: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_token_calculation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_multiplier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_base_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_tokens_per_tile: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens_per_second: Option<f64>,
}

/// 更新模型请求
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct UpdateModelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_md: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_token_calculation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_multiplier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_base_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_tokens_per_tile: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens_per_second: Option<f64>,
}

/// OpenAI 兼容的模型列表响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenAIModelsListResponse {
    pub object: String,
    pub data: Vec<OpenAIModelInfo>,
    pub providers: Vec<OpenAIProviderInfo>,
}

/// OpenAI 兼容的模型信息
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenAIModelInfo {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
    pub key: String,
}

/// OpenAI 兼容的供应商信息
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenAIProviderInfo {
    pub id: String,
    pub provider_code: String,
    pub name: String,
    pub provider_type: String,
}

/// 公开模型列表响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicModelsListResponse {
    pub object: String,
    pub data: Vec<PublicModelInfo>,
    pub total: i64,
}

/// 公开模型信息
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_code: String,
    pub model_code: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_md: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
}

/// OpenAI 兼容的模型列表
///
/// 返回所有可用模型的 OpenAI 兼容格式列表
#[utoipa::path(
    get,
    path = "/api/v1/models",
    responses(
        (status = 200, description = "查询成功", body = OpenAIModelsListResponse),
        (status = 500, description = "服务器错误", body = OpenAIModelsListResponse)
    ),
    tag = "models"
)]
pub async fn list_openai_models(
    State(ctx): State<Arc<ServiceContext>>,
) -> Result<Json<OpenAIModelsListResponse>> {
    let model_router = ctx.model_router.clone();

    let models = model_router.list_all_models().await?;

    let mut provider_map = std::collections::HashMap::new();
    let mut openai_models = Vec::new();

    for model_info in models {
        let provider_id = model_info.provider.id.clone().unwrap_or_default();
        let provider_code = model_info.provider.provider_code.clone();
        let model_code = model_info.model_base.model_code.clone();

        provider_map
            .entry(provider_id.clone())
            .or_insert_with(|| OpenAIProviderInfo {
                id: provider_id.clone(),
                provider_code: provider_code.clone(),
                name: model_info.provider.name.clone(),
                provider_type: model_info.provider.provider_type.clone(),
            });

        openai_models.push(OpenAIModelInfo {
            id: format!("{}/{}", provider_code, model_code),
            object: "model".to_string(),
            created: model_info
                .model_base
                .created_at
                .as_ref()
                .and_then(|s| s.parse::<i64>().ok())
                .map(|ts| ts / 1000)
                .unwrap_or(0),
            owned_by: provider_code.clone(),
            key: model_code,
        });
    }

    let providers: Vec<OpenAIProviderInfo> = provider_map.into_values().collect();

    Ok(Json(OpenAIModelsListResponse {
        object: "list".to_string(),
        data: openai_models,
        providers,
    }))
}

/// 公开模型列表
///
/// 返回所有可用模型的公开列表，支持分页和搜索
#[utoipa::path(
    get,
    path = "/rsllm/public/get_models",
    params(
        ("page" = Option<i64>, Query, description = "页码"),
        ("page_size" = Option<i64>, Query, description = "每页数量"),
        ("search" = Option<String>, Query, description = "搜索关键词")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PublicModelsListResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<PublicModelsListResponse>)
    ),
    tag = "models"
)]
pub async fn list_public_models(
    State(ctx): State<Arc<ServiceContext>>,
    Query(params): Query<ListModelsParams>,
) -> Result<Json<ApiResponse<PublicModelsListResponse>>> {
    let model_router = ctx.model_router.clone();

    let page = i64::try_from(resolve_page(params.page)?)
        .map_err(|_| Error::ValidationError("page is too large".to_string()))?;
    let page_size = i64::try_from(resolve_page_size(params.page_size)?)
        .map_err(|_| Error::ValidationError("page_size is too large".to_string()))?;
    let search = params.search.unwrap_or_default();

    let models = if search.is_empty() {
        model_router.list_all_models_page(page, page_size).await?
    } else {
        model_router
            .search_models_page(&search, page, page_size)
            .await?
    };

    let public_models: Vec<PublicModelInfo> = models
        .into_iter()
        .map(|model_info| {
            let currency = model_info
                .model_base
                .currency
                .clone()
                .unwrap_or_else(|| "USD".to_string());
            PublicModelInfo {
                id: format!(
                    "{}/{}",
                    model_info.provider.provider_code, model_info.model_base.model_code
                ),
                name: model_info.model_base.name.clone(),
                provider: model_info.provider.name.clone(),
                provider_code: model_info.provider.provider_code.clone(),
                model_code: model_info.model_base.model_code.clone(),
                model_type: normalize_model_type(&model_info.model_base.model_type),
                input_price: model_info.model_base.input_price,
                output_price: model_info.model_base.output_price,
                price_unit: model_info.model_base.price_unit.clone(),
                currency,
                max_tokens_per_request: model_info.model_base.max_tokens_per_request,
                max_requests_per_minute: model_info.model_base.max_requests_per_minute,
                description: model_info.model_base.description,
                model_category: model_info.model_base.model_category,
                documentation_md: model_info.model_base.documentation_md,
                documentation_options: model_info.model_base.documentation_options,
                capabilities: model_info.model_base.capabilities,
            }
        })
        .collect();

    let total = public_models.len() as i64;

    Ok(Json(ApiResponse::success(PublicModelsListResponse {
        object: "list".to_string(),
        data: public_models,
        total,
    })))
}
