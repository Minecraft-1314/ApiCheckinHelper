use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::config_store::ConfigStore;
use crate::core::credential_import::{self, CredentialImportEntry};
use crate::core::error::{AppError, AppResult};
use crate::core::model::{Credential, MetaEntry, Settings};
use crate::core::model_rules;
use crate::core::net::NetClient;
use crate::core::oauth_flow;
use crate::core::protocol::ProtocolCaches;
use crate::core::runner::Runner;
use crate::core::verify::VerifyEngine;

use super::bridge_json;
use super::external_launcher;
use super::operation_gate::OperationGate;
use super::state_view;

pub const MAX_ADDRESS_LENGTH: usize = 2048;
pub const MAX_SITE_ID_LENGTH: usize = 128;
pub const MAX_NAME_LENGTH: usize = 80;
pub const MAX_SHORT_VALUE_LENGTH: usize = 32;

pub struct CommandRouter {
    store: &'static ConfigStore,
    net: Arc<NetClient>,
    caches: Arc<ProtocolCaches>,
    operations: OperationGate,
    progress_count: AtomicI32,
    post_event: Box<dyn Fn(Value) + Send + Sync>,
}

impl CommandRouter {
    pub fn new(post_event: Box<dyn Fn(Value) + Send + Sync>) -> Self {
        Self {
            store: ConfigStore::global(),
            net: Arc::new(NetClient::new()),
            caches: Arc::new(ProtocolCaches::new()),
            operations: OperationGate::new(),
            progress_count: AtomicI32::new(0),
            post_event,
        }
    }

    pub fn dispose(&self) {
        self.operations.cancel_current();
    }

