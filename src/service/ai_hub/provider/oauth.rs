use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tokio::sync::RwLock;

pub const PROVIDER_OAUTH_SESSION_EXPIRE_MINUTES: i64 = 10;
const METRICS_WINDOW_SECONDS: i64 = 60;
const OAUTH_UNAUTHORIZED_COOLDOWN_SECONDS: i64 = 300;

#[derive(Debug, Clone)]
pub struct ProviderOAuthSession {
    pub provider_id: String,
    pub provider_type: String,
    pub code_verifier: String,
    pub flow_type: String,
    pub device_code: String,
    pub device_id: String,
    pub redirect_uri: String,
    pub frontend_redirect_uri: String,
    pub poll_interval_seconds: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ProviderOAuthAccount {
    pub account_key: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub id_token: String,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub access_token_expires_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthAccountSummary {
    pub account_key: String,
    pub enabled: bool,
    pub weight: u32,
    pub needs_reauth: bool,
    pub total_requests: u64,
    pub success_requests: u64,
    pub failed_requests: u64,
    pub qps: f64,
    pub error_rate: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_minute_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_minute_output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_minute_total_tokens: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthAccountSetting {
    pub account_key: String,
    pub enabled: bool,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthAccountsUpdateRequest {
    pub accounts: Vec<ProviderOAuthAccountSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthAccountImportItem {
    #[serde(default)]
    pub account_key: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub id_token: String,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub access_token_expires_at: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthAccountsImportRequest {
    pub accounts: Vec<ProviderOAuthAccountImportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ProviderOAuthDisconnectRequest {
    #[serde(default)]
    pub account_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SelectedProviderOAuthToken {
    pub access_token: String,
    pub token_type: String,
    pub account_key: String,
    pub account_id: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderOAuthAccountMetricsSnapshot {
    pub total_requests: u64,
    pub success_requests: u64,
    pub failed_requests: u64,
    pub qps: f64,
    pub error_rate: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub last_minute_input_tokens: u64,
    pub last_minute_output_tokens: u64,
    pub last_minute_total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderRequestPoint {
    ts: i64,
    ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderTokenPoint {
    ts: i64,
    input_tokens: i64,
    output_tokens: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProviderAccountMetricsState {
    #[serde(default)]
    total_requests: u64,
    #[serde(default)]
    success_requests: u64,
    #[serde(default)]
    failed_requests: u64,
    #[serde(default)]
    recent: VecDeque<ProviderRequestPoint>,
    #[serde(default)]
    total_input_tokens: u64,
    #[serde(default)]
    total_output_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
    #[serde(default)]
    recent_tokens: VecDeque<ProviderTokenPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthStartRequest {
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthStartResponse {
    pub provider_type: String,
    pub authorize_url: String,
    pub state: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_uri_complete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthDevicePollRequest {
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthDevicePollResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_status: Option<ProviderOAuthStatusResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOAuthDeviceCodeData {
    pub device_code: String,
    #[serde(default)]
    pub user_code: String,
    #[serde(default)]
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: String,
    #[serde(default)]
    pub expires_in: i64,
    #[serde(default)]
    pub interval: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthCallbackRequest {
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderOAuthStatusResponse {
    pub provider_type: String,
    pub connected: bool,
    pub needs_reauth: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_account_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accounts: Option<Vec<ProviderOAuthAccountSummary>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOAuthTokenData {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub id_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: i64,
}

static PROVIDER_OAUTH_SESSIONS: OnceLock<RwLock<HashMap<String, ProviderOAuthSession>>> =
    OnceLock::new();

static PROVIDER_OAUTH_RR_COUNTERS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
static PROVIDER_OAUTH_ACCOUNT_METRICS: OnceLock<
    Mutex<HashMap<String, HashMap<String, ProviderAccountMetricsState>>>,
> = OnceLock::new();
static PROVIDER_OAUTH_ACCOUNT_COOLDOWNS: OnceLock<Mutex<HashMap<String, HashMap<String, i64>>>> =
    OnceLock::new();

pub fn provider_oauth_sessions() -> &'static RwLock<HashMap<String, ProviderOAuthSession>> {
    PROVIDER_OAUTH_SESSIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_weight() -> u32 {
    1
}

fn rr_counters() -> &'static Mutex<HashMap<String, u64>> {
    PROVIDER_OAUTH_RR_COUNTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn metrics_store() -> &'static Mutex<HashMap<String, HashMap<String, ProviderAccountMetricsState>>>
{
    PROVIDER_OAUTH_ACCOUNT_METRICS.get_or_init(|| Mutex::new(load_metrics_from_disk()))
}

fn metrics_store_file_path() -> PathBuf {
    if let Ok(p) = std::env::var("RSLLM_OAUTH_METRICS_FILE") {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    PathBuf::from("./oauth_account_metrics.json")
}

fn load_metrics_from_path(
    path: &PathBuf,
) -> HashMap<String, HashMap<String, ProviderAccountMetricsState>> {
    let Ok(content) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    serde_json::from_str(&content).unwrap_or_default()
}

fn load_metrics_from_disk() -> HashMap<String, HashMap<String, ProviderAccountMetricsState>> {
    let path = metrics_store_file_path();
    load_metrics_from_path(&path)
}

fn persist_metrics_to_path(
    path: &PathBuf,
    store: &HashMap<String, HashMap<String, ProviderAccountMetricsState>>,
) {
    if let Ok(serialized) = serde_json::to_string(store) {
        let _ = fs::write(path, serialized);
    }
}

fn persist_metrics_to_disk(store: &HashMap<String, HashMap<String, ProviderAccountMetricsState>>) {
    let path = metrics_store_file_path();
    persist_metrics_to_path(&path, store);
}

fn cooldown_store() -> &'static Mutex<HashMap<String, HashMap<String, i64>>> {
    PROVIDER_OAUTH_ACCOUNT_COOLDOWNS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn provider_scope_key(provider_id: &str, provider_type: &str) -> String {
    format!(
        "{}:{}",
        provider_id.trim(),
        provider_type.trim().to_lowercase()
    )
}

pub fn provider_oauth_key(provider_type: &str) -> String {
    format!(
        "{}_oauth",
        provider_type.trim().to_lowercase().replace('-', "_")
    )
}

pub fn provider_oauth_accounts_key(provider_type: &str) -> String {
    format!(
        "{}_oauth_accounts",
        provider_type.trim().to_lowercase().replace('-', "_")
    )
}

fn parse_expires_ts(s: &str) -> Option<i64> {
    if s.trim().is_empty() {
        return None;
    }

    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

pub fn provider_account_needs_reauth(account: &ProviderOAuthAccount) -> bool {
    if account.access_token.trim().is_empty() {
        return true;
    }

    parse_expires_ts(&account.access_token_expires_at)
        .map(|ts| ts <= Utc::now().timestamp())
        .unwrap_or(false)
}

pub fn provider_accounts_from_auth_config(
    auth_config: &Option<JsonValue>,
    provider_type: &str,
) -> Vec<ProviderOAuthAccount> {
    let Some(auth_config) = auth_config else {
        return Vec::new();
    };

    let accounts_key = provider_oauth_accounts_key(provider_type);
    let mut accounts = auth_config
        .get(&accounts_key)
        .and_then(|v| serde_json::from_value::<Vec<ProviderOAuthAccount>>(v.clone()).ok())
        .unwrap_or_default();

    if accounts.is_empty() {
        let legacy_key = provider_oauth_key(provider_type);
        if let Some(legacy) = auth_config.get(&legacy_key)
            && let Ok(mut account) = serde_json::from_value::<ProviderOAuthAccount>(legacy.clone())
        {
            if account.account_key.trim().is_empty() {
                account.account_key = "legacy".to_string();
            }
            if account.weight == 0 {
                account.weight = 1;
            }
            accounts.push(account);
        }
    }

    for account in &mut accounts {
        if account.token_type.trim().is_empty() {
            account.token_type = "Bearer".to_string();
        }
        if account.weight == 0 {
            account.weight = 1;
        }
    }

    accounts
}

fn cleanup_recent_points(recent: &mut VecDeque<ProviderRequestPoint>, now_ts: i64) {
    while let Some(front) = recent.front() {
        if now_ts - front.ts > METRICS_WINDOW_SECONDS {
            recent.pop_front();
        } else {
            break;
        }
    }
}

fn cleanup_recent_token_points(recent: &mut VecDeque<ProviderTokenPoint>, now_ts: i64) {
    while let Some(front) = recent.front() {
        if now_ts - front.ts > METRICS_WINDOW_SECONDS {
            recent.pop_front();
        } else {
            break;
        }
    }
}

fn sum_recent_token_points(recent: &VecDeque<ProviderTokenPoint>) -> (u64, u64, u64) {
    let mut input: u64 = 0;
    let mut output: u64 = 0;
    for p in recent {
        if p.input_tokens > 0 {
            input = input.saturating_add(p.input_tokens as u64);
        }
        if p.output_tokens > 0 {
            output = output.saturating_add(p.output_tokens as u64);
        }
    }
    let total = input.saturating_add(output);
    (input, output, total)
}

pub fn record_provider_token_usage(
    provider_id: &str,
    provider_type: &str,
    account_key: &str,
    input_tokens: i64,
    output_tokens: i64,
) {
    if provider_id.trim().is_empty() || account_key.trim().is_empty() {
        return;
    }

    let scope_key = provider_scope_key(provider_id, provider_type);
    let now_ts = Utc::now().timestamp();
    if let Ok(mut guard) = metrics_store().lock() {
        let provider_metrics = guard.entry(scope_key).or_default();
        let account_metrics = provider_metrics.entry(account_key.to_string()).or_default();

        if input_tokens > 0 {
            account_metrics.total_input_tokens = account_metrics
                .total_input_tokens
                .saturating_add(input_tokens as u64);
        }
        if output_tokens > 0 {
            account_metrics.total_output_tokens = account_metrics
                .total_output_tokens
                .saturating_add(output_tokens as u64);
        }
        account_metrics.total_tokens = account_metrics
            .total_input_tokens
            .saturating_add(account_metrics.total_output_tokens);

        account_metrics.recent_tokens.push_back(ProviderTokenPoint {
            ts: now_ts,
            input_tokens,
            output_tokens,
        });
        cleanup_recent_token_points(&mut account_metrics.recent_tokens, now_ts);

        persist_metrics_to_disk(&guard);
    }
}

fn is_account_in_cooldown(scope_key: &str, account_key: &str, now_ts: i64) -> bool {
    if let Ok(mut guard) = cooldown_store().lock()
        && let Some(provider_map) = guard.get_mut(scope_key)
    {
        provider_map.retain(|_, until_ts| *until_ts > now_ts);
        if let Some(until_ts) = provider_map.get(account_key) {
            return *until_ts > now_ts;
        }
    }
    false
}

pub fn mark_provider_account_unauthorized(
    provider_id: &str,
    provider_type: &str,
    account_key: &str,
) {
    if provider_id.trim().is_empty() || account_key.trim().is_empty() {
        return;
    }

    let scope_key = provider_scope_key(provider_id, provider_type);
    let until_ts = Utc::now().timestamp() + OAUTH_UNAUTHORIZED_COOLDOWN_SECONDS;
    if let Ok(mut guard) = cooldown_store().lock() {
        let provider_map = guard.entry(scope_key).or_default();
        provider_map.insert(account_key.to_string(), until_ts);
    }
}

pub fn record_provider_request_result(
    provider_id: &str,
    provider_type: &str,
    account_key: &str,
    ok: bool,
) {
    if provider_id.trim().is_empty() || account_key.trim().is_empty() {
        return;
    }

    let scope_key = provider_scope_key(provider_id, provider_type);
    let now_ts = Utc::now().timestamp();
    if let Ok(mut guard) = metrics_store().lock() {
        let provider_metrics = guard.entry(scope_key).or_default();
        let account_metrics = provider_metrics.entry(account_key.to_string()).or_default();

        account_metrics.total_requests = account_metrics.total_requests.saturating_add(1);
        if ok {
            account_metrics.success_requests = account_metrics.success_requests.saturating_add(1);
        } else {
            account_metrics.failed_requests = account_metrics.failed_requests.saturating_add(1);
        }

        account_metrics
            .recent
            .push_back(ProviderRequestPoint { ts: now_ts, ok });
        cleanup_recent_points(&mut account_metrics.recent, now_ts);
        cleanup_recent_token_points(&mut account_metrics.recent_tokens, now_ts);
        persist_metrics_to_disk(&guard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_metrics_persist_roundtrip() {
        let path = std::env::temp_dir().join(format!(
            "rsllm-oauth-metrics-{}.json",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));

        let mut store: HashMap<String, HashMap<String, ProviderAccountMetricsState>> =
            HashMap::new();
        let mut account = ProviderAccountMetricsState {
            total_requests: 5,
            success_requests: 4,
            failed_requests: 1,
            recent: VecDeque::new(),
            total_input_tokens: 11,
            total_output_tokens: 7,
            total_tokens: 18,
            recent_tokens: VecDeque::new(),
        };
        account
            .recent
            .push_back(ProviderRequestPoint { ts: 123, ok: true });
        account
            .recent
            .push_back(ProviderRequestPoint { ts: 124, ok: false });

        account.recent_tokens.push_back(ProviderTokenPoint {
            ts: 125,
            input_tokens: 5,
            output_tokens: 2,
        });
        account.recent_tokens.push_back(ProviderTokenPoint {
            ts: 126,
            input_tokens: 1,
            output_tokens: 0,
        });

        let mut provider_map = HashMap::new();
        provider_map.insert("acc-1".to_string(), account);
        store.insert("provider-1".to_string(), provider_map);

        persist_metrics_to_path(&path, &store);
        let loaded = load_metrics_from_path(&path);

        let loaded_acc = loaded
            .get("provider-1")
            .and_then(|m| m.get("acc-1"))
            .cloned()
            .unwrap_or_default();
        assert_eq!(loaded_acc.total_requests, 5);
        assert_eq!(loaded_acc.success_requests, 4);
        assert_eq!(loaded_acc.failed_requests, 1);
        assert_eq!(loaded_acc.recent.len(), 2);
        assert_eq!(loaded_acc.total_input_tokens, 11);
        assert_eq!(loaded_acc.total_output_tokens, 7);
        assert_eq!(loaded_acc.total_tokens, 18);
        assert_eq!(loaded_acc.recent_tokens.len(), 2);
        let (i, o, t) = sum_recent_token_points(&loaded_acc.recent_tokens);
        assert_eq!(i, 6);
        assert_eq!(o, 2);
        assert_eq!(t, 8);

        let mut recent = loaded_acc.recent_tokens.clone();
        cleanup_recent_token_points(&mut recent, 126 + METRICS_WINDOW_SECONDS + 1);
        assert_eq!(recent.len(), 0);
        let (i2, o2, t2) = sum_recent_token_points(&recent);
        assert_eq!(i2, 0);
        assert_eq!(o2, 0);
        assert_eq!(t2, 0);

        // 注意：record_provider_token_usage 会写入全局 metrics_store 并落盘；
        // 单测这里仅验证序列化 roundtrip + 窗口清理逻辑。
        // 如需覆盖 record_provider_token_usage，应单独隔离 RSLLM_OAUTH_METRICS_FILE。
        let _no_side_effects = 1;

        // keep rustc happy for the comment above
        assert_eq!(_no_side_effects, 1);

        let _ = fs::remove_file(path);
    }
}

pub fn get_provider_account_metrics(
    provider_id: &str,
    provider_type: &str,
    account_key: &str,
) -> ProviderOAuthAccountMetricsSnapshot {
    if provider_id.trim().is_empty() || account_key.trim().is_empty() {
        return ProviderOAuthAccountMetricsSnapshot::default();
    }

    let scope_key = provider_scope_key(provider_id, provider_type);
    let now_ts = Utc::now().timestamp();
    if let Ok(mut guard) = metrics_store().lock()
        && let Some(provider_metrics) = guard.get_mut(&scope_key)
        && let Some(account_metrics) = provider_metrics.get_mut(account_key)
    {
        cleanup_recent_points(&mut account_metrics.recent, now_ts);
        cleanup_recent_token_points(&mut account_metrics.recent_tokens, now_ts);

        let recent_total = account_metrics.recent.len() as f64;
        let recent_failed = account_metrics.recent.iter().filter(|p| !p.ok).count() as f64;
        let (last_minute_input, last_minute_output, last_minute_total) =
            sum_recent_token_points(&account_metrics.recent_tokens);

        return ProviderOAuthAccountMetricsSnapshot {
            total_requests: account_metrics.total_requests,
            success_requests: account_metrics.success_requests,
            failed_requests: account_metrics.failed_requests,
            qps: if recent_total > 0.0 {
                recent_total / METRICS_WINDOW_SECONDS as f64
            } else {
                0.0
            },
            error_rate: if recent_total > 0.0 {
                recent_failed / recent_total
            } else {
                0.0
            },
            total_input_tokens: account_metrics.total_input_tokens,
            total_output_tokens: account_metrics.total_output_tokens,
            total_tokens: account_metrics.total_tokens,
            last_minute_input_tokens: last_minute_input,
            last_minute_output_tokens: last_minute_output,
            last_minute_total_tokens: last_minute_total,
        };
    }

    ProviderOAuthAccountMetricsSnapshot::default()
}

pub fn select_provider_oauth_token(
    config_details: &JsonValue,
    provider_id: &str,
    provider_type: &str,
) -> Option<SelectedProviderOAuthToken> {
    let auth_cfg = Some(config_details.clone());
    let accounts = provider_accounts_from_auth_config(&auth_cfg, provider_type);
    let scope_key = provider_scope_key(provider_id, provider_type);
    let now_ts = Utc::now().timestamp();

    let mut eligible: Vec<ProviderOAuthAccount> = accounts
        .iter()
        .filter(|acc| {
            acc.enabled
                && !acc.access_token.trim().is_empty()
                && !provider_account_needs_reauth(acc)
                && !is_account_in_cooldown(&scope_key, &acc.account_key, now_ts)
        })
        .cloned()
        .collect();

    if eligible.is_empty() {
        eligible = accounts
            .into_iter()
            .filter(|acc| acc.enabled && !acc.access_token.trim().is_empty())
            .collect();
    }

    if eligible.is_empty() {
        return None;
    }

    let total_weight: u64 = eligible
        .iter()
        .map(|acc| u64::from(acc.weight.max(1)))
        .sum();
    let cursor = {
        let mut guard = rr_counters().lock().ok()?;
        let entry = guard.entry(scope_key.clone()).or_insert(0);
        let cur = *entry;
        *entry = entry.wrapping_add(1);
        cur
    };

    let mut position = cursor % total_weight.max(1);
    for account in eligible {
        let weight = u64::from(account.weight.max(1));
        if position < weight {
            let token_type = if account.token_type.trim().is_empty()
                || account.token_type.eq_ignore_ascii_case("bearer")
            {
                "Bearer".to_string()
            } else {
                account.token_type.clone()
            };

            return Some(SelectedProviderOAuthToken {
                access_token: account.access_token,
                token_type,
                account_key: account.account_key,
                account_id: account.account_id,
                email: if account.email.trim().is_empty() {
                    None
                } else {
                    Some(account.email)
                },
            });
        }
        position -= weight;
    }

    None
}
