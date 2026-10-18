use std::collections::BTreeMap;

use super::flexible;
use super::model::{
    Credential, HistoryBucket, MetaEntry, RunResult, Settings, Site, SiteSnapshot, VerifyEntry,
    VerifySummary,
};
use super::network_policy;

pub const STATE_VERSION: i32 = 1;
pub const HISTORY_LIMIT: usize = 30;
pub const DEFAULT_QUOTA_PER_UNIT: i32 = 500_000;
pub const DEFAULT_TIMEOUT_MS: i32 = 15_000;
pub const DEFAULT_CONCURRENCY: i32 = 3;
pub const CONCURRENCY_MIN: i32 = 1;
pub const CONCURRENCY_MAX: i32 = 6;
pub const TIMEOUT_MIN_MS: i32 = 5_000;
pub const TIMEOUT_MAX_MS: i32 = 60_000;
pub const MAX_NAME_LENGTH: usize = 80;
pub const MAX_ENTRY_LENGTH: usize = 2048;
pub const MAX_TOKEN_LENGTH: usize = 4096;
pub const MAX_GROUP_ID_LENGTH: usize = 64;
pub const MAX_CREDENTIAL_GROUPS: usize = 20;
pub const MAX_TAG_COUNT: usize = 8;
pub const MAX_TAG_LENGTH: usize = 80;
pub const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_IMPORT_ENTRIES: usize = 100;
pub const MAX_USER_ID: i32 = 1_000_000_000;

pub const CREDENTIAL_TYPES: [&str; 1] = ["token"];
pub const THEMES: [&str; 3] = ["auto", "light", "dark"];
pub const LANGUAGES: [&str; 3] = ["auto", "zh-cn", "en-us"];

pub fn slugify(host: Option<&str>) -> String {
    let mut builder = String::new();
    let mut pending_dash = false;
    for raw in host.unwrap_or("").chars() {
        let ch = raw.to_ascii_lowercase();
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            if pending_dash && !builder.is_empty() {
                builder.push('-');
            }
            pending_dash = false;
            builder.push(ch);
        } else {
            pending_dash = true;
        }
    }
    builder
}

pub fn account_key(site_id: &str, group_id: &str) -> String {
    if group_id.is_empty() {
        format!("{site_id}@")
    } else {
        format!("{site_id}@{group_id}")
    }
}

pub fn site_id_from_account_key(account_key: &str) -> String {
    match account_key.rfind('@') {
        Some(index) => account_key[..index].to_string(),
        None => account_key.to_string(),
    }
}

pub fn is_valid_host(host: &str) -> bool {
    network_policy::is_valid_host(host)
}

pub fn format_host(host: &str) -> String {
    network_policy::format_host(host)
}

pub fn to_punycode(host: &str) -> Option<String> {
    network_policy::to_punycode(host)
}

pub fn split_host(raw: Option<&str>) -> Option<(String, String)> {
    let text = raw.unwrap_or("").trim().to_ascii_lowercase();
    if text.is_empty() {
        return None;
    }
    let mut scheme = "https".to_string();
    let mut rest = text.as_str();
    if let Some(index) = rest.find("://") {
        scheme = rest[..index].trim().to_string();
        rest = &rest[index + 3..];
    }
    if scheme != "https" && scheme != "http" {
        return None;
    }
    if let Some(index) = rest.find(['#', '?']) {
        rest = &rest[..index];
    }
    if let Some(index) = rest.rfind('@') {
        rest = &rest[index + 1..];
    }
    if let Some(index) = rest.find('/') {
        rest = &rest[..index];
    }
    if rest.is_empty() {
        return None;
    }
    if rest.starts_with('[') {
        let close = rest.find(']')?;
        let inner = to_punycode(&rest[1..close]).unwrap_or_default();
        return if is_valid_host(&inner) {
            Some((inner, scheme))
        } else {
            None
        };
    }
    let mut host = rest.to_string();
    if let Some(colon) = rest.rfind(':') {
        let port = &rest[colon + 1..];
        if port.is_empty() || !port.chars().all(|item| item.is_ascii_digit()) {
            return None;
        }
        host = rest[..colon].to_string();
    }
    let ascii = to_punycode(&host)?;
    if !is_valid_host(&ascii) {
        return None;
    }
    Some((ascii, scheme))
}

pub fn normalize_entry(raw: Option<&str>) -> String {
    let entry = raw.unwrap_or("/").trim().to_string();
    if entry.is_empty() {
        return "/".to_string();
    }
    let normalized = if entry.starts_with('/') {
        entry
    } else {
        format!("/{entry}")
    };
    clamp(&normalized, MAX_ENTRY_LENGTH)
}

