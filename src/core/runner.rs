use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use super::cancel::CancelToken;
use super::checkin::{CheckinEngine, CheckinOutcome};
use super::config_store::ConfigStore;
use super::error::{AppError, AppResult};
use super::model::{Credential, MetaEntry, RunResult, Settings, Site, VerifyEntry, VerifySummary};
use super::model_rules;
use super::net::NetClient;
use super::protocol::ProtocolCaches;
use super::verify::{VerifyEngine, VerifyOutcome};

pub const MAX_WORKERS: i32 = 8;
pub const VERIFY_EXTRA_WORKERS: i32 = 2;

#[derive(Debug, Clone)]
pub struct PlanEntry {
    pub site: Site,
    pub group_id: String,
    pub credential: Credential,
    pub meta: MetaEntry,
}

#[derive(Debug, Clone, Default)]
pub struct CheckinSummary {
    pub total: i32,
    pub elapsed_ms: i64,
    pub results: BTreeMap<String, RunResult>,
}

pub struct Runner<'a> {
    store: &'a ConfigStore,
    net: &'a NetClient,
    caches: &'a ProtocolCaches,
}

impl<'a> Runner<'a> {
    pub fn new(store: &'a ConfigStore, net: &'a NetClient, caches: &'a ProtocolCaches) -> Self {
        Self { store, net, caches }
    }

    pub fn skip_reason(entry: &PlanEntry, settings: &Settings) -> Option<String> {
        if !model_rules::is_configured(Some(&entry.credential)) {
            return Some("未配置访问令牌".to_string());
        }
        if settings.skip_manual_only && entry.meta.turnstile {
            return Some("该站点需人机验证，仅能手动完成".to_string());
        }
        None
    }

    pub fn build_plan(&self, only_site_id: Option<&str>) -> (Vec<PlanEntry>, Settings) {
        let state = self.store.load(false);
        let settings = model_rules::normalize_settings(Some(&state.settings));
        let mut plan: Vec<PlanEntry> = Vec::new();
        for site in &state.sites {
            if let Some(target) = only_site_id {
                if site.id != target {
                    continue;
                }
            }
            let meta = state.meta.get(&site.id).cloned().unwrap_or_default();
            let groups = state.credentials.get(&site.id).cloned().unwrap_or_default();
            let configured: Vec<Credential> = groups
                .into_iter()
                .filter(|group| model_rules::is_configured(Some(group)))
                .collect();
            if configured.is_empty() {
                plan.push(PlanEntry {
                    site: site.clone(),
                    credential: model_rules::normalize_credential(None),
                    group_id: String::new(),
                    meta,
                });
                continue;
            }
            for group in configured {
                plan.push(PlanEntry {
                    site: site.clone(),
                    credential: group.clone(),
                    group_id: group.group_id.clone(),
                    meta: meta.clone(),
                });
            }
        }
        (plan, settings)
    }

