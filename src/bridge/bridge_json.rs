use serde_json::{Map, Value};

pub fn text(source: &Value, name: &str, fallback: &str) -> String {
    match source.get(name) {
        Some(Value::String(value)) => value.clone(),
        _ => fallback.to_string(),
    }
}

pub fn int(source: &Value, name: &str, fallback: i32) -> i32 {
    match source.get(name) {
        Some(Value::Number(number)) => number
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(fallback),
        Some(Value::String(value)) => value.trim().parse::<i32>().unwrap_or(fallback),
        _ => fallback,
    }
}

pub fn boolean(source: &Value, name: &str, fallback: bool) -> bool {
    match source.get(name) {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value.trim().parse::<bool>().unwrap_or(fallback),
        _ => fallback,
    }
}

pub fn object(source: &Value) -> Map<String, Value> {
    source.as_object().cloned().unwrap_or_default()
}