pub fn normalize_snapshot(source: Option<&SiteSnapshot>) -> SiteSnapshot {
    let empty = SiteSnapshot::default();
    let source = source.unwrap_or(&empty);
    let version = source
        .version
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string();
    let display_type = source.display_type.trim().to_ascii_uppercase();
    let quota = if source.quota_per_unit == flexible::INT_UNPARSABLE {
        DEFAULT_QUOTA_PER_UNIT
    } else {
        source.quota_per_unit
    };
    let exchange = if source.exchange_rate == flexible::DOUBLE_UNPARSABLE {
        1.0
    } else {
        source.exchange_rate
    };
    SiteSnapshot {
        system_name: source.system_name.clone(),
        version: if version.is_empty() {
            None
        } else {
            Some(version)
        },
        checkin_enabled: source.checkin_enabled,
        turnstile: source.turnstile,
        quota_per_unit: quota.max(1),
        display_in_currency: source.display_in_currency,
        display_type: if display_type.is_empty() {
            "USD".to_string()
        } else {
            clamp(&display_type, 12)
        },
        exchange_rate: clamp_float(exchange, 1.0, 0.000_001, 1_000_000.0),
    }
}

pub fn normalize_settings(source: Option<&Settings>) -> Settings {
    let empty = Settings::default();
    let source = source.unwrap_or(&empty);
    let theme = source.theme.trim().to_ascii_lowercase();
    let mut language = source.language.trim().to_ascii_lowercase();
    if matches!(language.as_str(), "zh" | "zh-cn" | "zh-hans") {
        language = "zh-cn".to_string();
    }
    if matches!(language.as_str(), "en" | "en-us" | "en-gb") {
        language = "en-us".to_string();
    }
    let concurrency = if source.concurrency == flexible::INT_UNPARSABLE {
        DEFAULT_CONCURRENCY
    } else {
        source.concurrency
    };
    let timeout = if source.timeout_ms == flexible::INT_UNPARSABLE {
        DEFAULT_TIMEOUT_MS
    } else {
        source.timeout_ms
    };
    Settings {
        concurrency: concurrency.clamp(CONCURRENCY_MIN, CONCURRENCY_MAX),
        timeout_ms: timeout.clamp(TIMEOUT_MIN_MS, TIMEOUT_MAX_MS),
        notify: source.notify,
        skip_manual_only: source.skip_manual_only,
        theme: if THEMES.contains(&theme.as_str()) {
            theme
        } else {
            "auto".to_string()
        },
        language: if LANGUAGES.contains(&language.as_str()) {
            language
        } else {
            "auto".to_string()
        },
    }
}

pub fn normalize_credential(source: Option<&Credential>) -> Credential {
    let empty = Credential::default();
    let source = source.unwrap_or(&empty);
    let credential_type = if CREDENTIAL_TYPES.contains(&source.credential_type.as_str()) {
        source.credential_type.clone()
    } else {
        "token".to_string()
    };
    Credential {
        group_id: clamp(source.group_id.trim(), MAX_GROUP_ID_LENGTH),
        credential_type,
        token: clamp(source.token.trim(), MAX_TOKEN_LENGTH),
        user_id: source.user_id.clamp(0, MAX_USER_ID),
    }
}

pub fn normalize_credential_groups(source: Option<&[Credential]>) -> Vec<Credential> {
    let mut groups: Vec<Credential> = Vec::new();
    let mut used: Vec<String> = Vec::new();
    let Some(items) = source else {
        return groups;
    };
    for item in items {
        let group = normalize_credential(Some(item));
        if !is_configured(Some(&group)) {
            continue;
        }
        if !group.group_id.is_empty() {
            if used.contains(&group.group_id) {
                continue;
            }
            used.push(group.group_id.clone());
        }
        if groups.len() >= MAX_CREDENTIAL_GROUPS {
            break;
        }
        groups.push(group);
    }
    let mut serial = 1;
    for group in groups.iter_mut() {
        if !group.group_id.is_empty() {
            continue;
        }
        let mut generated = format!("cred-{serial}");
        while used.contains(&generated) {
            serial += 1;
            generated = format!("cred-{serial}");
        }
        group.group_id = generated;
        serial += 1;
    }
    groups
}

