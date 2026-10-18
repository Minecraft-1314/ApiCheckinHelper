use serde_json::Value;

use super::coerce;
use super::json;
use super::net::NetResponse;
use super::protocol::{self, CheckinVerdict};

pub fn pick_quota_node(
    data: Option<&serde_json::Map<String, Value>>,
    payload: Option<&Value>,
) -> Option<Value> {
    if let Some(map) = data {
        if let Some(value) = map.get("quota_awarded") {
            return Some(value.clone());
        }
        if let Some(value) = map.get("quota") {
            return Some(value.clone());
        }
    }
    json::get(payload, &["quota"]).cloned()
}

pub fn interpret(status: i32, response: &NetResponse) -> CheckinVerdict {
    let payload = response.data.as_ref();
    if status == 401 {
        return CheckinVerdict {
            state: "unauthorized".to_string(),
            message: "访问令牌无效或已过期".to_string(),
            ..CheckinVerdict::default()
        };
    }
    if status == 403 {
        return CheckinVerdict {
            state: "challenge".to_string(),
            message: "被站点风控拦截（403）".to_string(),
            ..CheckinVerdict::default()
        };
    }
    if status == 429 {
        return CheckinVerdict {
            state: "failed".to_string(),
            message: "请求过于频繁（429），稍后再试".to_string(),
            ..CheckinVerdict::default()
        };
    }
    if status >= 500 {
        return CheckinVerdict {
            state: "failed".to_string(),
            message: format!("服务端错误（{status}）"),
            ..CheckinVerdict::default()
        };
    }
    let Some(payload) = payload else {
        let html = status == 200 && response.looks_html();
        return CheckinVerdict {
            state: if html {
                "unsupported".to_string()
            } else {
                "failed".to_string()
            },
            message: if html {
                "该站点不是 New API 面板".to_string()
            } else {
                format!("无法解析响应（HTTP {status}）")
            },
            ..CheckinVerdict::default()
        };
    };
    let text = protocol::read_message(Some(payload));
    if protocol::read_success(Some(payload)) == Some(true) {
        let data = protocol::read_data(Some(payload));
        let quota = pick_quota_node(data, Some(payload));
        let date = data
            .map(|map| json::text(Some(&Value::Object(map.clone())), &["checkin_date"]))
            .unwrap_or_default();
        return CheckinVerdict {
            state: "success".to_string(),
            message: if text.is_empty() {
                "签到成功".to_string()
            } else {
                text
            },
            quota: coerce::to_long(quota.as_ref(), 0, Some(0), Some(i64::MAX)),
            date,
            ..CheckinVerdict::default()
        };
    }
    if protocol::needs_token_pattern().is_match(&text) {
        return CheckinVerdict {
            state: "challenge".to_string(),
            message: protocol::first_non_empty(&[&text, "该站点要求人机验证，无法自动完成"]),
            ..CheckinVerdict::default()
        };
    }
    if protocol::already_pattern().is_match(&text) {
        return CheckinVerdict {
            state: "already".to_string(),
            message: protocol::first_non_empty(&[&text, "今日已签到"]),
            ..CheckinVerdict::default()
        };
    }
    if protocol::disabled_pattern().is_match(&text) {
        return CheckinVerdict {
            state: "disabled".to_string(),
            message: protocol::first_non_empty(&[&text, "站点未开放签到"]),
            ..CheckinVerdict::default()
        };
    }
    if protocol::auth_pattern().is_match(&text) {
        return CheckinVerdict {
            state: "unauthorized".to_string(),
            message: protocol::first_non_empty(&[&text, "访问令牌无效"]),
            ..CheckinVerdict::default()
        };
    }
    if protocol::missing_endpoint_pattern().is_match(&text) {
        return CheckinVerdict {
            state: "skip".to_string(),
            message: text,
            ..CheckinVerdict::default()
        };
    }
    CheckinVerdict {
        state: "failed".to_string(),
        message: protocol::first_non_empty(&[&text, &format!("签到失败（HTTP {status}）")]),
        ..CheckinVerdict::default()
    }
}
