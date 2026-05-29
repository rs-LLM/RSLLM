use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexOAuthAccount {
    pub account_key: String,
    #[serde(default)]
    pub api_key: String,
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

#[derive(Debug, Clone)]
pub struct SelectedCodexToken {
    pub access_token: String,
    pub token_type: String,
    pub account_key: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct CodexAccountMetricsSnapshot {
    pub total_requests: u64,
    pub success_requests: u64,
    pub failed_requests: u64,
    pub qps: f64,
    pub error_rate: f64,
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

static CODEX_RR_COUNTERS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
const METRICS_WINDOW_SECONDS: i64 = 60;
const UNAUTHORIZED_COOLDOWN_SECONDS: i64 = 300;

#[derive(Debug, Clone)]
struct CodexRequestPoint {
    ts: i64,
    ok: bool,
}

#[derive(Debug, Default)]
struct CodexAccountMetricsState {
    total_requests: u64,
    success_requests: u64,
    failed_requests: u64,
    recent: VecDeque<CodexRequestPoint>,
}

static CODEX_ACCOUNT_METRICS: OnceLock<
    Mutex<HashMap<String, HashMap<String, CodexAccountMetricsState>>>,
> = OnceLock::new();
static CODEX_ACCOUNT_COOLDOWNS: OnceLock<Mutex<HashMap<String, HashMap<String, i64>>>> =
    OnceLock::new();

fn counters() -> &'static Mutex<HashMap<String, u64>> {
    CODEX_RR_COUNTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn metrics_store() -> &'static Mutex<HashMap<String, HashMap<String, CodexAccountMetricsState>>> {
    CODEX_ACCOUNT_METRICS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cooldown_store() -> &'static Mutex<HashMap<String, HashMap<String, i64>>> {
    CODEX_ACCOUNT_COOLDOWNS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_account_in_cooldown(provider_id: &str, account_key: &str, now_ts: i64) -> bool {
    if let Ok(mut guard) = cooldown_store().lock()
        && let Some(provider_map) = guard.get_mut(provider_id)
    {
        provider_map.retain(|_, until_ts| *until_ts > now_ts);
        if let Some(until_ts) = provider_map.get(account_key) {
            return *until_ts > now_ts;
        }
    }
    false
}

pub fn mark_codex_account_unauthorized(provider_id: &str, account_key: &str) {
    if provider_id.trim().is_empty() || account_key.trim().is_empty() {
        return;
    }

    let until_ts = Utc::now().timestamp() + UNAUTHORIZED_COOLDOWN_SECONDS;
    if let Ok(mut guard) = cooldown_store().lock() {
        let provider_map = guard.entry(provider_id.to_string()).or_default();
        provider_map.insert(account_key.to_string(), until_ts);
    }
}

fn cleanup_recent_points(recent: &mut VecDeque<CodexRequestPoint>, now_ts: i64) {
    while let Some(front) = recent.front() {
        if now_ts - front.ts > METRICS_WINDOW_SECONDS {
            recent.pop_front();
        } else {
            break;
        }
    }
}

pub fn record_codex_request_result(provider_id: &str, account_key: &str, ok: bool) {
    if provider_id.trim().is_empty() || account_key.trim().is_empty() {
        return;
    }

    let now_ts = Utc::now().timestamp();
    if let Ok(mut guard) = metrics_store().lock() {
        let provider_metrics = guard.entry(provider_id.to_string()).or_default();
        let account_metrics = provider_metrics.entry(account_key.to_string()).or_default();

        account_metrics.total_requests = account_metrics.total_requests.saturating_add(1);
        if ok {
            account_metrics.success_requests = account_metrics.success_requests.saturating_add(1);
        } else {
            account_metrics.failed_requests = account_metrics.failed_requests.saturating_add(1);
        }

        account_metrics
            .recent
            .push_back(CodexRequestPoint { ts: now_ts, ok });
        cleanup_recent_points(&mut account_metrics.recent, now_ts);
    }
}

pub fn get_codex_account_metrics(
    provider_id: &str,
    account_key: &str,
) -> CodexAccountMetricsSnapshot {
    if provider_id.trim().is_empty() || account_key.trim().is_empty() {
        return CodexAccountMetricsSnapshot::default();
    }

    let now_ts = Utc::now().timestamp();
    if let Ok(mut guard) = metrics_store().lock()
        && let Some(provider_metrics) = guard.get_mut(provider_id)
        && let Some(account_metrics) = provider_metrics.get_mut(account_key)
    {
        cleanup_recent_points(&mut account_metrics.recent, now_ts);
        let recent_total = account_metrics.recent.len() as f64;
        let recent_failed = account_metrics.recent.iter().filter(|p| !p.ok).count() as f64;

        return CodexAccountMetricsSnapshot {
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
        };
    }

    CodexAccountMetricsSnapshot::default()
}

fn parse_expires_ts(s: &str) -> Option<i64> {
    if s.trim().is_empty() {
        return None;
    }

    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

pub fn account_needs_reauth(account: &CodexOAuthAccount) -> bool {
    if !account.api_key.trim().is_empty() {
        return false;
    }

    if account.access_token.trim().is_empty() {
        return true;
    }

    parse_expires_ts(&account.access_token_expires_at)
        .map(|ts| ts <= Utc::now().timestamp())
        .unwrap_or(false)
}

pub fn accounts_from_auth_config(auth_config: &Option<JsonValue>) -> Vec<CodexOAuthAccount> {
    let Some(auth_config) = auth_config else {
        return Vec::new();
    };

    let mut accounts = auth_config
        .get("codex_oauth_accounts")
        .and_then(|v| serde_json::from_value::<Vec<CodexOAuthAccount>>(v.clone()).ok())
        .unwrap_or_default();

    if accounts.is_empty()
        && let Some(legacy) = auth_config.get("codex_oauth")
    {
        if let Ok(mut account) = serde_json::from_value::<CodexOAuthAccount>(legacy.clone()) {
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

pub fn select_codex_oauth_token(
    config_details: &JsonValue,
    provider_id: &str,
) -> Option<SelectedCodexToken> {
    let auth_cfg = Some(config_details.clone());
    let accounts = accounts_from_auth_config(&auth_cfg);
    let now_ts = Utc::now().timestamp();

    let mut eligible: Vec<CodexOAuthAccount> = accounts
        .into_iter()
        .filter(|acc| {
            acc.enabled
                && (!acc.api_key.trim().is_empty() || !acc.access_token.trim().is_empty())
                && !account_needs_reauth(acc)
                && !is_account_in_cooldown(provider_id, &acc.account_key, now_ts)
        })
        .collect();

    if eligible.is_empty() {
        eligible = accounts_from_auth_config(&auth_cfg)
            .into_iter()
            .filter(|acc| {
                acc.enabled
                    && (!acc.api_key.trim().is_empty() || !acc.access_token.trim().is_empty())
            })
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
        let mut guard = counters().lock().ok()?;
        let entry = guard.entry(provider_id.to_string()).or_insert(0);
        let cur = *entry;
        *entry = entry.wrapping_add(1);
        cur
    };

    let mut position = cursor % total_weight.max(1);
    for account in eligible {
        let weight = u64::from(account.weight.max(1));
        if position < weight {
            return Some(SelectedCodexToken {
                access_token: if !account.api_key.trim().is_empty() {
                    account.api_key
                } else {
                    account.access_token
                },
                token_type: {
                    let raw = account.token_type.trim();
                    if raw.is_empty() || raw.eq_ignore_ascii_case("bearer") {
                        "Bearer".to_string()
                    } else {
                        raw.to_string()
                    }
                },
                account_key: account.account_key,
                account_id: account.account_id,
            });
        }
        position -= weight;
    }

    None
}