pub fn normalize_site(source: Option<&Site>) -> Option<Site> {
    let source = source?;
    let host_raw = if source.host.trim().is_empty() {
        source.origin.clone()
    } else {
        source.host.clone()
    };
    let (host, scheme) = split_host(Some(&host_raw))?;
    let entry = normalize_entry(Some(&source.entry));
    let origin = format!("{}://{}", scheme, network_policy::format_host(&host));
    let raw_id = source.id.trim().to_ascii_lowercase();
    let name = clamp(source.name.trim(), MAX_NAME_LENGTH);
    let tags: Vec<String> = source
        .tags
        .iter()
        .filter(|tag| !tag.trim().is_empty())
        .take(MAX_TAG_COUNT)
        .map(|tag| clamp(tag.trim(), MAX_TAG_LENGTH))
        .collect();
    Some(Site {
        id: if raw_id.is_empty() {
            slugify(Some(&host))
        } else {
            raw_id
        },
        name: if name.is_empty() { host.clone() } else { name },
        host,
        origin: origin.clone(),
        entry: entry.clone(),
        tags,
        bookmark: format!("{origin}{entry}"),
        snapshot: normalize_snapshot(Some(&source.snapshot)),
    })
}

pub fn normalize_site_list(raw: Option<&[Site]>) -> Vec<Site> {
    let mut result: Vec<Site> = Vec::new();
    let Some(items) = raw else {
        return result;
    };
    let mut ids: Vec<String> = Vec::new();
    let mut hosts: Vec<String> = Vec::new();
    for item in items {
        let Some(site) = normalize_site(Some(item)) else {
            continue;
        };
        if ids.contains(&site.id) {
            continue;
        }
        ids.push(site.id.clone());
        if hosts.contains(&site.host) {
            continue;
        }
        hosts.push(site.host.clone());
        result.push(site);
    }
    result
}

pub fn normalize_result(source: Option<&RunResult>) -> Option<RunResult> {
    let source = source?;
    Some(RunResult {
        id: source.id.clone(),
        state: if source.state.is_empty() {
            "failed".to_string()
        } else {
            source.state.clone()
        },
        message: source.message.clone(),
        quota: source.quota,
        path: source.path.clone(),
        http_status: source.http_status.clamp(0, 1000),
        at: source.at.max(0),
    })
}

pub fn normalize_history_bucket(source: Option<&HistoryBucket>) -> HistoryBucket {
    let empty = HistoryBucket::default();
    let source = source.unwrap_or(&empty);
    HistoryBucket {
        total: source.total.max(0),
        success: source.success.max(0),
        already: source.already.max(0),
        failed: source.failed.max(0),
        skipped: source.skipped.max(0),
        quota: source.quota.max(0),
        quota_usd: clamp_float(source.quota_usd, 0.0, 0.0, 1_000_000_000_000.0),
        at: source.at.max(0),
    }
}

pub fn normalize_verify(source: Option<&VerifySummary>) -> VerifySummary {
    let empty = VerifySummary::default();
    let source = source.unwrap_or(&empty);
    VerifySummary {
        at: source.at.max(0),
        total: source.total.clamp(0, 1_000_000),
        ok: source.ok.clamp(0, 1_000_000),
        failed: source.failed.clamp(0, 1_000_000),
        items: source
            .items
            .iter()
            .map(|item| VerifyEntry {
                id: item.id.clone(),
                name: item.name.clone(),
                host: item.host.clone(),
                ok: item.ok,
                message: item.message.clone(),
            })
            .collect(),
    }
}

pub fn prune_history(history: BTreeMap<String, HistoryBucket>) -> BTreeMap<String, HistoryBucket> {
    if history.len() <= HISTORY_LIMIT {
        return history;
    }
    let mut result = BTreeMap::new();
    let keys: Vec<String> = history.keys().rev().take(HISTORY_LIMIT).cloned().collect();
    for key in keys {
        if let Some(bucket) = history.get(&key) {
            result.insert(key, bucket.clone());
        }
    }
    result
}

pub fn is_configured(credential: Option<&Credential>) -> bool {
    match credential {
        None => false,
        Some(item) => item.credential_type == "token" && !item.token.trim().is_empty(),
    }
}

pub fn meta_patch_from_snapshot(meta: Option<&SiteSnapshot>) -> MetaEntry {
    match meta {
        None => MetaEntry::default(),
        Some(value) => MetaEntry {
            system_name: value.system_name.clone(),
            version: value.version.clone().unwrap_or_default(),
            checkin_enabled: value.checkin_enabled,
            turnstile: value.turnstile,
            quota_per_unit: value.quota_per_unit,
            display_in_currency: value.display_in_currency,
            display_type: value.display_type.clone(),
            exchange_rate: value.exchange_rate,
            account: String::new(),
            user_id: 0,
            probed_at: 0,
        },
    }
}

pub fn clamp(value: &str, maximum_length: usize) -> String {
    if value.chars().count() <= maximum_length {
        value.to_string()
    } else {
        value.chars().take(maximum_length).collect()
    }
}

fn clamp_float(value: f64, fallback: f64, low: f64, high: f64) -> f64 {
    let base = if value.is_finite() { value } else { fallback };
    base.max(low).min(high)
}
