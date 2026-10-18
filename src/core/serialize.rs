use serde_json::{Map, Value};

use super::coerce;
use super::error::{AppError, AppResult};
use super::model::{
    AppState, Credential, HistoryBucket, LastRun, MetaEntry, RunResult, Settings, Site,
    SiteSnapshot, VerifyEntry, VerifySummary,
};

pub fn state_to_value(state: &AppState) -> Value {
    let mut root = Map::new();
    root.insert("Version".to_string(), Value::from(state.version));
    root.insert("Settings".to_string(), settings_to_value(&state.settings));
    root.insert(
        "Sites".to_string(),
        Value::Array(state.sites.iter().map(site_to_value).collect()),
    );
    let mut credentials = Map::new();
    for (key, groups) in &state.credentials {
        credentials.insert(
            key.clone(),
            Value::Array(groups.iter().map(credential_to_value).collect()),
        );
    }
    root.insert("Credentials".to_string(), Value::Object(credentials));
    let mut meta = Map::new();
    for (key, item) in &state.meta {
        meta.insert(key.clone(), meta_to_value(item));
    }
    root.insert("Meta".to_string(), Value::Object(meta));
    let mut results = Map::new();
    for (key, item) in &state.results {
        results.insert(
            key.clone(),
            match item {
                Some(result) => result_to_value(result),
                None => Value::Null,
            },
        );
    }
    root.insert("Results".to_string(), Value::Object(results));
    root.insert("LastRun".to_string(), last_run_to_value(&state.last_run));
    root.insert("Verify".to_string(), verify_to_value(&state.verify));
    let mut history = Map::new();
    for (key, bucket) in &state.history {
        history.insert(key.clone(), bucket_to_value(bucket));
    }
    root.insert("History".to_string(), Value::Object(history));
    Value::Object(root)
}

pub fn state_from_value(value: &Value) -> AppResult<AppState> {
    let root = value
        .as_object()
        .ok_or_else(|| AppError::general("配置内容为空"))?;
    let mut state = AppState {
        version: coerce::to_int(root.get("Version"), 1, None, None),
        settings: root
            .get("Settings")
            .map(settings_from_value)
            .unwrap_or_default(),
        sites: root
            .get("Sites")
            .and_then(|item| item.as_array())
            .map(|items| items.iter().map(site_from_value).collect())
            .unwrap_or_default(),
        ..AppState::default()
    };
    if let Some(map) = root.get("Credentials").and_then(|item| item.as_object()) {
        for (key, item) in map {
            let groups = match item {
                Value::Array(items) => items.iter().map(credential_from_value).collect(),
                Value::Null => Vec::new(),
                other => vec![credential_from_value(other)],
            };
            state.credentials.insert(key.clone(), groups);
        }
    }
    if let Some(map) = root.get("Meta").and_then(|item| item.as_object()) {
        for (key, item) in map {
            state.meta.insert(key.clone(), meta_from_value(item));
        }
    }
    if let Some(map) = root.get("Results").and_then(|item| item.as_object()) {
        for (key, item) in map {
            let result = if item.is_null() {
                None
            } else {
                Some(result_from_value(item))
            };
            state.results.insert(key.clone(), result);
        }
    }
    state.last_run = root
        .get("LastRun")
        .map(last_run_from_value)
        .unwrap_or_default();
    state.verify = root
        .get("Verify")
        .map(verify_from_value)
        .unwrap_or_default();
    if let Some(map) = root.get("History").and_then(|item| item.as_object()) {
        for (key, item) in map {
            state.history.insert(key.clone(), bucket_from_value(item));
        }
    }
    Ok(state)
}

fn settings_to_value(settings: &Settings) -> Value {
    let mut map = Map::new();
    map.insert("Concurrency".to_string(), Value::from(settings.concurrency));
    map.insert("TimeoutMs".to_string(), Value::from(settings.timeout_ms));
    map.insert("Notify".to_string(), Value::from(settings.notify));
    map.insert(
        "SkipManualOnly".to_string(),
        Value::from(settings.skip_manual_only),
    );
    map.insert("Theme".to_string(), Value::from(settings.theme.clone()));
    map.insert(
        "Language".to_string(),
        Value::from(settings.language.clone()),
    );
    Value::Object(map)
}

