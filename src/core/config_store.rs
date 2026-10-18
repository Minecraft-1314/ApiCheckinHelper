use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use serde_json::Value;

use super::app_log::AppLog;
use super::coerce;
use super::config_schema;
use super::error::{AppError, AppResult};
use super::model::{
    AppState, Credential, HistoryBucket, LastRun, MetaEntry, RunResult, Settings, Site,
    VerifySummary,
};
use super::model_rules;
use super::paths::{self, Paths};
use super::protocol_async::now_ms;
use super::serialize;

pub struct ConfigStore {
    paths: Paths,
    log: Arc<AppLog>,
    inner: Mutex<Inner>,
}

struct Inner {
    state: Option<AppState>,
    revision: i64,
    dirty: bool,
    batch_depth: u32,
    last_error: String,
}

static GLOBAL: OnceLock<ConfigStore> = OnceLock::new();

impl ConfigStore {
    pub fn bind() -> &'static ConfigStore {
        GLOBAL.get_or_init(|| {
            let log = Arc::new(AppLog::bind());
            ConfigStore {
                paths: Paths::bind(),
                log,
                inner: Mutex::new(Inner {
                    state: None,
                    revision: 0,
                    dirty: false,
                    batch_depth: 0,
                    last_error: String::new(),
                }),
            }
        })
    }

    pub fn global() -> &'static ConfigStore {
        Self::bind()
    }

    pub fn data_directory(&self) -> PathBuf {
        self.paths.data_directory.clone()
    }

    pub fn config_path(&self) -> PathBuf {
        self.paths.config_path.clone()
    }

    pub fn log(&self) -> &AppLog {
        &self.log
    }

    pub fn last_error(&self) -> String {
        self.lock().last_error.clone()
    }

    pub fn revision(&self) -> i64 {
        self.lock().revision
    }

    pub fn today_key(&self) -> String {
        let seconds = now_ms() / 1000;
        let local = seconds + super::app_log::local_offset_seconds(seconds);
        let days = local.div_euclid(86_400);
        let (year, month, day) = super::app_log::civil_from_days(days);
        format!("{year:04}-{month:02}-{day:02}")
    }

    pub fn now_ms(&self) -> i64 {
        now_ms()
    }

    pub fn load(&self, refresh: bool) -> AppState {
        let mut inner = self.lock();
        self.load_locked(&mut inner, refresh).clone()
    }

    pub fn normalize(&self, source: AppState) -> AppState {
        let mut normalized = AppState {
            version: model_rules::STATE_VERSION,
            settings: model_rules::normalize_settings(Some(&source.settings)),
            sites: model_rules::normalize_site_list(Some(&source.sites)),
            last_run: LastRun {
                day: source.last_run.day.clone(),
                at: source.last_run.at.max(0),
            },
            verify: model_rules::normalize_verify(Some(&source.verify)),
            ..AppState::default()
        };
        for site in &normalized.sites {
            let groups = source.credentials.get(&site.id).map(|item| item.as_slice());
            normalized.credentials.insert(
                site.id.clone(),
                model_rules::normalize_credential_groups(groups),
            );
            normalized.meta.insert(
                site.id.clone(),
                source.meta.get(&site.id).cloned().unwrap_or_default(),
            );
            normalized.results.insert(
                site.id.clone(),
                model_rules::normalize_result(
                    source.results.get(&site.id).and_then(|item| item.as_ref()),
                ),
            );
            let site_groups = normalized
                .credentials
                .get(&site.id)
                .cloned()
                .unwrap_or_default();
            for group in site_groups {
                let account_key = model_rules::account_key(&site.id, &group.group_id);
                normalized.results.insert(
                    account_key.clone(),
                    model_rules::normalize_result(
                        source
                            .results
                            .get(&account_key)
                            .and_then(|item| item.as_ref()),
                    ),
                );
            }
        }
        for (key, bucket) in &source.history {
            normalized.history.insert(
                key.clone(),
                model_rules::normalize_history_bucket(Some(bucket)),
            );
        }
        normalized.history = model_rules::prune_history(normalized.history);
        normalized
    }

    pub fn flush(&self) -> bool {
        let mut inner = self.lock();
        !inner.dirty || self.write_now(&mut inner, true)
    }

    pub fn update<T>(&self, mutator: impl FnOnce(&mut AppState) -> AppResult<T>) -> AppResult<T> {
        let mut inner = self.lock();
        let mut working = self.load_locked(&mut inner, false).clone();
        let value = mutator(&mut working)?;
        let normalized = self.normalize(working);
        inner.state = Some(normalized);
        inner.revision += 1;
        inner.dirty = true;
        if inner.batch_depth == 0 {
            self.write_now(&mut inner, false);
        }
        Ok(value)
    }

    pub fn batch<T>(&self, action: impl FnOnce(&ConfigStore) -> AppResult<T>) -> AppResult<T> {
        {
            let mut inner = self.lock();
            inner.batch_depth += 1;
        }
        let outcome = action(self);
        let mut inner = self.lock();
        inner.batch_depth = inner.batch_depth.saturating_sub(1);
        if inner.batch_depth == 0 {
            self.write_now(&mut inner, false);
        }
        drop(inner);
        outcome
    }

    pub fn sites(&self) -> Vec<Site> {
        let mut inner = self.lock();
        self.load_locked(&mut inner, false).sites.clone()
    }

    pub fn find_site(&self, site_id: &str) -> Option<Site> {
        let mut inner = self.lock();
        self.load_locked(&mut inner, false)
            .sites
            .iter()
            .find(|site| site.id == site_id)
            .cloned()
    }

    pub fn find_by_host(&self, host: &str) -> Option<Site> {
        let key = strip_www(host);
        let mut inner = self.lock();
        self.load_locked(&mut inner, false)
            .sites
            .iter()
            .find(|site| strip_www(&site.host) == key)
            .cloned()
    }

    pub fn credentials_for(&self, site_id: &str) -> Credential {
        let groups = self.credential_groups_for(site_id);
        match groups.first() {
            Some(group) => group.clone(),
            None => model_rules::normalize_credential(None),
        }
    }

    pub fn credential_groups_for(&self, site_id: &str) -> Vec<Credential> {
        let mut inner = self.lock();
        self.load_locked(&mut inner, false)
            .credentials
            .get(site_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn credential_for(&self, site_id: &str, group_id: &str) -> Option<Credential> {
        let groups = self.credential_groups_for(site_id);
        let key = group_id.trim();
        if key.is_empty() {
            groups.first().cloned()
        } else {
            groups.iter().find(|group| group.group_id == key).cloned()
        }
    }

    pub fn credential_count(&self, site_id: Option<&str>) -> i32 {
        match site_id {
            None => {
                let mut inner = self.lock();
                let state = self.load_locked(&mut inner, false);
                state
                    .credentials
                    .values()
                    .flat_map(|groups| groups.iter())
                    .filter(|group| model_rules::is_configured(Some(group)))
                    .count() as i32
            }
            Some(id) => self
                .credential_groups_for(id)
                .iter()
                .filter(|group| model_rules::is_configured(Some(group)))
                .count() as i32,
        }
    }

    pub fn settings(&self) -> Settings {
        let mut inner = self.lock();
        self.load_locked(&mut inner, false).settings.clone()
    }

    pub fn meta_for(&self, site_id: &str) -> MetaEntry {
        let mut inner = self.lock();
        self.load_locked(&mut inner, false)
            .meta
            .get(site_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn result_for(&self, site_id: &str) -> Option<RunResult> {
        let mut inner = self.lock();
        let state = self.load_locked(&mut inner, false);
        model_rules::normalize_result(state.results.get(site_id).and_then(|item| item.as_ref()))
    }

    pub fn result_for_group(&self, site_id: &str, group_id: &str) -> Option<RunResult> {
        let account_key = model_rules::account_key(site_id, group_id);
        self.result_for(&account_key)
    }

    pub fn patch_settings(&self, patch: Settings) -> AppResult<()> {
        let normalized = model_rules::normalize_settings(Some(&patch));
        self.update(|state| {
            state.settings = normalized.clone();
            Ok(())
        })
    }

    pub fn set_credential(&self, site_id: &str, patch: Credential) -> AppResult<()> {
        let normalized = model_rules::normalize_credential(Some(&patch));
        self.update(|state| {
            if !state.credentials.contains_key(site_id) {
                return Ok(());
            }
            let existing: Vec<Credential> =
                state.credentials.get(site_id).cloned().unwrap_or_default();
            let mut next = normalized.clone();
            next.group_id = if existing.is_empty() {
                next_group_id(state, site_id)
            } else {
                existing[0].group_id.clone()
            };
            let groups = state.credentials.entry(site_id.to_string()).or_default();
            if groups.is_empty() {
                let key = model_rules::account_key(site_id, &next.group_id);
                state.results.entry(key).or_insert(None);
                groups.push(next);
            } else {
                groups[0] = next;
            }
            Ok(())
        })
    }

    pub fn save_credential_group(
        &self,
        site_id: &str,
        patch: Credential,
        group_id: &str,
    ) -> AppResult<Credential> {
        let mut normalized = model_rules::normalize_credential(Some(&patch));
        let trimmed = group_id.trim();
        if !trimmed.is_empty() {
            normalized.group_id = trimmed.to_string();
        }
        if !model_rules::is_configured(Some(&normalized)) {
            return Err(AppError::validation("请填写访问令牌"));
        }
        let saved = self.update(|state| {
            if !state.sites.iter().any(|site| site.id == site_id) {
                return Err(AppError::validation("站点不存在"));
            }
            let key = if normalized.group_id.is_empty() {
                next_group_id(state, site_id)
            } else {
                normalized.group_id.clone()
            };
            normalized.group_id = key.clone();
            let groups = state.credentials.entry(site_id.to_string()).or_default();
            match groups.iter().position(|group| group.group_id == key) {
                Some(index) => groups[index] = normalized.clone(),
                None => {
                    if groups.len() >= model_rules::MAX_CREDENTIAL_GROUPS {
                        return Err(AppError::validation(format!(
                            "每个站点最多 {} 组凭据",
                            model_rules::MAX_CREDENTIAL_GROUPS
                        )));
                    }
                    groups.push(normalized.clone());
                    let account_key = model_rules::account_key(site_id, &key);
                    state.results.entry(account_key).or_insert(None);
                }
            }
            Ok(key)
        })?;
        Ok(self.credential_for(site_id, &saved).unwrap_or(normalized))
    }

    pub fn add_credential_groups(&self, site_id: &str, patches: &[Credential]) -> AppResult<i32> {
        self.update(|state| {
            if !state.sites.iter().any(|site| site.id == site_id) {
                return Err(AppError::validation("站点不存在"));
            }
            let mut added = 0;
            let mut pending: Vec<Credential> = Vec::new();
            let existing = state
                .credentials
                .get(site_id)
                .map(|groups| groups.len())
                .unwrap_or(0);
            for patch in patches {
                let mut normalized = model_rules::normalize_credential(Some(patch));
                if !model_rules::is_configured(Some(&normalized)) {
                    continue;
                }
                if existing + pending.len() >= model_rules::MAX_CREDENTIAL_GROUPS {
                    break;
                }
                if normalized.group_id.is_empty() {
                    normalized.group_id = next_group_id_with(state, site_id, &pending);
                }
                let account_key = model_rules::account_key(site_id, &normalized.group_id);
                state.results.entry(account_key).or_insert(None);
                pending.push(normalized);
                added += 1;
            }
            state
                .credentials
                .entry(site_id.to_string())
                .or_default()
                .extend(pending);
            Ok(added)
        })
    }

    pub fn remove_credential_group(&self, site_id: &str, group_id: &str) -> AppResult<()> {
        let key = group_id.trim().to_string();
        if key.is_empty() {
            return Ok(());
        }
        self.update(|state| {
            let Some(groups) = state.credentials.get_mut(site_id) else {
                return Ok(());
            };
            groups.retain(|group| group.group_id != key);
            let account_key = model_rules::account_key(site_id, &key);
            state.results.remove(&account_key);
            Ok(())
        })
    }

    pub fn clear_credential(&self, site_id: &str) -> AppResult<()> {
        self.update(|state| {
            state.credentials.insert(site_id.to_string(), Vec::new());
            let prefix = format!("{site_id}@");
            let keys: Vec<String> = state
                .results
                .keys()
                .filter(|key| key.as_str() == site_id || key.starts_with(&prefix))
                .cloned()
                .collect();
            for key in keys {
                state.results.insert(key, None);
            }
            Ok(())
        })
    }

    pub fn clear_credentials(&self) -> AppResult<()> {
        self.update(|state| {
            let ids: Vec<String> = state.credentials.keys().cloned().collect();
            for site_id in ids {
                state.credentials.insert(site_id.clone(), Vec::new());
                let prefix = format!("{site_id}@");
                let keys: Vec<String> = state
                    .results
                    .keys()
                    .filter(|key| key.as_str() == site_id || key.starts_with(&prefix))
                    .cloned()
                    .collect();
                for key in keys {
                    state.results.insert(key, None);
                }
            }
            Ok(())
        })
    }

    pub fn set_user_id(&self, site_id: &str, user_id: i32) -> AppResult<()> {
        self.update(|state| {
            let Some(groups) = state.credentials.get_mut(site_id) else {
                return Ok(());
            };
            if groups.is_empty() {
                return Ok(());
            }
            let mut group = model_rules::normalize_credential(Some(&groups[0]));
            group.user_id = coerce::to_int(
                Some(&Value::from(user_id)),
                0,
                Some(0),
                Some(model_rules::MAX_USER_ID),
            );
            groups[0] = group;
            Ok(())
        })
    }

    pub fn set_meta(&self, site_id: &str, patch: &MetaEntry) -> AppResult<()> {
        let stamp = now_ms();
        self.update(|state| {
            let mut next = state.meta.get(site_id).cloned().unwrap_or_default();
            apply_meta(&mut next, patch);
            next.probed_at = stamp;
            state.meta.insert(site_id.to_string(), next);
            Ok(())
        })
    }

    pub fn set_meta_bulk(&self, patches: &BTreeMap<String, MetaEntry>) -> AppResult<()> {
        let stamp = now_ms();
        self.update(|state| {
            for (site_id, patch) in patches {
                let Some(current) = state.meta.get(site_id).cloned() else {
                    continue;
                };
                let mut next = current;
                apply_meta(&mut next, patch);
                next.probed_at = stamp;
                state.meta.insert(site_id.clone(), next);
            }
            Ok(())
        })
    }

    pub fn set_verify(&self, summary: &VerifySummary) -> AppResult<()> {
        let mut normalized = model_rules::normalize_verify(Some(summary));
        normalized.at = now_ms();
        self.update(|state| {
            state.verify = normalized.clone();
            Ok(())
        })
    }

    pub fn add_site(&self, host: &str, name: &str) -> AppResult<Site> {
        let draft = Site {
            host: host.to_string(),
            name: name.to_string(),
            ..Site::default()
        };
        let Some(mut normalized) = model_rules::normalize_site(Some(&draft)) else {
            return Err(AppError::validation(
                "站点地址不合法，请填写形如 api.example.com 的域名",
            ));
        };
        {
            let mut inner = self.lock();
            let state = self.load_locked(&mut inner, false);
            if state.sites.iter().any(|site| site.host == normalized.host) {
                return Err(AppError::validation(format!(
                    "这个域名已经在列表里了：{}",
                    normalized.host
                )));
            }
        }
        let base_id = normalized.id.clone();
        let existing: Vec<String> = self.sites().iter().map(|site| site.id.clone()).collect();
        let mut suffix = 2;
        while existing.contains(&normalized.id) {
            normalized.id = format!("{base_id}-{suffix}");
            suffix += 1;
        }
        let added = normalized.clone();
        self.update(|state| {
            state.sites.push(added.clone());
            attach(state, &added);
            Ok(())
        })?;
        Ok(added)
    }

    pub fn update_site(
        &self,
        site_id: &str,
        host: Option<&str>,
        name: Option<&str>,
    ) -> AppResult<Site> {
        let current = self
            .find_site(site_id)
            .ok_or_else(|| AppError::validation("站点不存在"))?;
        let mut draft = current.clone();
        if let Some(value) = host {
            draft.host = value.to_string();
        }
        if let Some(value) = name {
            draft.name = value.to_string();
        }
        let Some(mut normalized) = model_rules::normalize_site(Some(&draft)) else {
            return Err(AppError::validation("站点地址不合法"));
        };
        normalized.id = site_id.to_string();
        if self
            .sites()
            .iter()
            .any(|site| site.host == normalized.host && site.id != site_id)
        {
            return Err(AppError::validation(format!(
                "这个域名已被其他站点占用：{}",
                normalized.host
            )));
        }
        let next = normalized.clone();
        self.update(|state| {
            if let Some(index) = state.sites.iter().position(|site| site.id == site_id) {
                state.sites[index] = next.clone();
            }
            attach(state, &next);
            Ok(())
        })?;
        Ok(next)
    }

    pub fn remove_site(&self, site_id: &str) -> AppResult<()> {
        if self.sites().iter().all(|site| site.id != site_id) {
            return Err(AppError::validation("站点不存在"));
        }
        self.update(|state| {
            let prefix = format!("{site_id}@");
            let keys: Vec<String> = state
                .results
                .keys()
                .filter(|key| key.as_str() == site_id || key.starts_with(&prefix))
                .cloned()
                .collect();
            for key in keys {
                state.results.remove(&key);
            }
            state.sites.retain(|site| site.id != site_id);
            state.credentials.remove(site_id);
            state.meta.remove(site_id);
            Ok(())
        })
    }

    pub fn reset_all(&self) -> AppState {
        let mut inner = self.lock();
        if self.paths.config_path.exists() {
            if let Err(error) = fs::remove_file(&self.paths.config_path) {
                inner.last_error = "现有配置无法删除，已回退到空配置".to_string();
                self.log.error("现有配置无法删除", Some(&error.to_string()));
            }
        }
        inner.state = None;
        inner.dirty = false;
        let state = self.load_locked(&mut inner, true).clone();
        if inner.batch_depth == 0 {
            self.write_now(&mut inner, false);
        }
        state
    }

    pub fn record_run(&self, results: &[RunResult], started_at: i64) -> AppResult<()> {
        let day = self.today_key();
        let units: BTreeMap<String, i32> = self
            .sites()
            .iter()
            .map(|site| {
                (
                    site.id.clone(),
                    if site.snapshot.quota_per_unit > 0 {
                        site.snapshot.quota_per_unit
                    } else {
                        model_rules::DEFAULT_QUOTA_PER_UNIT
                    },
                )
            })
            .collect();
        let site_ids: BTreeSet<String> = self.sites().iter().map(|site| site.id.clone()).collect();
        self.update(|state| {
            let mut bucket = state.history.get(&day).cloned().unwrap_or_default();
            for result in results {
                let result_site_id = model_rules::site_id_from_account_key(&result.id);
                if state.results.contains_key(&result.id) || site_ids.contains(&result_site_id) {
                    let mut stored = result.clone();
                    stored.at = now_ms();
                    state.results.insert(result.id.clone(), Some(stored));
                }
                bucket.total += 1;
                if result.state == "success" {
                    bucket.success += 1;
                    let quota = coerce::to_long(
                        Some(&Value::from(result.quota)),
                        0,
                        Some(0),
                        Some(i64::MAX),
                    );
                    bucket.quota += quota;
                    let mut unit = *units
                        .get(&result_site_id)
                        .unwrap_or(&model_rules::DEFAULT_QUOTA_PER_UNIT);
                    if unit <= 0 {
                        unit = model_rules::DEFAULT_QUOTA_PER_UNIT;
                    }
                    bucket.quota_usd += quota as f64 / f64::from(unit);
                } else if result.state == "already" {
                    bucket.already += 1;
                } else if result.state == "skipped" {
                    bucket.skipped += 1;
                } else {
                    bucket.failed += 1;
                }
            }
            bucket.at = now_ms();
            state.history.insert(day.clone(), bucket);
            state.history = model_rules::prune_history(state.history.clone());
            state.last_run = LastRun {
                day: day.clone(),
                at: started_at.max(0),
            };
            Ok(())
        })
    }

    pub fn history_for(&self, day: Option<&str>) -> HistoryBucket {
        let key = match day {
            Some(value) if !value.is_empty() => value.to_string(),
            _ => self.today_key(),
        };
        let mut inner = self.lock();
        let state = self.load_locked(&mut inner, false);
        model_rules::normalize_history_bucket(state.history.get(&key))
    }

    pub fn last_run(&self) -> LastRun {
        let mut inner = self.lock();
        self.load_locked(&mut inner, false).last_run.clone()
    }

    pub fn verify_summary(&self) -> VerifySummary {
        let mut inner = self.lock();
        let state = self.load_locked(&mut inner, false);
        model_rules::normalize_verify(Some(&state.verify))
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn load_locked(&self, inner: &mut Inner, refresh: bool) -> AppState {
        if inner.state.is_none() || refresh {
            inner.last_error = String::new();
            inner.state = Some(self.build_initial(inner));
        }
        inner.state.clone().unwrap_or_default()
    }

    fn build_initial(&self, inner: &mut Inner) -> AppState {
        if paths::ensure_data_directory(&self.paths.data_directory).is_err() {
            return self.normalize(AppState::default());
        }
        if !self.paths.config_path.exists() {
            return self.normalize(AppState::default());
        }
        let metadata = fs::metadata(&self.paths.config_path);
        if let Ok(meta) = metadata {
            if meta.len() > model_rules::MAX_CONFIG_BYTES {
                self.quarantine(
                    inner,
                    "配置文件超过大小上限，已备份并回退到空配置",
                    "配置文件超过大小上限",
                );
                return self.normalize(AppState::default());
            }
        }
        let json = match fs::read_to_string(&self.paths.config_path) {
            Ok(value) => value,
            Err(error) => {
                inner.last_error = "配置文件暂时无法读取，已回退到空配置".to_string();
                self.log
                    .error("配置文件无法读取，回退到初始状态", Some(&error.to_string()));
                return self.normalize(AppState::default());
            }
        };
        if let Err(error) = config_schema::validate(&json) {
            self.quarantine(
                inner,
                "配置文件不符合当前格式，已备份并回退到空配置",
                &error.to_string(),
            );
            return self.normalize(AppState::default());
        }
        let value: Value = match serde_json::from_str(&json) {
            Ok(value) => value,
            Err(error) => {
                self.quarantine(
                    inner,
                    "配置文件不符合当前格式，已备份并回退到空配置",
                    &error.to_string(),
                );
                return self.normalize(AppState::default());
            }
        };
        match serialize::state_from_value(&value) {
            Ok(state) => self.normalize(state),
            Err(error) => {
                self.quarantine(
                    inner,
                    "配置文件不符合当前格式，已备份并回退到空配置",
                    &error.message(),
                );
                self.normalize(AppState::default())
            }
        }
    }

    fn quarantine(&self, inner: &mut Inner, user_message: &str, detail: &str) {
        let stamp = now_ms();
        let mut target = self
            .paths
            .data_directory
            .join(format!("config.corrupt-{stamp}.json"));
        let mut suffix = 1;
        while target.exists() {
            target = self
                .paths
                .data_directory
                .join(format!("config.corrupt-{stamp}-{suffix}.json"));
            suffix += 1;
        }
        match fs::rename(&self.paths.config_path, &target) {
            Ok(()) => {
                inner.last_error = user_message.to_string();
                let name = target
                    .file_name()
                    .map(|item| item.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.log
                    .error(&format!("配置文件损坏，已备份为 {name}"), Some(detail));
            }
            Err(error) => {
                inner.last_error = "配置文件无法隔离，已暂时回退到空配置".to_string();
                self.log
                    .error("配置文件损坏且无法备份", Some(&error.to_string()));
            }
        }
    }

    fn write_now(&self, inner: &mut Inner, durable: bool) -> bool {
        let Some(state) = inner.state.as_ref() else {
            inner.dirty = false;
            return true;
        };
        let temp_path = self
            .paths
            .config_path
            .with_extension(format!("{}.tmp", std::process::id()));
        let value = serialize::state_to_value(state);
        let bytes = match serde_json::to_vec(&value) {
            Ok(value) => value,
            Err(error) => {
                inner.last_error = "配置写入失败，本次改动仅保留在内存中".to_string();
                self.log.error(
                    "写入配置失败，本次改动仅保留在内存中",
                    Some(&error.to_string()),
                );
                return false;
            }
        };
        if paths::ensure_data_directory(&self.paths.data_directory).is_err() {
            inner.last_error = "配置写入失败，本次改动仅保留在内存中".to_string();
            self.log.warn("数据目录不可用，写入已跳过");
            return false;
        }
        let write_result = fs::write(&temp_path, &bytes).and_then(|()| {
            if durable {
                if let Ok(file) = fs::OpenOptions::new().write(true).open(&temp_path) {
                    let _ = file.sync_all();
                }
            }
            fs::rename(&temp_path, &self.paths.config_path)
        });
        match write_result {
            Ok(()) => {
                inner.dirty = false;
                inner.last_error = String::new();
                true
            }
            Err(error) => {
                inner.last_error = "配置写入失败，本次改动仅保留在内存中".to_string();
                self.log.error(
                    "写入配置失败，本次改动仅保留在内存中",
                    Some(&error.to_string()),
                );
                if temp_path.exists() {
                    let _ = fs::remove_file(&temp_path);
                }
                false
            }
        }
    }
}

pub fn strip_www(host: &str) -> String {
    let normalized = host.trim().to_ascii_lowercase();
    normalized
        .strip_prefix("www.")
        .map(|value| value.to_string())
        .unwrap_or(normalized)
}

fn attach(state: &mut AppState, site: &Site) {
    state.credentials.entry(site.id.clone()).or_default();
    state.meta.entry(site.id.clone()).or_default();
    state.results.entry(site.id.clone()).or_insert(None);
}

fn apply_meta(target: &mut MetaEntry, patch: &MetaEntry) {
    target.system_name = patch.system_name.clone();
    target.version = patch.version.clone();
    target.checkin_enabled = patch.checkin_enabled;
    target.turnstile = patch.turnstile;
    target.quota_per_unit = patch.quota_per_unit;
    target.display_in_currency = patch.display_in_currency;
    target.display_type = patch.display_type.clone();
    target.exchange_rate = patch.exchange_rate;
    if !patch.account.is_empty() {
        target.account = patch.account.clone();
    }
    if patch.user_id > 0 {
        target.user_id = patch.user_id;
    }
}

fn next_group_id(state: &AppState, site_id: &str) -> String {
    next_group_id_with(state, site_id, &[])
}

fn next_group_id_with(state: &AppState, site_id: &str, pending: &[Credential]) -> String {
    let mut used: Vec<String> = state
        .credentials
        .get(site_id)
        .map(|groups| groups.iter().map(|group| group.group_id.clone()).collect())
        .unwrap_or_default();
    used.extend(pending.iter().map(|group| group.group_id.clone()));
    for serial in 1..=(model_rules::MAX_CREDENTIAL_GROUPS + 1) {
        let candidate = format!("cred-{serial}");
        if !used.contains(&candidate) {
            return candidate;
        }
    }
    format!("cred-{}", now_ms())
}
