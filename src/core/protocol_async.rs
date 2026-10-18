use std::collections::BTreeMap;

use serde_json::Value;

use super::cancel::CancelToken;
use super::error::AppResult;
use super::json;
use super::model::{MetaEntry, Site, SiteSnapshot};
use super::net::NetClient;
use super::protocol::{self, IdentityResult, StatusResult};

pub fn from_meta(meta: Option<SiteSnapshot>, origin: &str, http_status: i32) -> StatusResult {
    let Some(meta) = meta else {
        return StatusResult {
            state: "unsupported".to_string(),
            message: "该站点不是 New API 面板".to_string(),
            origin: origin.to_string(),
            http_status,
            meta: None,
        };
    };
    if meta.checkin_enabled == Some(false) {
        return StatusResult {
            state: "disabled".to_string(),
            message: "站点未开启签到功能".to_string(),
            origin: origin.to_string(),
            http_status,
            meta: Some(meta),
        };
    }
    let message = if meta.turnstile {
        "需人机验证，仅能手动完成".to_string()
    } else {
        "可签到".to_string()
    };
    StatusResult {
        state: "idle".to_string(),
        message,
        origin: origin.to_string(),
        http_status,
        meta: Some(meta),
    }
}

pub fn fetch_status(
    net: &NetClient,
    site: &Site,
    timeout_ms: i32,
    cached_meta: Option<&MetaEntry>,
    deadline: &CancelToken,
) -> AppResult<StatusResult> {
    let fresh = cached_meta
        .map(|item| item.probed_at > 0 && now_ms() - item.probed_at < protocol::STATUS_TTL_MS)
        .unwrap_or(false);
    if fresh {
        if let Some(view) = protocol::build_meta_view(cached_meta) {
            return Ok(from_meta(Some(view), &site.origin, 200));
        }
    }
    let mut result = StatusResult {
        state: "offline".to_string(),
        message: "无法连接站点".to_string(),
        ..StatusResult::default()
    };
    for origin in protocol::candidate_origins(site) {
        result = fetch_status_once(net, &origin, timeout_ms, deadline)?;
        let done = result.state != "offline";
        let expired = deadline.is_cancelled();
        if done || expired {
            break;
        }
    }
    Ok(result)
}

fn fetch_status_once(
    net: &NetClient,
    origin: &str,
    timeout_ms: i32,
    deadline: &CancelToken,
) -> AppResult<StatusResult> {
    let url = format!("{origin}/api/status");
    let response = match net.get(&url, timeout_ms, None, deadline) {
        Ok(value) => value,
        Err(error) => {
            if error.is_cancelled() {
                return Err(error);
            }
            let message = error
                .fault()
                .map(|fault| fault.message.clone())
                .unwrap_or_else(|| error.message());
            return Ok(StatusResult {
                state: "offline".to_string(),
                message,
                ..StatusResult::default()
            });
        }
    };
    if response.status >= 500 {
        return Ok(StatusResult {
            state: "offline".to_string(),
            message: format!("站点返回 {}", response.status),
            http_status: response.status,
            origin: origin.to_string(),
            meta: None,
        });
    }
    if response.status >= 400 {
        return Ok(StatusResult {
            state: "offline".to_string(),
            message: format!("站点不可用（HTTP {}）", response.status),
            http_status: response.status,
            origin: origin.to_string(),
            meta: None,
        });
    }
    match protocol::read_meta(response.data.as_ref()) {
        None => Ok(StatusResult {
            state: "unsupported".to_string(),
            message: "该站点不是 New API 面板".to_string(),
            http_status: response.status,
            origin: origin.to_string(),
            meta: None,
        }),
        Some(meta) => Ok(from_meta(Some(meta), origin, response.status)),
    }
}

pub fn interpret_self_status(status: i32, payload: Option<&Value>) -> Option<IdentityResult> {
    if status == 404 {
        return None;
    }
    if status == 401 {
        return Some(IdentityResult {
            state: "unauthorized".to_string(),
            message: "访问令牌无效或已过期".to_string(),
            ..IdentityResult::default()
        });
    }
    if status == 403 {
        return Some(IdentityResult {
            state: "challenge".to_string(),
            message: "站点风控拦截，令牌请求未被接受".to_string(),
            ..IdentityResult::default()
        });
    }
    if status >= 500 {
        return Some(IdentityResult {
            state: "offline".to_string(),
            message: format!("站点返回 {status}"),
            ..IdentityResult::default()
        });
    }
    let message = protocol::read_message(payload);
    let code = protocol::read_code(payload);
    let unauthorized = protocol::read_success(payload) == Some(false)
        && (protocol::auth_pattern().is_match(&message) || code == "AUTH_UNAUTHORIZED");
    if unauthorized {
        return Some(IdentityResult {
            state: "unauthorized".to_string(),
            message: if message.is_empty() {
                "访问令牌无效或已过期".to_string()
            } else {
                message
            },
            ..IdentityResult::default()
        });
    }
    None
}

pub fn read_self(
    net: &NetClient,
    site: &Site,
    headers: &BTreeMap<String, String>,
    timeout_ms: i32,
    deadline: &CancelToken,
) -> AppResult<IdentityResult> {
    for path in protocol::SELF_PATHS {
        let url = format!("{}{}", site.origin, path);
        let response = match net.get(&url, timeout_ms, Some(headers), deadline) {
            Ok(value) => value,
            Err(error) => {
                if error.is_cancelled() {
                    return Err(error);
                }
                let message = error
                    .fault()
                    .map(|fault| fault.message.clone())
                    .unwrap_or_else(|| error.message());
                return Ok(IdentityResult {
                    state: "offline".to_string(),
                    message,
                    ..IdentityResult::default()
                });
            }
        };
        if let Some(short_circuit) = interpret_self_status(response.status, response.data.as_ref())
        {
            return Ok(short_circuit);
        }
        if response.status == 404 {
            continue;
        }
        let Some(data) = protocol::read_data(response.data.as_ref()) else {
            continue;
        };
        let value = Value::Object(data.clone());
        return Ok(IdentityResult {
            state: "success".to_string(),
            message: "凭据可用".to_string(),
            user_id: json::int(Some(&value), "id", 0, 0, 1_000_000_000),
            account: protocol::first_non_empty(&[
                &json::text(Some(&value), &["display_name"]),
                &json::text(Some(&value), &["username"]),
            ]),
        });
    }
    Ok(IdentityResult {
        state: "unsupported".to_string(),
        message: "旧版本站点没有可用的用户信息接口".to_string(),
        ..IdentityResult::default()
    })
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|item| item.as_millis() as i64)
        .unwrap_or(0)
}
