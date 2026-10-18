use std::collections::BTreeMap;

use super::auth;

use super::cancel::CancelToken;
use super::coerce;
use super::error::AppResult;
use super::model::{Credential, MetaEntry, Site, SiteSnapshot};
use super::net::NetClient;
use super::net_errors;
use super::protocol::{CheckinVerdict, ProtocolCaches};

pub const USER_HEADER: &str = "New-Api-User";
pub const PATH_BUDGET_FACTOR: f64 = 2.0;

#[derive(Debug, Clone, Default)]
pub struct CheckinOutcome {
    pub state: String,
    pub http_status: i32,
    pub message: String,
    pub quota: i64,
    pub checkin_date: String,
    pub path: String,
    pub meta: Option<SiteSnapshot>,
    pub user_id: i32,
    pub account: String,
}

pub struct CheckinEngine<'a> {
    net: &'a NetClient,
    caches: &'a ProtocolCaches,
}

impl<'a> CheckinEngine<'a> {
    pub fn new(net: &'a NetClient, caches: &'a ProtocolCaches) -> Self {
        Self { net, caches }
    }

    pub fn budget(&self, timeout_ms: i32, factor: f64) -> std::time::Duration {
        let ms = coerce::to_int(
            Some(&serde_json::Value::from(timeout_ms)),
            super::net::DEFAULT_TIMEOUT_MS,
            Some(500),
            Some(600_000),
        );
        let seconds = (ms as f64 / 1000.0).max(3.0) * factor;
        std::time::Duration::from_millis((seconds * 1000.0) as u64)
    }

    pub fn probe(
        &self,
        site: &Site,
        timeout_ms: i32,
        cached_meta: Option<&MetaEntry>,
        deadline: &CancelToken,
    ) -> AppResult<CheckinOutcome> {
        let status =
            super::protocol_async::fetch_status(self.net, site, timeout_ms, cached_meta, deadline)?;
        Ok(CheckinOutcome {
            state: status.state,
            message: status.message,
            meta: status.meta,
            http_status: status.http_status,
            ..CheckinOutcome::default()
        })
    }

    pub fn run(
        &self,
        site: &Site,
        credential: &Credential,
        timeout_ms: i32,
        cached_meta: Option<&MetaEntry>,
        outer: Option<&CancelToken>,
    ) -> AppResult<CheckinOutcome> {
        let mut outcome = CheckinOutcome::default();
        let budget = CancelToken::new();
        spawn_expiry(
            budget.clone(),
            self.budget(timeout_ms, PATH_BUDGET_FACTOR + 1.0),
        );
        let deadline = match outer {
            Some(parent) => CancelToken::link(&[&budget, parent]),
            None => budget.clone(),
        };

        let resolved = auth::resolve(site, credential, timeout_ms, &deadline)?;
        if !resolved.ok {
            outcome.state = resolved.state;
            outcome.message = resolved.message;
            return Ok(outcome);
        }
        let mut headers = auth::site_headers_with(&resolved.headers, site, "/");
        let status = super::protocol_async::fetch_status(
            self.net,
            site,
            timeout_ms,
            cached_meta,
            &deadline,
        )?;
        outcome.meta = status.meta;
        outcome.http_status = status.http_status;
        if matches!(
            status.state.as_str(),
            "offline" | "unsupported" | "disabled"
        ) {
            outcome.state = status.state;
            outcome.message = status.message;
            return Ok(outcome);
        }

        let identity =
            super::protocol_async::read_self(self.net, site, &headers, timeout_ms, &deadline)?;
        if matches!(
            identity.state.as_str(),
            "offline" | "challenge" | "unauthorized"
        ) {
            outcome.state = identity.state;
            outcome.message = identity.message;
            return Ok(outcome);
        }
        if identity.state == "success" {
            outcome.user_id = identity.user_id;
            outcome.account = identity.account;
        }
        resolve_user(&mut headers, &mut outcome, credential, cached_meta);

        let verdict = self.attempt_with_retry(
            site,
            headers,
            &mut outcome,
            cached_meta,
            timeout_ms,
            &deadline,
        )?;
        outcome.state = verdict.state;
        outcome.message = verdict.message;
        outcome.quota = coerce::to_long(
            Some(&serde_json::Value::from(verdict.quota)),
            0,
            Some(0),
            Some(i64::MAX),
        );
        outcome.checkin_date = verdict.date;
        outcome.path = verdict.path;
        outcome.http_status = coerce::to_int(
            Some(&serde_json::Value::from(verdict.http_status)),
            outcome.http_status,
            Some(0),
            Some(1000),
        );
        if outcome.account.is_empty() {
            outcome.account = cached_meta
                .map(|item| item.account.clone())
                .unwrap_or_default();
        }
        Ok(outcome)
    }

