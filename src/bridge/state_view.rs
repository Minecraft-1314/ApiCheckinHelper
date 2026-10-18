use serde_json::{json, Map, Value};

use crate::core::config_store::ConfigStore;
use crate::core::model::{AppState, Credential, MetaEntry, RunResult, Site};
use crate::core::model_rules;

pub fn build(store: &ConfigStore, refresh: bool) -> Value {
    let state = store.load(refresh);
    let sites: Vec<Value> = state.sites.iter().map(|site| of(site, &state)).collect();
    let history: Map<String, Value> = state
        .history
        .iter()
        .map(|(key, bucket)| (key.clone(), bucket_value(bucket)))
        .collect();
    json!({
        "sites": sites,
        "settings": {
            "concurrency": state.settings.concurrency,
            "timeoutMs": state.settings.timeout_ms,
            "notify": state.settings.notify,
            "skipManualOnly": state.settings.skip_manual_only,
            "theme": state.settings.theme,
            "language": state.settings.language,
        },
        "history": Value::Object(history),
        "verify": {
            "at": state.verify.at,
            "total": state.verify.total,
            "ok": state.verify.ok,
            "failed": state.verify.failed,
            "items": state.verify.items.iter().map(|item| json!({
                "id": item.id,
                "name": item.name,
                "host": item.host,
                "ok": item.ok,
                "message": item.message,
            })).collect::<Vec<Value>>(),
        },
        "theme": state.settings.theme,
        "language": state.settings.language,
        "dataDirectory": crate::core::paths::display_path(&store.data_directory()),
        "settingsPath": crate::core::paths::display_path(&store.config_path()),
        "lastError": store.last_error(),
        "version": env!("CARGO_PKG_VERSION"),
        "runtimeVersion": env!("APP_RUST_VERSION"),
        "installPath": install_path(),
    })
}

fn install_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|value| value.to_string_lossy().to_string())
        })
        .unwrap_or_default()
}

pub fn of(site: &Site, state: &AppState) -> Value {
    let credentials = state.credentials.get(&site.id).cloned().unwrap_or_default();
    let meta = state.meta.get(&site.id).cloned().unwrap_or_default();
    let mut account_results: Vec<RunResult> = credentials
        .iter()
        .filter_map(|group| {
            let key = model_rules::account_key(&site.id, &group.group_id);
            state.results.get(&key).and_then(|item| item.clone())
        })
        .collect();
    account_results.sort_by_key(|item| std::cmp::Reverse(item.at));
    let direct = state.results.get(&site.id).and_then(|item| item.clone());
    let result = direct.or_else(|| account_results.first().cloned());
    let configured = credentials
        .iter()
        .any(|group| model_rules::is_configured(Some(group)));
    let first = credentials.first().cloned().unwrap_or_default();
    let group_views: Vec<Value> = credentials
        .iter()
        .map(|group| {
            let key = model_rules::account_key(&site.id, &group.group_id);
            let group_result = state.results.get(&key).and_then(|item| item.clone());
            json!({
                "groupId": group.group_id,
                "type": group.credential_type,
                "token": group.token,
                "userId": group.user_id,
                "result": group_result.as_ref().map(result_value),
            })
        })
        .collect();
    json!({
        "id": site.id,
        "name": site.name,
        "host": site.host,
        "origin": site.origin,
        "entry": site.entry,
        "bookmark": site.bookmark,
        "configured": configured,
        "credential": credential_value(&first),
        "credentials": group_views,
        "credentialCount": credentials.len(),
        "meta": meta_value(&meta),
        "snapshot": {
            "systemName": site.snapshot.system_name,
            "version": site.snapshot.version,
            "checkinEnabled": site.snapshot.checkin_enabled,
            "turnstile": site.snapshot.turnstile,
            "quotaPerUnit": site.snapshot.quota_per_unit,
            "displayInCurrency": site.snapshot.display_in_currency,
            "displayType": site.snapshot.display_type,
            "exchangeRate": site.snapshot.exchange_rate,
        },
        "result": result.as_ref().map(result_value),
    })
}

fn credential_value(credential: &Credential) -> Value {
    json!({
        "groupId": credential.group_id,
        "type": credential.credential_type,
        "token": credential.token,
        "userId": credential.user_id,
    })
}

fn meta_value(meta: &MetaEntry) -> Value {
    json!({
        "systemName": meta.system_name,
        "version": meta.version,
        "checkinEnabled": meta.checkin_enabled,
        "turnstile": meta.turnstile,
        "quotaPerUnit": meta.quota_per_unit,
        "displayInCurrency": meta.display_in_currency,
        "displayType": meta.display_type,
        "exchangeRate": meta.exchange_rate,
        "account": meta.account,
        "userId": meta.user_id,
        "probedAt": meta.probed_at,
    })
}

pub fn bucket_value(bucket: &crate::core::model::HistoryBucket) -> Value {
    json!({
        "total": bucket.total,
        "success": bucket.success,
        "already": bucket.already,
        "failed": bucket.failed,
        "skipped": bucket.skipped,
        "quota": bucket.quota,
        "quotaUsd": bucket.quota_usd,
        "at": bucket.at,
    })
}

pub fn result_value(result: &RunResult) -> Value {
    json!({
        "id": result.id,
        "state": result.state,
        "message": result.message,
        "quota": result.quota,
        "path": result.path,
        "httpStatus": result.http_status,
        "at": result.at,
    })
}