    pub fn run_checkin(
        &self,
        only_site_id: Option<&str>,
        progress: &(dyn Fn(RunResult) + Sync),
        cancel: &CancelToken,
    ) -> AppResult<CheckinSummary> {
        cancel.throw_if_cancelled()?;
        let started = Instant::now();
        let started_at = self.store.now_ms();
        let (plan, settings) = self.build_plan(only_site_id);
        if plan.is_empty() {
            return Ok(CheckinSummary {
                total: 0,
                ..CheckinSummary::default()
            });
        }
        let limit = settings.concurrency.clamp(1, MAX_WORKERS) as usize;
        let results: Mutex<BTreeMap<String, RunResult>> = Mutex::new(BTreeMap::new());
        let metas: Mutex<BTreeMap<String, MetaEntry>> = Mutex::new(BTreeMap::new());
        let cursor = AtomicUsize::new(0);
        let workers = limit.min(plan.len());
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| loop {
                    if cancel.is_cancelled() {
                        return;
                    }
                    let index = cursor.fetch_add(1, Ordering::SeqCst);
                    let Some(entry) = plan.get(index) else {
                        return;
                    };
                    let item = self.run_one(entry, &settings, &metas, cancel);
                    if let Some(item) = item {
                        let key = item.id.clone();
                        if let Ok(mut map) = results.lock() {
                            map.insert(key, item.clone());
                        }
                        progress(item);
                    }
                });
            }
        });
        cancel.throw_if_cancelled()?;
        let collected: BTreeMap<String, MetaEntry> = metas.into_inner().unwrap_or_default();
        if !collected.is_empty() {
            self.store.batch(|store| store.set_meta_bulk(&collected))?;
        }
        let ordered: Vec<RunResult> = {
            let map = results.into_inner().unwrap_or_default();
            plan.iter()
                .filter_map(|entry| {
                    let key = model_rules::account_key(&entry.site.id, &entry.group_id);
                    map.get(&key).cloned()
                })
                .collect()
        };
        self.store.record_run(&ordered, started_at)?;
        let final_results = {
            let state = self.store.load(false);
            let mut map = BTreeMap::new();
            for entry in &plan {
                let key = model_rules::account_key(&entry.site.id, &entry.group_id);
                if let Some(item) = state.results.get(&key).and_then(|value| value.clone()) {
                    map.insert(key, item);
                }
            }
            map
        };
        Ok(CheckinSummary {
            total: plan.len() as i32,
            elapsed_ms: started.elapsed().as_millis() as i64,
            results: final_results,
        })
    }

    fn run_one(
        &self,
        entry: &PlanEntry,
        settings: &Settings,
        metas: &Mutex<BTreeMap<String, MetaEntry>>,
        cancel: &CancelToken,
    ) -> Option<RunResult> {
        if let Some(reason) = Self::skip_reason(entry, settings) {
            return Some(RunResult {
                id: model_rules::account_key(&entry.site.id, &entry.group_id),
                state: "skipped".to_string(),
                message: reason,
                ..RunResult::default()
            });
        }
        let engine = CheckinEngine::new(self.net, self.caches);
        let outcome = match engine.run(
            &entry.site,
            &entry.credential,
            settings.timeout_ms,
            Some(&entry.meta),
            Some(cancel),
        ) {
            Ok(value) => value,
            Err(error) => {
                if error.is_cancelled() {
                    return None;
                }
                CheckinOutcome {
                    state: "failed".to_string(),
                    message: format!("执行异常：{}: {}", error.type_name(), error.message()),
                    ..CheckinOutcome::default()
                }
            }
        };
        if let Some(meta) = outcome.meta.as_ref() {
            let mut patch = model_rules::meta_patch_from_snapshot(Some(meta));
            patch.account = if !outcome.account.is_empty() {
                outcome.account.clone()
            } else {
                entry.meta.account.clone()
            };
            patch.user_id = if outcome.user_id > 0 {
                outcome.user_id
            } else {
                entry.meta.user_id
            };
            if let Ok(mut map) = metas.lock() {
                map.insert(entry.site.id.clone(), patch);
            }
        }
        Some(RunResult {
            id: model_rules::account_key(&entry.site.id, &entry.group_id),
            state: outcome.state,
            message: outcome.message,
            quota: outcome.quota,
            path: outcome.path,
            http_status: outcome.http_status,
            at: 0,
        })
    }

    pub fn run_verify(
        &self,
        progress: &(dyn Fn(VerifyEntry) + Sync),
        cancel: &CancelToken,
    ) -> AppResult<VerifySummary> {
        cancel.throw_if_cancelled()?;
        let (plan, settings) = self.build_plan(None);
        let targets: Vec<PlanEntry> = plan
            .into_iter()
            .filter(|entry| model_rules::is_configured(Some(&entry.credential)))
            .collect();
        if targets.is_empty() {
            return Ok(VerifySummary::default());
        }
        let limit = (settings.concurrency + VERIFY_EXTRA_WORKERS).clamp(1, MAX_WORKERS) as usize;
        let items: Mutex<Vec<VerifyEntry>> = Mutex::new(Vec::new());
        let metas: Mutex<BTreeMap<String, MetaEntry>> = Mutex::new(BTreeMap::new());
        let ok_count = AtomicI32::new(0);
        let failed_count = AtomicI32::new(0);
        let cursor = AtomicUsize::new(0);
        let workers = limit.min(targets.len());
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| loop {
                    if cancel.is_cancelled() {
                        return;
                    }
                    let index = cursor.fetch_add(1, Ordering::SeqCst);
                    let Some(entry) = targets.get(index) else {
                        return;
                    };
                    let engine = VerifyEngine::new(self.net, self.caches);
                    let outcome = match engine.run(
                        &entry.site,
                        &entry.credential,
                        settings.timeout_ms,
                        Some(&entry.meta),
                        cancel,
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            if error.is_cancelled() {
                                return;
                            }
                            VerifyOutcome {
                                ok: false,
                                state: "failed".to_string(),
                                message: format!(
                                    "校验异常：{}: {}",
                                    error.type_name(),
                                    error.message()
                                ),
                                ..VerifyOutcome::default()
                            }
                        }
                    };
                    if outcome.ok {
                        ok_count.fetch_add(1, Ordering::SeqCst);
                        if let Ok(mut map) = metas.lock() {
                            map.insert(
                                entry.site.id.clone(),
                                MetaEntry {
                                    account: if outcome.account.is_empty() {
                                        entry.meta.account.clone()
                                    } else {
                                        outcome.account.clone()
                                    },
                                    user_id: if outcome.user_id > 0 {
                                        outcome.user_id
                                    } else {
                                        entry.meta.user_id
                                    },
                                    ..MetaEntry::default()
                                },
                            );
                        }
                    } else {
                        failed_count.fetch_add(1, Ordering::SeqCst);
                    }
                    let record = VerifyEntry {
                        id: model_rules::account_key(&entry.site.id, &entry.group_id),
                        name: entry.site.name.clone(),
                        host: entry.site.host.clone(),
                        ok: outcome.ok,
                        message: outcome.message,
                    };
                    if let Ok(mut list) = items.lock() {
                        list.push(record.clone());
                    }
                    progress(record);
                });
            }
        });
        cancel.throw_if_cancelled()?;
        let collected: BTreeMap<String, MetaEntry> = metas.into_inner().unwrap_or_default();
        let collected_items = items.into_inner().unwrap_or_default();
        let summary = VerifySummary {
            total: targets.len() as i32,
            ok: ok_count.load(Ordering::SeqCst),
            failed: failed_count.load(Ordering::SeqCst),
            items: collected_items,
            at: 0,
        };
        self.store.batch(|store| {
            if !collected.is_empty() {
                store.set_meta_bulk(&collected)?;
            }
            store.set_verify(&summary)
        })?;
        Ok(self.store.verify_summary())
    }

    pub fn probe(&self, site_id: &str, cancel: &CancelToken) -> AppResult<CheckinOutcome> {
        let Some(site) = self.store.find_site(site_id) else {
            return Ok(CheckinOutcome {
                state: "failed".to_string(),
                message: "站点不存在".to_string(),
                ..CheckinOutcome::default()
            });
        };
        let settings = self.store.settings();
        let meta = self.store.meta_for(site_id);
        let engine = CheckinEngine::new(self.net, self.caches);
        let outcome = match engine.probe(&site, settings.timeout_ms, Some(&meta), cancel) {
            Ok(value) => value,
            Err(error) => {
                if error.is_cancelled() {
                    return Err(error);
                }
                CheckinOutcome {
                    state: "offline".to_string(),
                    message: error
                        .fault()
                        .map(|fault| fault.message.clone())
                        .unwrap_or_else(|| error.message()),
                    ..CheckinOutcome::default()
                }
            }
        };
        if let Some(snapshot) = outcome.meta.as_ref() {
            self.store.set_meta(
                site_id,
                &model_rules::meta_patch_from_snapshot(Some(snapshot)),
            )?;
        }
        Ok(outcome)
    }
}

pub fn error_result(entry: &PlanEntry, error: &AppError) -> RunResult {
    RunResult {
        id: model_rules::account_key(&entry.site.id, &entry.group_id),
        state: "failed".to_string(),
        message: format!("执行异常：{}: {}", error.type_name(), error.message()),
        ..RunResult::default()
    }
}