    fn attempt_with_retry(
        &self,
        site: &Site,
        mut headers: BTreeMap<String, String>,
        outcome: &mut CheckinOutcome,
        cached_meta: Option<&MetaEntry>,
        timeout_ms: i32,
        deadline: &CancelToken,
    ) -> AppResult<CheckinVerdict> {
        let budget = CancelToken::new();
        spawn_expiry(budget.clone(), self.budget(timeout_ms, PATH_BUDGET_FACTOR));
        let linked = CancelToken::link(&[&budget, deadline]);
        let verdict = self.attempt_paths(site, &headers, timeout_ms, &linked)?;
        let retryable = verdict.state == "unauthorized" && !headers.contains_key(USER_HEADER);
        if !retryable {
            return Ok(verdict);
        }
        let known = coerce::to_int(
            cached_meta
                .map(|item| serde_json::Value::from(item.user_id))
                .as_ref(),
            0,
            Some(0),
            Some(super::model_rules::MAX_USER_ID),
        );
        if known <= 0 {
            return Ok(verdict);
        }
        headers.insert(USER_HEADER.to_string(), known.to_string());
        outcome.user_id = known;
        let retry_budget = CancelToken::new();
        spawn_expiry(
            retry_budget.clone(),
            self.budget(timeout_ms, PATH_BUDGET_FACTOR),
        );
        let retry_linked = CancelToken::link(&[&retry_budget, deadline]);
        let retry = self.attempt_paths(site, &headers, timeout_ms, &retry_linked)?;
        if retry.state == "unauthorized" || retry.state == "failed" {
            Ok(verdict)
        } else {
            Ok(retry)
        }
    }

    fn attempt_paths(
        &self,
        site: &Site,
        headers: &BTreeMap<String, String>,
        timeout_ms: i32,
        deadline: &CancelToken,
    ) -> AppResult<CheckinVerdict> {
        let mut skipped: Option<CheckinVerdict> = None;
        for path in self.caches.ordered_paths(&site.id) {
            if deadline.is_cancelled() {
                return Ok(CheckinVerdict {
                    state: "failed".to_string(),
                    message: "已用完本次签到的时间预算".to_string(),
                    path,
                    ..CheckinVerdict::default()
                });
            }
            let url = format!("{}{}", site.origin, path);
            let response = match self
                .net
                .post(&url, timeout_ms, Some(headers), None, deadline)
            {
                Ok(value) => value,
                Err(error) => {
                    if error.is_cancelled() {
                        return Err(error);
                    }
                    let fault = error.fault().cloned().unwrap_or_default();
                    return Ok(CheckinVerdict {
                        state: net_errors::offline_state(&fault),
                        message: fault.message,
                        path,
                        ..CheckinVerdict::default()
                    });
                }
            };
            let status = response.status;
            if matches!(status, 404 | 405 | 501) {
                continue;
            }
            let mut verdict = super::protocol_interpret::interpret(status, &response);
            verdict.path = path.clone();
            verdict.http_status = status;
            if verdict.state == "success" || verdict.state == "already" {
                self.caches.remember_path(&site.id, &path);
                return Ok(verdict);
            }
            if verdict.state != "skip" {
                return Ok(verdict);
            }
            if skipped.is_none() {
                skipped = Some(verdict);
            }
        }
        if let Some(value) = skipped {
            return Ok(value);
        }
        Ok(CheckinVerdict {
            state: "unsupported".to_string(),
            message: "未找到可用的签到接口".to_string(),
            ..CheckinVerdict::default()
        })
    }
}

impl Default for super::error::NetworkFault {
    fn default() -> Self {
        Self {
            message: String::new(),
            kind: "unknown".to_string(),
            retryable: false,
        }
    }
}

fn spawn_expiry(token: CancelToken, duration: std::time::Duration) {
    std::thread::spawn(move || {
        let step = std::time::Duration::from_millis(25);
        let mut remaining = duration;
        while !remaining.is_zero() {
            if token.is_cancelled() {
                return;
            }
            let sleep = remaining.min(step);
            std::thread::sleep(sleep);
            remaining = remaining.saturating_sub(sleep);
        }
        token.cancel();
    });
}

fn resolve_user(
    headers: &mut BTreeMap<String, String>,
    outcome: &mut CheckinOutcome,
    credential: &Credential,
    cached_meta: Option<&MetaEntry>,
) {
    if outcome.user_id > 0 {
        headers.insert(USER_HEADER.to_string(), outcome.user_id.to_string());
        return;
    }
    let manual = coerce::to_int(
        Some(&serde_json::Value::from(credential.user_id)),
        0,
        Some(0),
        Some(super::model_rules::MAX_USER_ID),
    );
    let known = coerce::to_int(
        cached_meta
            .map(|item| serde_json::Value::from(item.user_id))
            .as_ref(),
        0,
        Some(0),
        Some(super::model_rules::MAX_USER_ID),
    );
    let candidate = if manual > 0 { manual } else { known };
    if candidate > 0 {
        headers.insert(USER_HEADER.to_string(), candidate.to_string());
        outcome.user_id = candidate;
    }
}
