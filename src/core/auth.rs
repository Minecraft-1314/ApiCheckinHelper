use std::collections::BTreeMap;

use super::cancel::CancelToken;
use super::coerce;
use super::model::{Credential, Site};

#[derive(Debug, Clone, Default)]
pub struct AuthResult {
    pub ok: bool,
    pub state: String,
    pub message: String,
    pub headers: BTreeMap<String, String>,
}

impl AuthResult {
    pub fn fail(state: &str, message: &str) -> Self {
        Self {
            ok: false,
            state: state.to_string(),
            message: message.to_string(),
            headers: BTreeMap::new(),
        }
    }

    pub fn bearer(token: &str) -> Self {
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {token}"));
        Self {
            ok: true,
            state: "success".to_string(),
            message: "凭据可用".to_string(),
            headers,
        }
    }
}

pub fn strip_bearer(value: &str) -> String {
    let text = value.trim();
    if text.len() >= 7 && text[..7].eq_ignore_ascii_case("Bearer ") {
        text[7..].trim().to_string()
    } else {
        text.to_string()
    }
}

pub fn site_headers(site: &Site, path: &str) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    headers.insert("Accept".to_string(), "application/json".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Origin".to_string(), site.origin.clone());
    headers.insert("Referer".to_string(), format!("{}{}", site.origin, path));
    headers
}

pub fn site_headers_with(
    base: &BTreeMap<String, String>,
    site: &Site,
    path: &str,
) -> BTreeMap<String, String> {
    let mut headers = base.clone();
    let extra = site_headers(site, path);
    for (key, value) in extra {
        headers.entry(key).or_insert(value);
    }
    headers
}

pub fn resolve(
    _site: &Site,
    credential: &Credential,
    _timeout_ms: i32,
    _deadline: &CancelToken,
) -> Result<AuthResult, super::error::AppError> {
    let token = strip_bearer(&credential.token);
    if token.is_empty() {
        Ok(AuthResult::fail("unauthorized", "未配置访问令牌"))
    } else {
        Ok(AuthResult::bearer(&token))
    }
}

pub fn header_value(headers: &BTreeMap<String, String>, name: &str) -> String {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
        .unwrap_or_default()
}

pub fn to_text(value: Option<&serde_json::Value>) -> String {
    coerce::to_text(value, "")
}