fn settings_from_value(value: &Value) -> Settings {
    let mut settings = Settings::default();
    settings.concurrency =
        coerce::to_int(value.get("Concurrency"), settings.concurrency, None, None);
    settings.timeout_ms = coerce::to_int(value.get("TimeoutMs"), settings.timeout_ms, None, None);
    settings.notify = coerce::to_bool(value.get("Notify"), settings.notify);
    settings.skip_manual_only =
        coerce::to_bool(value.get("SkipManualOnly"), settings.skip_manual_only);
    settings.theme = coerce::to_text(value.get("Theme"), &settings.theme);
    settings.language = coerce::to_text(value.get("Language"), &settings.language);
    settings
}

fn site_to_value(site: &Site) -> Value {
    let mut map = Map::new();
    map.insert("Id".to_string(), Value::from(site.id.clone()));
    map.insert("Name".to_string(), Value::from(site.name.clone()));
    map.insert("Host".to_string(), Value::from(site.host.clone()));
    map.insert("Origin".to_string(), Value::from(site.origin.clone()));
    map.insert("Entry".to_string(), Value::from(site.entry.clone()));
    map.insert(
        "Tags".to_string(),
        Value::Array(site.tags.iter().cloned().map(Value::from).collect()),
    );
    map.insert("Bookmark".to_string(), Value::from(site.bookmark.clone()));
    map.insert("Snapshot".to_string(), snapshot_to_value(&site.snapshot));
    Value::Object(map)
}

fn site_from_value(value: &Value) -> Site {
    Site {
        id: coerce::to_text(value.get("Id"), ""),
        name: coerce::to_text(value.get("Name"), ""),
        host: coerce::to_text(value.get("Host"), ""),
        origin: coerce::to_text(value.get("Origin"), ""),
        entry: coerce::to_text(value.get("Entry"), "/"),
        tags: value
            .get("Tags")
            .and_then(|item| item.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(|item| coerce::to_text(Some(item), ""))
                    .collect()
            })
            .unwrap_or_default(),
        bookmark: coerce::to_text(value.get("Bookmark"), ""),
        snapshot: value
            .get("Snapshot")
            .map(snapshot_from_value)
            .unwrap_or_default(),
    }
}

fn snapshot_to_value(snapshot: &SiteSnapshot) -> Value {
    let mut map = Map::new();
    map.insert(
        "SystemName".to_string(),
        Value::from(snapshot.system_name.clone()),
    );
    if let Some(text) = &snapshot.version {
        map.insert("Version".to_string(), Value::from(text.clone()));
    }
    if let Some(flag) = snapshot.checkin_enabled {
        map.insert("CheckinEnabled".to_string(), Value::from(flag));
    }
    map.insert("Turnstile".to_string(), Value::from(snapshot.turnstile));
    map.insert(
        "QuotaPerUnit".to_string(),
        Value::from(snapshot.quota_per_unit),
    );
    map.insert(
        "DisplayInCurrency".to_string(),
        Value::from(snapshot.display_in_currency),
    );
    map.insert(
        "DisplayType".to_string(),
        Value::from(snapshot.display_type.clone()),
    );
    map.insert(
        "ExchangeRate".to_string(),
        Value::from(snapshot.exchange_rate),
    );
    Value::Object(map)
}

pub fn snapshot_from_value(value: &Value) -> SiteSnapshot {
    let mut snapshot = SiteSnapshot::default();
    snapshot.system_name = coerce::to_text(value.get("SystemName"), "");
    snapshot.version = match value.get("Version") {
        None | Some(Value::Null) => None,
        Some(item) => Some(coerce::to_text(Some(item), "")),
    };
    snapshot.checkin_enabled = match value.get("CheckinEnabled") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(flag)) => Some(*flag),
        Some(item) => Some(coerce::to_bool(Some(item), false)),
    };
    snapshot.turnstile = coerce::to_bool(value.get("Turnstile"), false);
    snapshot.quota_per_unit = coerce::to_int(
        value.get("QuotaPerUnit"),
        snapshot.quota_per_unit,
        None,
        None,
    );
    snapshot.display_in_currency = coerce::to_bool(value.get("DisplayInCurrency"), true);
    snapshot.display_type = coerce::to_text(value.get("DisplayType"), "USD");
    snapshot.exchange_rate = coerce::to_float(value.get("ExchangeRate"), 1.0, None, None);
    snapshot
}

