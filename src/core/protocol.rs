use std::sync::Mutex;

use regex::Regex;
use serde_json::Value;

use super::coerce;
use super::json;
use super::model::{Site, SiteSnapshot};
use super::model_rules;

pub const STATUS_TTL_MS: i64 = 15 * 60 * 1000;
pub const REFRESH_COOKIE: &str = "new_api_refresh";
pub const EXPIRY_MARGIN_MS: i64 = 60_000;
pub const DEFAULT_QUOTA_PER_UNIT: i32 = 500_000;

pub const CHECKIN_PATHS: [&str; 6] = [
    "/api/user/checkin",
    "/api/user/clock_in",
    "/api/user/check_in",
    "/api/user/sign_in",
    "/api/user/daily_checkin",
    "/api/user/attendance",
];

pub const SELF_PATHS: [&str; 2] = ["/api/user/self", "/api/user/token"];

pub const MESSAGE_KEYS: [&str; 4] = ["message", "msg", "error", "detail"];

#[derive(Debug, Clone, PartialEq)]
pub struct StatusResult {
    pub state: String,
    pub message: String,
    pub meta: Option<SiteSnapshot>,
    pub http_status: i32,
    pub origin: String,
}

impl Default for StatusResult {
    fn default() -> Self {
        Self {
            state: "offline".to_string(),
            message: String::new(),
            meta: None,
            http_status: 0,
            origin: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdentityResult {
    pub state: String,
    pub message: String,
    pub user_id: i32,
    pub account: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CheckinVerdict {
    pub state: String,
    pub message: String,
    pub quota: i64,
    pub date: String,
    pub path: String,
    pub http_status: i32,
}

pub struct ProtocolCaches {
    gate: Mutex<std::collections::HashMap<String, String>>,
}

impl ProtocolCaches {
    pub fn new() -> Self {
        Self {
            gate: Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn reset(&self) {
        if let Ok(mut cache) = self.gate.lock() {
            cache.clear();
        }
    }

    pub fn invalidate(&self, site_id: &str) {
        if let Ok(mut cache) = self.gate.lock() {
            cache.remove(site_id);
        }
    }

    pub fn remember_path(&self, site_id: &str, path: &str) {
        if let Ok(mut cache) = self.gate.lock() {
            cache.insert(site_id.to_string(), path.to_string());
        }
    }

    pub fn ordered_paths(&self, site_id: &str) -> Vec<String> {
        let learned = self
            .gate
            .lock()
            .ok()
            .and_then(|cache| cache.get(site_id).cloned());
        match learned {
            None => CHECKIN_PATHS.iter().map(|item| item.to_string()).collect(),
            Some(path) => {
                let mut result = vec![path.clone()];
                for candidate in CHECKIN_PATHS {
                    if candidate != path {
                        result.push(candidate.to_string());
                    }
                }
                result
            }
        }
    }
}

impl Default for ProtocolCaches {
    fn default() -> Self {
        Self::new()
    }
}

pub fn candidate_origins(site: &Site) -> Vec<String> {
    let alternate = if site.origin.starts_with("https://") {
        format!("http://{}", model_rules::format_host(&site.host))
    } else {
        format!("https://{}", model_rules::format_host(&site.host))
    };
    vec![site.origin.clone(), alternate]
}

pub fn first_non_empty(values: &[&str]) -> String {
    for value in values {
        if !value.is_empty() {
            return (*value).to_string();
        }
    }
    String::new()
}

pub fn read_message(node: Option<&Value>) -> String {
    let Some(map) = node.and_then(|item| item.as_object()) else {
        return String::new();
    };
    for key in MESSAGE_KEYS {
        if let Some(Value::String(text)) = map.get(key) {
            if !text.is_empty() {
                return text.clone();
            }
        }
    }
    String::new()
}

pub fn read_success(node: Option<&Value>) -> Option<bool> {
    let map = node.and_then(|item| item.as_object())?;
    if let Some(flag) = map.get("success").and_then(|item| item.as_bool()) {
        return Some(flag);
    }
    if let Some(code) = map.get("code") {
        if let Some(flag) = code.as_bool() {
            return Some(!flag);
        }
        if let Some(number) = code.as_i64() {
            return Some(number == 0);
        }
        if let Some(text) = code.as_str() {
            if let Ok(number) = text.trim().parse::<i64>() {
                return Some(number == 0);
            }
        }
    }
    None
}

pub fn read_data(node: Option<&Value>) -> Option<&serde_json::Map<String, Value>> {
    json::object(node, &["data"])
}

pub fn read_code(node: Option<&Value>) -> String {
    json::text(node, &["code"])
}

pub fn normalize_display_type(raw: &str) -> String {
    let text = coerce::to_text(Some(&Value::String(raw.to_string())), "USD")
        .trim()
        .to_ascii_uppercase();
    if text.is_empty() {
        "USD".to_string()
    } else {
        text.chars().take(12).collect()
    }
}

pub fn read_meta(node: Option<&Value>) -> Option<SiteSnapshot> {
    let data = read_data(node)?;
    let data_value = Value::Object(data.clone());
    let version = json::text(Some(&data_value), &["version"]);
    Some(SiteSnapshot {
        system_name: json::text(Some(&data_value), &["system_name"]),
        version: if version.is_empty() {
            None
        } else {
            Some(version)
        },
        checkin_enabled: json::bool_or_null(Some(&data_value), &["checkin_enabled"]),
        turnstile: json::boolean(Some(&data_value), &["turnstile_check"]),
        quota_per_unit: json::int(
            Some(&data_value),
            "quota_per_unit",
            DEFAULT_QUOTA_PER_UNIT,
            1,
            i32::MAX,
        ),
        display_in_currency: !data.contains_key("display_in_currency")
            || json::boolean(Some(&data_value), &["display_in_currency"]),
        display_type: normalize_display_type(&json::text(
            Some(&data_value),
            &["quota_display_type"],
        )),
        exchange_rate: json::double(
            Some(&data_value),
            "usd_exchange_rate",
            1.0,
            0.000_001,
            1_000_000.0,
        ),
    })
}

pub fn build_meta_view(info: Option<&super::model::MetaEntry>) -> Option<SiteSnapshot> {
    let info = info?;
    let version = info.version.clone();
    Some(SiteSnapshot {
        system_name: info.system_name.clone(),
        version: if version.is_empty() {
            None
        } else {
            Some(version)
        },
        checkin_enabled: info.checkin_enabled,
        turnstile: info.turnstile,
        quota_per_unit: if info.quota_per_unit > 0 {
            info.quota_per_unit
        } else {
            DEFAULT_QUOTA_PER_UNIT
        },
        display_in_currency: info.display_in_currency,
        display_type: normalize_display_type(&info.display_type),
        exchange_rate: coerce::to_float(
            Some(&Value::from(info.exchange_rate)),
            1.0,
            Some(0.000_001),
            Some(1_000_000.0),
        ),
    })
}

pub fn needs_token_pattern() -> Regex {
    Regex::new(r"(?i)turnstile|人机验证|请完成验证").expect("needs token pattern")
}

pub fn already_pattern() -> Regex {
    Regex::new(r"(?i)已签到|已打卡|重复签到|已经签到|already").expect("already pattern")
}

pub fn disabled_pattern() -> Regex {
    Regex::new(r"(?i)未启用|未开启|签到功能关闭|not enabled|disabled").expect("disabled pattern")
}

pub fn auth_pattern() -> Regex {
    Regex::new(r"(?i)未登录|无权|登录已过期|请先登录|未授权|unauthorized|not logged in|invalid access token")
        .expect("auth pattern")
}

pub fn missing_endpoint_pattern() -> Regex {
    Regex::new(r"(?i)未找到|不存在|not found|no such").expect("missing endpoint pattern")
}
