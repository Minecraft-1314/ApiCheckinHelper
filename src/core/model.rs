use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct SiteSnapshot {
    pub system_name: String,
    pub version: Option<String>,
    pub checkin_enabled: Option<bool>,
    pub turnstile: bool,
    pub quota_per_unit: i32,
    pub display_in_currency: bool,
    pub display_type: String,
    pub exchange_rate: f64,
}

impl Default for SiteSnapshot {
    fn default() -> Self {
        Self {
            system_name: String::new(),
            version: None,
            checkin_enabled: None,
            turnstile: false,
            quota_per_unit: 500_000,
            display_in_currency: true,
            display_type: "USD".to_string(),
            exchange_rate: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub group_id: String,
    pub credential_type: String,
    pub token: String,
    pub user_id: i32,
}

impl Default for Credential {
    fn default() -> Self {
        Self {
            group_id: String::new(),
            credential_type: "token".to_string(),
            token: String::new(),
            user_id: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub host: String,
    pub origin: String,
    pub entry: String,
    pub tags: Vec<String>,
    pub bookmark: String,
    pub snapshot: SiteSnapshot,
}

impl Default for Site {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            host: String::new(),
            origin: String::new(),
            entry: "/".to_string(),
            tags: Vec::new(),
            bookmark: String::new(),
            snapshot: SiteSnapshot::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaEntry {
    pub system_name: String,
    pub version: String,
    pub checkin_enabled: Option<bool>,
    pub turnstile: bool,
    pub quota_per_unit: i32,
    pub display_in_currency: bool,
    pub display_type: String,
    pub exchange_rate: f64,
    pub account: String,
    pub user_id: i32,
    pub probed_at: i64,
}

impl Default for MetaEntry {
    fn default() -> Self {
        Self {
            system_name: String::new(),
            version: String::new(),
            checkin_enabled: None,
            turnstile: false,
            quota_per_unit: 500_000,
            display_in_currency: true,
            display_type: "USD".to_string(),
            exchange_rate: 1.0,
            account: String::new(),
            user_id: 0,
            probed_at: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub concurrency: i32,
    pub timeout_ms: i32,
    pub notify: bool,
    pub skip_manual_only: bool,
    pub theme: String,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            concurrency: 3,
            timeout_ms: 15_000,
            notify: true,
            skip_manual_only: true,
            theme: "auto".to_string(),
            language: "auto".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HistoryBucket {
    pub total: i32,
    pub success: i32,
    pub already: i32,
    pub failed: i32,
    pub skipped: i32,
    pub quota: i64,
    pub quota_usd: f64,
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunResult {
    pub id: String,
    pub state: String,
    pub message: String,
    pub quota: i64,
    pub path: String,
    pub http_status: i32,
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VerifyEntry {
    pub id: String,
    pub name: String,
    pub host: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VerifySummary {
    pub at: i64,
    pub total: i32,
    pub ok: i32,
    pub failed: i32,
    pub items: Vec<VerifyEntry>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LastRun {
    pub day: String,
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub version: i32,
    pub settings: Settings,
    pub sites: Vec<Site>,
    pub credentials: BTreeMap<String, Vec<Credential>>,
    pub meta: BTreeMap<String, MetaEntry>,
    pub results: BTreeMap<String, Option<RunResult>>,
    pub last_run: LastRun,
    pub verify: VerifySummary,
    pub history: BTreeMap<String, HistoryBucket>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            version: 1,
            settings: Settings::default(),
            sites: Vec::new(),
            credentials: BTreeMap::new(),
            meta: BTreeMap::new(),
            results: BTreeMap::new(),
            last_run: LastRun::default(),
            verify: VerifySummary::default(),
            history: BTreeMap::new(),
        }
    }
}

pub fn clone_site(site: &Site) -> Site {
    site.clone()
}

pub fn clone_credential(credential: &Credential) -> Credential {
    credential.clone()
}

pub fn clone_meta(meta: &MetaEntry) -> MetaEntry {
    meta.clone()
}

pub fn clone_bucket(bucket: &HistoryBucket) -> HistoryBucket {
    bucket.clone()
}