fn credential_to_value(credential: &Credential) -> Value {
    let mut map = Map::new();
    map.insert(
        "GroupId".to_string(),
        Value::from(credential.group_id.clone()),
    );
    map.insert(
        "Type".to_string(),
        Value::from(credential.credential_type.clone()),
    );
    map.insert("Token".to_string(), Value::from(credential.token.clone()));
    map.insert("UserId".to_string(), Value::from(credential.user_id));
    Value::Object(map)
}

fn credential_from_value(value: &Value) -> Credential {
    Credential {
        group_id: coerce::to_text(value.get("GroupId"), ""),
        credential_type: coerce::to_text(value.get("Type"), "token"),
        token: coerce::to_text(value.get("Token"), ""),
        user_id: coerce::to_int(value.get("UserId"), 0, None, None),
    }
}

fn meta_to_value(meta: &MetaEntry) -> Value {
    let mut map = Map::new();
    map.insert(
        "SystemName".to_string(),
        Value::from(meta.system_name.clone()),
    );
    map.insert("Version".to_string(), Value::from(meta.version.clone()));
    if let Some(flag) = meta.checkin_enabled {
        map.insert("CheckinEnabled".to_string(), Value::from(flag));
    }
    map.insert("Turnstile".to_string(), Value::from(meta.turnstile));
    map.insert("QuotaPerUnit".to_string(), Value::from(meta.quota_per_unit));
    map.insert(
        "DisplayInCurrency".to_string(),
        Value::from(meta.display_in_currency),
    );
    map.insert(
        "DisplayType".to_string(),
        Value::from(meta.display_type.clone()),
    );
    map.insert("ExchangeRate".to_string(), Value::from(meta.exchange_rate));
    map.insert("Account".to_string(), Value::from(meta.account.clone()));
    map.insert("UserId".to_string(), Value::from(meta.user_id));
    map.insert("ProbedAt".to_string(), Value::from(meta.probed_at));
    Value::Object(map)
}

fn meta_from_value(value: &Value) -> MetaEntry {
    let mut meta = MetaEntry::default();
    meta.system_name = coerce::to_text(value.get("SystemName"), "");
    meta.version = coerce::to_text(value.get("Version"), "");
    meta.checkin_enabled = match value.get("CheckinEnabled") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(flag)) => Some(*flag),
        Some(item) => Some(coerce::to_bool(Some(item), false)),
    };
    meta.turnstile = coerce::to_bool(value.get("Turnstile"), false);
    meta.quota_per_unit =
        coerce::to_int(value.get("QuotaPerUnit"), meta.quota_per_unit, None, None);
    meta.display_in_currency = coerce::to_bool(value.get("DisplayInCurrency"), true);
    meta.display_type = coerce::to_text(value.get("DisplayType"), "USD");
    meta.exchange_rate = coerce::to_float(value.get("ExchangeRate"), 1.0, None, None);
    meta.account = coerce::to_text(value.get("Account"), "");
    meta.user_id = coerce::to_int(value.get("UserId"), 0, None, None);
    meta.probed_at = coerce::to_long(value.get("ProbedAt"), 0, None, None);
    meta
}

fn result_to_value(result: &RunResult) -> Value {
    let mut map = Map::new();
    map.insert("Id".to_string(), Value::from(result.id.clone()));
    map.insert("State".to_string(), Value::from(result.state.clone()));
    map.insert("Message".to_string(), Value::from(result.message.clone()));
    map.insert("Quota".to_string(), Value::from(result.quota));
    map.insert("Path".to_string(), Value::from(result.path.clone()));
    map.insert("HttpStatus".to_string(), Value::from(result.http_status));
    map.insert("At".to_string(), Value::from(result.at));
    Value::Object(map)
}

