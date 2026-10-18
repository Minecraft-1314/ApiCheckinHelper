use serde_json::Value;

use super::coerce;

pub fn get<'a>(node: Option<&'a Value>, path: &[&str]) -> Option<&'a Value> {
    let mut current = node;
    for key in path {
        let map = current?.as_object()?;
        current = map.get(*key);
    }
    current
}

pub fn text(node: Option<&Value>, path: &[&str]) -> String {
    coerce::to_text(get(node, path).filter(|item| !item.is_null()), "")
}

pub fn boolean(node: Option<&Value>, path: &[&str]) -> bool {
    coerce::to_bool(get(node, path), false)
}

pub fn bool_or_null(node: Option<&Value>, path: &[&str]) -> Option<bool> {
    let value = get(node, path)?;
    if value.is_null() {
        return None;
    }
    if let Some(flag) = value.as_bool() {
        return Some(flag);
    }
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        return match trimmed.to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
    }
    Some(coerce::to_bool(Some(value), false))
}

pub fn int(node: Option<&Value>, key: &str, fallback: i32, low: i32, high: i32) -> i32 {
    coerce::to_int(get(node, &[key]), fallback, Some(low), Some(high))
}

pub fn long(node: Option<&Value>, key: &str, fallback: i64) -> i64 {
    coerce::to_long(get(node, &[key]), fallback, None, None)
}

pub fn double(node: Option<&Value>, key: &str, fallback: f64, low: f64, high: f64) -> f64 {
    coerce::to_float(get(node, &[key]), fallback, Some(low), Some(high))
}

pub fn object<'a>(
    node: Option<&'a Value>,
    path: &[&str],
) -> Option<&'a serde_json::Map<String, Value>> {
    get(node, path)?.as_object()
}
