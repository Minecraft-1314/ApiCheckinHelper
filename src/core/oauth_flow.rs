use serde_json::Value;

use super::auth;
use super::cancel::CancelToken;
use super::config_store::ConfigStore;
use super::error::AppResult;
use super::json;
use super::model::Site;
use super::model_rules;
use super::net::NetClient;
use super::oauth::{self, DiscoveryResult, OAuthFlowResult, OAuthToken};
use super::protocol;

pub fn status_data(
    net: &NetClient,
    site: &Site,
    timeout_ms: i32,
    deadline: &CancelToken,
) -> AppResult<Option<Value>> {
    for origin in protocol::candidate_origins(site) {
        let url = format!("{origin}/api/status");
        let response = match net.get(&url, timeout_ms, None, deadline) {
            Ok(value) => value,
            Err(error) => {
                if error.is_cancelled() {
                    return Err(error);
                }
                continue;
            }
        };
        if response.status < 400 {
            if let Some(data) = protocol::read_data(response.data.as_ref()) {
                return Ok(Some(Value::Object(data.clone())));
            }
        }
    }
    Ok(None)
}

pub fn start(
    net: &NetClient,
    site: &Site,
    slug: &str,
    timeout_ms: i32,
    deadline: &CancelToken,
) -> AppResult<OAuthFlowResult> {
    let Some(data) = status_data(net, site, timeout_ms, deadline)? else {
        return Ok(OAuthFlowResult {
            ok: false,
            state: "offline".to_string(),
            message: "站点不可用或不是 New API 面板".to_string(),
            ..OAuthFlowResult::default()
        });
    };
    let provider = oauth::read_providers(Some(&data))
        .into_iter()
        .find(|item| item.slug == slug);
    let Some(provider) = provider else {
        return Ok(OAuthFlowResult {
            ok: false,
            state: "disabled".to_string(),
            message: "该站点未开启这个登录方式".to_string(),
            ..OAuthFlowResult::default()
        });
    };
    let headers = auth::site_headers(site, "/login");
    let body = serde_json::json!({ "provider": slug, "intent": "login" });
    let url = format!("{}/api/oauth/state", site.origin);
    let response = match net.post(&url, timeout_ms, Some(&headers), Some(&body), deadline) {
        Ok(value) => value,
        Err(error) => {
            if error.is_cancelled() {
                return Err(error);
            }
            let message = error
                .fault()
                .map(|fault| fault.message.clone())
                .unwrap_or_else(|| error.message());
            return Ok(OAuthFlowResult {
                ok: false,
                state: "offline".to_string(),
                message,
                ..OAuthFlowResult::default()
            });
        }
    };
    if response.status == 404 || response.status == 405 {
        return Ok(OAuthFlowResult {
            ok: false,
            state: "unsupported".to_string(),
            message: "该站点版本较旧，没有可用的 OAuth 授权接口".to_string(),
            ..OAuthFlowResult::default()
        });
    }
    let res_data = protocol::read_data(response.data.as_ref()).cloned();
    let res_value = res_data.clone().map(Value::Object);
    let flow_token = json::text(res_value.as_ref(), &["flow_token"]);
    if flow_token.is_empty() {
        return Ok(OAuthFlowResult {
            ok: false,
            state: "failed".to_string(),
            message: protocol::first_non_empty(&[
                &protocol::read_message(response.data.as_ref()),
                "未能获取授权状态",
            ]),
            ..OAuthFlowResult::default()
        });
    }
    let mut authorize = json::text(res_value.as_ref(), &["authorization_url"]);
    if authorize.is_empty() {
        authorize = oauth::build_authorize_url(site, &provider, &flow_token, Some(&data));
    }
    if authorize.is_empty() {
        return Ok(OAuthFlowResult {
            ok: false,
            state: "failed".to_string(),
            message: "这个登录方式缺少可用的授权地址，请改用访问令牌或 Cookie".to_string(),
            ..OAuthFlowResult::default()
        });
    }
    Ok(OAuthFlowResult {
        ok: true,
        state: "success".to_string(),
        message: String::new(),
        name: provider.name.clone(),
        slug: slug.to_string(),
        flow_token,
        expires_at: json::long(res_value.as_ref(), "expires_at", 0),
        authorize_url: authorize,
        redirect_uri: format!("{}/oauth/{}", site.origin, slug),
    })
}

pub fn parse_callback(text: Option<&str>) -> Option<(String, String)> {
    let raw = text.unwrap_or("").trim().to_string();
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with('{') {
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            return None;
        };
        let code = json::text(Some(&value), &["code"]);
        if code.is_empty() {
            return None;
        }
        return Some((code, json::text(Some(&value), &["state"])));
    }
    let query = if let Some(index) = raw.find('?') {
        raw[index + 1..].to_string()
    } else if raw.starts_with("code=") || raw.starts_with("state=") {
        raw.clone()
    } else {
        return None;
    };
    let mut code = String::new();
    let mut state = String::new();
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "code" => code = oauth::url_decode(value),
            "state" => state = oauth::url_decode(value),
            _ => {}
        }
    }
    if code.is_empty() {
        return None;
    }
    Some((code, state))
}