fn result_from_value(value: &Value) -> RunResult {
    RunResult {
        id: coerce::to_text(value.get("Id"), ""),
        state: coerce::to_text(value.get("State"), ""),
        message: coerce::to_text(value.get("Message"), ""),
        quota: coerce::to_long(value.get("Quota"), 0, None, None),
        path: coerce::to_text(value.get("Path"), ""),
        http_status: coerce::to_int(value.get("HttpStatus"), 0, None, None),
        at: coerce::to_long(value.get("At"), 0, None, None),
    }
}

fn last_run_to_value(last_run: &LastRun) -> Value {
    let mut map = Map::new();
    map.insert("Day".to_string(), Value::from(last_run.day.clone()));
    map.insert("At".to_string(), Value::from(last_run.at));
    Value::Object(map)
}

fn last_run_from_value(value: &Value) -> LastRun {
    LastRun {
        day: coerce::to_text(value.get("Day"), ""),
        at: coerce::to_long(value.get("At"), 0, None, None),
    }
}

fn verify_to_value(summary: &VerifySummary) -> Value {
    let mut map = Map::new();
    map.insert("At".to_string(), Value::from(summary.at));
    map.insert("Total".to_string(), Value::from(summary.total));
    map.insert("Ok".to_string(), Value::from(summary.ok));
    map.insert("Failed".to_string(), Value::from(summary.failed));
    map.insert(
        "Items".to_string(),
        Value::Array(summary.items.iter().map(verify_entry_to_value).collect()),
    );
    Value::Object(map)
}

fn verify_from_value(value: &Value) -> VerifySummary {
    VerifySummary {
        at: coerce::to_long(value.get("At"), 0, None, None),
        total: coerce::to_int(value.get("Total"), 0, None, None),
        ok: coerce::to_int(value.get("Ok"), 0, None, None),
        failed: coerce::to_int(value.get("Failed"), 0, None, None),
        items: value
            .get("Items")
            .and_then(|item| item.as_array())
            .map(|items| items.iter().map(verify_entry_from_value).collect())
            .unwrap_or_default(),
    }
}

fn verify_entry_to_value(entry: &VerifyEntry) -> Value {
    let mut map = Map::new();
    map.insert("Id".to_string(), Value::from(entry.id.clone()));
    map.insert("Name".to_string(), Value::from(entry.name.clone()));
    map.insert("Host".to_string(), Value::from(entry.host.clone()));
    map.insert("Ok".to_string(), Value::from(entry.ok));
    map.insert("Message".to_string(), Value::from(entry.message.clone()));
    Value::Object(map)
}

fn verify_entry_from_value(value: &Value) -> VerifyEntry {
    VerifyEntry {
        id: coerce::to_text(value.get("Id"), ""),
        name: coerce::to_text(value.get("Name"), ""),
        host: coerce::to_text(value.get("Host"), ""),
        ok: coerce::to_bool(value.get("Ok"), false),
        message: coerce::to_text(value.get("Message"), ""),
    }
}

fn bucket_to_value(bucket: &HistoryBucket) -> Value {
    let mut map = Map::new();
    map.insert("Total".to_string(), Value::from(bucket.total));
    map.insert("Success".to_string(), Value::from(bucket.success));
    map.insert("Already".to_string(), Value::from(bucket.already));
    map.insert("Failed".to_string(), Value::from(bucket.failed));
    map.insert("Skipped".to_string(), Value::from(bucket.skipped));
    map.insert("Quota".to_string(), Value::from(bucket.quota));
    map.insert("QuotaUsd".to_string(), Value::from(bucket.quota_usd));
    map.insert("At".to_string(), Value::from(bucket.at));
    Value::Object(map)
}

fn bucket_from_value(value: &Value) -> HistoryBucket {
    HistoryBucket {
        total: coerce::to_int(value.get("Total"), 0, None, None),
        success: coerce::to_int(value.get("Success"), 0, None, None),
        already: coerce::to_int(value.get("Already"), 0, None, None),
        failed: coerce::to_int(value.get("Failed"), 0, None, None),
        skipped: coerce::to_int(value.get("Skipped"), 0, None, None),
        quota: coerce::to_long(value.get("Quota"), 0, None, None),
        quota_usd: coerce::to_float(value.get("QuotaUsd"), 0.0, None, None),
        at: coerce::to_long(value.get("At"), 0, None, None),
    }
}
