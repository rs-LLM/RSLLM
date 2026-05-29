use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct UpstreamOAuthInfo {
    pub provider_id: String,
    pub provider_type: String,
    pub account_key: String,
    pub account_id: String,
    pub email: Option<String>,
}

type TraceItem = (i64, UpstreamOAuthInfo);

type TraceStore = HashMap<String, TraceItem>;

static UPSTREAM_OAUTH_TRACE_STORE: OnceLock<Mutex<TraceStore>> = OnceLock::new();

fn store() -> &'static Mutex<TraceStore> {
    UPSTREAM_OAUTH_TRACE_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn cleanup(store: &mut TraceStore, now: i64) {
    // 仅保留 5 分钟内，且最多保留 10k 条（非常保守的上限）。
    const TTL_SECONDS: i64 = 300;
    const MAX_ITEMS: usize = 10_000;

    store.retain(|_, (ts, _)| now - *ts <= TTL_SECONDS);

    if store.len() <= MAX_ITEMS {
        return;
    }

    // 超过上限时按时间戳从旧到新清理。
    let mut items: Vec<(String, i64)> = store.iter().map(|(k, (ts, _))| (k.clone(), *ts)).collect();
    items.sort_by_key(|(_, ts)| *ts);

    let to_remove = store.len().saturating_sub(MAX_ITEMS);
    for (k, _) in items.into_iter().take(to_remove) {
        store.remove(&k);
    }
}

pub fn record_upstream_oauth(key: &str, info: UpstreamOAuthInfo) {
    let k = key.trim();
    if k.is_empty() {
        return;
    }

    let now = now_ts();
    if let Ok(mut guard) = store().lock() {
        cleanup(&mut guard, now);
        guard.insert(k.to_string(), (now, info));
    }
}

pub fn get_upstream_oauth(key: &str) -> Option<UpstreamOAuthInfo> {
    let k = key.trim();
    if k.is_empty() {
        return None;
    }

    let now = now_ts();
    if let Ok(mut guard) = store().lock() {
        cleanup(&mut guard, now);
        return guard.get(k).map(|(_, info)| info.clone());
    }

    None
}

pub fn remove_upstream_oauth(key: &str) {
    let k = key.trim();
    if k.is_empty() {
        return;
    }

    let now = now_ts();
    if let Ok(mut guard) = store().lock() {
        cleanup(&mut guard, now);
        guard.remove(k);
    }
}

#[allow(dead_code)]
pub fn take_upstream_oauth(key: &str) -> Option<UpstreamOAuthInfo> {
    let info = get_upstream_oauth(key);
    if info.is_some() {
        remove_upstream_oauth(key);
    }
    info
}