fn fail(message: &str) -> OAuthToken {
    OAuthToken {
        ok: false,
        state: "failed".to_string(),
        message: message.to_string(),
        ..OAuthToken::default()
    }
}

pub fn finish(
    net: &NetClient,
    site: &Site,
    slug: &str,
    text: Option<&str>,
    expected_state: &str,
    timeout_ms: i32,
    deadline: &CancelToken,
) -> AppResult<OAuthToken> {
    let Some((code, state)) = parse_callback(text) else {
        return Ok(fail(
            "没识别出授权码，请把地址栏里带 code 参数的整段地址粘进来",
        ));
    };
    if state.is_empty() {
        return Ok(fail("地址里缺少 state 参数，请确认复制的是完整的回调地址"));
    }
    if !expected_state.is_empty() && state != expected_state {
        return Ok(fail("state 与本次发起的授权不一致，请重新从本窗口发起一次"));
    }
    let query = format!(
        "code={}&state={}",
        oauth::url_encode(&code),
        oauth::url_encode(&state)
    );
    let url = format!(
        "{}/api/oauth/{}?{}",
        site.origin,
        oauth::url_encode(slug),
        query
    );
    let headers = auth::site_headers(site, &format!("/oauth/{slug}"));
    let response = match net.get(&url, timeout_ms, Some(&headers), deadline) {
        Ok(value) => value,
        Err(error) => {
            if error.is_cancelled() {
                return Err(error);
            }
            let message = error
                .fault()
                .map(|fault| fault.message.clone())
                .unwrap_or_else(|| error.message());
            return Ok(OAuthToken {
                ok: false,
                state: "offline".to_string(),
                message,
                ..OAuthToken::default()
            });
        }
    };
    let data = protocol::read_data(response.data.as_ref()).cloned();
    let data_value = data.clone().map(Value::Object);
    let token = json::text(data_value.as_ref(), &["access_token"]);
    if token.is_empty() {
        return Ok(OAuthToken {
            ok: false,
            state: "unauthorized".to_string(),
            message: protocol::first_non_empty(&[
                &protocol::read_message(response.data.as_ref()),
                "站点未返回访问令牌",
            ]),
            ..OAuthToken::default()
        });
    }
    let refresh = response
        .cookie(protocol::REFRESH_COOKIE)
        .or_else(|| response.cookie("session"))
        .unwrap_or_default();
    let user = json::get(data_value.as_ref(), &["user"]).cloned();
    let account = user
        .as_ref()
        .map(|value| {
            protocol::first_non_empty(&[
                &json::text(Some(value), &["display_name"]),
                &json::text(Some(value), &["username"]),
            ])
        })
        .unwrap_or_default();
    Ok(OAuthToken {
        ok: true,
        state: "success".to_string(),
        access_token: token,
        access_expires_at: json::long(data_value.as_ref(), "access_expires_at", 0),
        refresh_cookie: refresh,
        account,
        message: String::new(),
    })
}

pub fn discover(
    store: &ConfigStore,
    net: &NetClient,
    url: &str,
    timeout_ms: i32,
    deadline: &CancelToken,
) -> AppResult<DiscoveryResult> {
    let draft = Site {
        host: url.to_string(),
        ..Site::default()
    };
    let Some(site) = model_rules::normalize_site(Some(&draft)) else {
        return Ok(DiscoveryResult {
            ok: false,
            message: "地址格式不合法，请填写形如 api.example.com 的域名".to_string(),
            ..DiscoveryResult::default()
        });
    };
    if let Some(existing) = store.find_by_host(&site.host) {
        return Ok(DiscoveryResult {
            ok: true,
            exists: true,
            host: existing.host.clone(),
            site_id: existing.id.clone(),
            name: existing.name.clone(),
            ..DiscoveryResult::default()
        });
    }
    let Some(data) = status_data(net, &site, timeout_ms, deadline)? else {
        return Ok(DiscoveryResult {
            ok: false,
            host: site.host.clone(),
            message: "访问不到这个地址，或者它不是 New API 面板".to_string(),
            ..DiscoveryResult::default()
        });
    };
    let names: Vec<String> = oauth::read_providers(Some(&data))
        .into_iter()
        .map(|item| item.name)
        .collect();
    let password_login = !json::get(Some(&data), &["password_login_enabled"]).is_some()
        || json::boolean(Some(&data), &["password_login_enabled"]);
    Ok(DiscoveryResult {
        ok: true,
        exists: false,
        host: site.host.clone(),
        origin: site.origin.clone(),
        name: protocol::first_non_empty(&[&json::text(Some(&data), &["system_name"]), &site.host]),
        version: json::text(Some(&data), &["version"]),
        message: String::new(),
        checkin_enabled: json::boolean(Some(&data), &["checkin_enabled"]),
        turnstile: json::boolean(Some(&data), &["turnstile_check"]),
        password_login,
        oauth: names,
        site_id: String::new(),
    })
}
