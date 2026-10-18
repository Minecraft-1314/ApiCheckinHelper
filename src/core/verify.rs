use super::auth;
use super::cancel::CancelToken;
use super::coerce;
use super::error::AppResult;
use super::model::{Credential, MetaEntry, Site};
use super::net::NetClient;
use super::protocol::ProtocolCaches;
use super::protocol_async;

pub const MAX_MESSAGE: usize = 60;

#[derive(Debug, Clone, Default)]
pub struct VerifyOutcome {
    pub ok: bool,
    pub state: String,
    pub message: String,
    pub user_id: i32,
    pub account: String,
}

pub struct VerifyEngine<'a> {
    net: &'a NetClient,
    _caches: &'a ProtocolCaches,
}

impl<'a> VerifyEngine<'a> {
    pub fn new(net: &'a NetClient, caches: &'a ProtocolCaches) -> Self {
        Self {
            net,
            _caches: caches,
        }
    }

    pub fn run(
        &self,
        site: &Site,
        credential: &Credential,
        timeout_ms: i32,
        cached_meta: Option<&MetaEntry>,
        deadline: &CancelToken,
    ) -> AppResult<VerifyOutcome> {
        let resolved = auth::resolve(site, credential, timeout_ms, deadline)?;
        if !resolved.ok {
            return Ok(VerifyOutcome {
                ok: false,
                state: resolved.state,
                message: resolved.message,
                ..VerifyOutcome::default()
            });
        }
        let mut headers = resolved.headers.clone();
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
            headers.insert(
                super::checkin::USER_HEADER.to_string(),
                candidate.to_string(),
            );
        }

        let identity = protocol_async::read_self(self.net, site, &headers, timeout_ms, deadline)?;
        if identity.state == "success" {
            return Ok(VerifyOutcome {
                ok: true,
                state: "success".to_string(),
                message: "凭据可用".to_string(),
                user_id: identity.user_id,
                account: identity.account,
            });
        }
        let mut state = identity.state;
        let mut message = identity.message;
        if state == "unsupported" {
            let status =
                protocol_async::fetch_status(self.net, site, timeout_ms, cached_meta, deadline)?;
            state = if status.state == "unsupported" || status.state == "offline" {
                status.state
            } else {
                "unsupported".to_string()
            };
            if state == "unsupported" {
                message = "凭据没被拒绝，但站点读不出用户信息，请手动确认用户 ID".to_string();
            }
        }
        let trimmed: String = message.chars().take(MAX_MESSAGE).collect();
        Ok(VerifyOutcome {
            ok: false,
            state,
            message: trimmed,
            ..VerifyOutcome::default()
        })
    }
}