    pub fn store(&self) -> &'static ConfigStore {
        self.store
    }

    pub fn execute(&self, action: &str, payload: &Value) -> AppResult<Value> {
        match action {
            "getState" => Ok(state_view::build(
                self.store,
                bridge_json::boolean(payload, "refresh", false),
            )),
            "openExternal" => Ok(Value::Bool(external_launcher::open(
                self.store,
                &self.read_text(payload, "url", MAX_ADDRESS_LENGTH)?,
            ))),
            "openDataDirectory" => Ok(Value::Bool(external_launcher::open_data_directory(
                self.store, None,
            ))),
            "addSite" => self.add_site(payload),
            "discoverSite" => self.discover(payload),
            "probeSite" => self.probe_site(payload),
            "renameSite" => self.rename_site(payload),
            "removeSite" => self.remove_site(payload),
            "saveCredential" => self.save_credential(payload),
            "removeCredential" => self.remove_credential(payload),
            "clearCredential" => self.clear_credential(payload),
            "clearAllCredentials" => {
                self.store.clear_credentials()?;
                Ok(Value::Bool(true))
            }
            "verifySite" => self.verify_site(payload),
            "verifyAll" => self.verify_all(),
            "importCredentials" => self.import_credentials(payload),
            "runCheckin" => self.run_checkin(payload),
            "patchSettings" => self.patch_settings(payload),
            "setTheme" => self.set_theme(payload),
            "cancel" => {
                self.operations.cancel_current();
                Ok(Value::Bool(true))
            }
            _ => Err(AppError::validation(format!("未知的界面操作：{action}"))),
        }
    }

    fn add_site(&self, payload: &Value) -> AppResult<Value> {
        let site = self.store.add_site(
            &self.read_text(payload, "host", MAX_ADDRESS_LENGTH)?,
            &self.read_text(payload, "name", MAX_NAME_LENGTH)?,
        )?;
        Ok(state_view::of(&site, &self.store.load(false)))
    }

    fn discover(&self, payload: &Value) -> AppResult<Value> {
        let settings = self.store.settings();
        let lease = self.operations.create()?;
        let host = self.read_text(payload, "host", MAX_ADDRESS_LENGTH)?;
        let outcome = self.with_lease(&lease, |_| {
            oauth_flow::discover(self.store, &self.net, &host, settings.timeout_ms, &lease)
        });
        self.operations.release(&lease);
        outcome.map(discovery_value)
    }

    fn probe_site(&self, payload: &Value) -> AppResult<Value> {
        let lease = self.operations.create()?;
        let site_id = self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?;
        let outcome = self.with_lease(&lease, |runner| runner.probe(&site_id, &lease));
        self.operations.release(&lease);
        let outcome = outcome?;
        Ok(json!({
            "state": outcome.state,
            "message": outcome.message,
            "httpStatus": outcome.http_status,
        }))
    }

    fn rename_site(&self, payload: &Value) -> AppResult<Value> {
        let site = self.store.update_site(
            &self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?,
            None,
            Some(&self.read_text(payload, "name", MAX_NAME_LENGTH)?),
        )?;
        Ok(state_view::of(&site, &self.store.load(false)))
    }

    fn remove_site(&self, payload: &Value) -> AppResult<Value> {
        let site_id = self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?;
        self.store.remove_site(&site_id)?;
        self.caches.invalidate(&site_id);
        Ok(Value::Bool(true))
    }

    fn save_credential(&self, payload: &Value) -> AppResult<Value> {
        let site_id = self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?;
        let group_id = self.read_text(payload, "groupId", model_rules::MAX_GROUP_ID_LENGTH)?;
        let credential = Credential {
            group_id: String::new(),
            credential_type: "token".to_string(),
            token: self.read_text(payload, "token", model_rules::MAX_TOKEN_LENGTH)?,
            user_id: bridge_json::int(payload, "userId", 0),
        };
        if !model_rules::is_configured(Some(&credential)) {
            return Err(AppError::validation("请填写访问令牌"));
        }
        let saved = self
            .store
            .save_credential_group(&site_id, credential, &group_id)?;
        Ok(json!({
            "groupId": saved.group_id,
            "type": saved.credential_type,
            "token": saved.token,
            "userId": saved.user_id,
        }))
    }

    fn remove_credential(&self, payload: &Value) -> AppResult<Value> {
        let site_id = self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?;
        let group_id = self.read_text(payload, "groupId", model_rules::MAX_GROUP_ID_LENGTH)?;
        self.store.remove_credential_group(&site_id, &group_id)?;
        Ok(Value::Bool(true))
    }

    fn clear_credential(&self, payload: &Value) -> AppResult<Value> {
        self.store
            .clear_credential(&self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?)?;
        Ok(Value::Bool(true))
    }

    fn verify_site(&self, payload: &Value) -> AppResult<Value> {
        let site_id = self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?;
        let group_id = self.read_text(payload, "groupId", model_rules::MAX_GROUP_ID_LENGTH)?;
        let Some(site) = self.store.find_site(&site_id) else {
            return Err(AppError::validation("站点不存在"));
        };
        let meta = self.store.meta_for(&site_id);
        let settings = self.store.settings();
        let mut credentials: Vec<Credential> = self
            .store
            .credential_groups_for(&site_id)
            .into_iter()
            .filter(|credential| model_rules::is_configured(Some(credential)))
            .collect();
        if !group_id.is_empty() {
            credentials.retain(|credential| credential.group_id == group_id);
        }
        if credentials.is_empty() {
            return Err(AppError::validation("请先保存访问令牌"));
        }
        let lease = self.operations.create()?;
        let engine = VerifyEngine::new(&self.net, &self.caches);
        let mut outcomes = Vec::new();
        let mut failure: Option<AppError> = None;
        for credential in &credentials {
            match engine.run(&site, credential, settings.timeout_ms, Some(&meta), &lease) {
                Ok(outcome) => {
                    if outcome.ok {
                        self.store.set_meta(
                            &site_id,
                            &MetaEntry {
                                account: if outcome.account.is_empty() {
                                    meta.account.clone()
                                } else {
                                    outcome.account.clone()
                                },
                                user_id: if outcome.user_id > 0 {
                                    outcome.user_id
                                } else {
                                    meta.user_id
                                },
                                ..MetaEntry::default()
                            },
                        )?;
                    }
                    outcomes.push(outcome);
                }
                Err(error) => {
                    if error.is_cancelled() {
                        failure = Some(error);
                        break;
                    }
                    outcomes.push(crate::core::verify::VerifyOutcome {
                        ok: false,
                        state: "failed".to_string(),
                        message: format!("校验异常：{}: {}", error.type_name(), error.message()),
                        ..Default::default()
                    });
                }
            }
        }
        self.operations.release(&lease);
        if let Some(error) = failure {
            return Err(error);
        }
        let first = outcomes.first().cloned().unwrap_or_default();
        let ok_count = outcomes.iter().filter(|item| item.ok).count();
        let all_ok = ok_count == outcomes.len();
        let message = if outcomes.len() == 1 {
            first.message.clone()
        } else {
            format!("体检完成：{ok_count}/{} 可用", outcomes.len())
        };
        Ok(json!({
            "ok": all_ok,
            "state": first.state,
            "message": message,
            "userId": first.user_id,
            "account": first.account,
            "groupId": if credentials.len() == 1 { credentials[0].group_id.clone() } else { String::new() },
        }))
    }

    fn verify_all(&self) -> AppResult<Value> {
        let lease = self.operations.create()?;
        let outcome = self.with_lease(&lease, |runner| runner.run_verify(&|_| {}, &lease));
        self.operations.release(&lease);
        let summary = outcome?;
        Ok(json!({
            "total": summary.total,
            "ok": summary.ok,
            "failed": summary.failed,
            "items": summary.items.iter().map(|item| json!({
                "id": item.id,
                "name": item.name,
                "host": item.host,
                "ok": item.ok,
                "message": item.message,
            })).collect::<Vec<Value>>(),
        }))
    }

    fn import_credentials(&self, payload: &Value) -> AppResult<Value> {
        let Some(entries) = payload.get("entries").and_then(|item| item.as_array()) else {
            return Err(AppError::validation("缺少导入内容"));
        };
        if entries.len() > model_rules::MAX_IMPORT_ENTRIES {
            return Err(AppError::validation(format!(
                "一次最多导入 {} 条凭据",
                model_rules::MAX_IMPORT_ENTRIES
            )));
        }
        let mut imports: Vec<CredentialImportEntry> = Vec::new();
        for node in entries {
            imports.push(CredentialImportEntry {
                name: self
                    .read_text(node, "name", MAX_NAME_LENGTH)?
                    .trim()
                    .to_string(),
                host: self
                    .read_text(node, "host", MAX_ADDRESS_LENGTH)?
                    .trim()
                    .to_string(),
                credential_type: self
                    .read_text(node, "type", MAX_SHORT_VALUE_LENGTH)?
                    .trim()
                    .to_string(),
                token: self.read_text(node, "token", model_rules::MAX_TOKEN_LENGTH)?,
                user_id: bridge_json::int(node, "userId", 0),
            });
        }
        let result = credential_import::apply(self.store, &imports)?;
        Ok(json!({
            "imported": result.imported,
            "addedSites": result.added_sites,
            "errors": result.errors,
        }))
    }

    fn run_checkin(&self, payload: &Value) -> AppResult<Value> {
        let site_id = self.read_text(payload, "siteId", MAX_SITE_ID_LENGTH)?;
        let lease = self.operations.create()?;
        self.progress_count.store(0, Ordering::SeqCst);
        let total = if site_id.is_empty() {
            self.store.credential_count(None)
        } else if self.store.find_site(&site_id).is_none() {
            0
        } else {
            self.store.credential_count(Some(&site_id))
        };
        let scope = if site_id.is_empty() {
            None
        } else {
            Some(site_id.as_str())
        };
        let outcome = self.with_lease(&lease, |runner| {
            runner.run_checkin(
                scope,
                &|result| self.report_progress(&result, total),
                &lease,
            )
        });
        self.operations.release(&lease);
        let summary = outcome?;
        let results: BTreeMap<String, Value> = summary
            .results
            .iter()
            .map(|(key, item)| (key.clone(), state_view::result_value(item)))
            .collect();
        Ok(json!({
            "total": summary.total,
            "elapsedMs": summary.elapsed_ms,
            "results": Value::Object(results.into_iter().collect()),
        }))
    }

    fn report_progress(&self, result: &crate::core::model::RunResult, total: i32) {
        let done = self.progress_count.fetch_add(1, Ordering::SeqCst) + 1;
        let event = json!({
            "type": "progress",
            "kind": "checkin",
            "done": done,
            "total": total,
            "label": format!("{done}/{total}"),
            "result": state_view::result_value(result),
        });
        (self.post_event)(event);
    }

    fn patch_settings(&self, payload: &Value) -> AppResult<Value> {
        self.store.patch_settings(self.read_settings(payload)?)?;
        Ok(state_view::build(self.store, false))
    }

    fn set_theme(&self, payload: &Value) -> AppResult<Value> {
        let mut current = self.store.settings();
        current.theme = self.read_text(payload, "theme", MAX_SHORT_VALUE_LENGTH)?;
        self.store.patch_settings(current)?;
        Ok(state_view::build(self.store, false))
    }

    fn read_settings(&self, payload: &Value) -> AppResult<Settings> {
        let current = self.store.settings();
        Ok(model_rules::normalize_settings(Some(&Settings {
            concurrency: bridge_json::int(payload, "concurrency", current.concurrency),
            timeout_ms: bridge_json::int(payload, "timeoutMs", current.timeout_ms),
            notify: bridge_json::boolean(payload, "notify", current.notify),
            skip_manual_only: bridge_json::boolean(
                payload,
                "skipManualOnly",
                current.skip_manual_only,
            ),
            theme: self
                .read_text(payload, "theme", MAX_SHORT_VALUE_LENGTH)
                .unwrap_or(current.theme),
            language: self
                .read_text(payload, "language", MAX_SHORT_VALUE_LENGTH)
                .unwrap_or(current.language),
        })))
    }

    fn with_lease<T>(
        &self,
        lease: &crate::core::cancel::CancelToken,
        action: impl FnOnce(&Runner<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        let _ = lease;
        let runner = Runner::new(self.store, &self.net, &self.caches);
        action(&runner)
    }

    fn read_text(&self, source: &Value, name: &str, maximum_length: usize) -> AppResult<String> {
        let value = bridge_json::text(source, name, "");
        if value.chars().count() > maximum_length {
            return Err(AppError::validation(format!("字段 {name} 超过长度上限")));
        }
        Ok(value)
    }
}

fn discovery_value(result: crate::core::oauth::DiscoveryResult) -> Value {
    json!({
        "ok": result.ok,
        "exists": result.exists,
        "host": result.host,
        "origin": result.origin,
        "siteId": result.site_id,
        "name": result.name,
        "version": result.version,
        "message": result.message,
        "checkinEnabled": result.checkin_enabled,
        "turnstile": result.turnstile,
        "passwordLogin": result.password_login,
        "oauth": result.oauth,
    })
}
