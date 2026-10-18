use serde_json::Value;

use super::coerce;

pub const INT_UNPARSABLE: i32 = i32::MIN;
pub const LONG_UNPARSABLE: i64 = i64::MIN;
pub const DOUBLE_UNPARSABLE: f64 = f64::MIN;

pub fn read_int(value: Option<&Value>) -> i32 {
    coerce::to_int(value, INT_UNPARSABLE, None, None)
}

pub fn read_long(value: Option<&Value>) -> i64 {
    coerce::to_long(value, LONG_UNPARSABLE, None, None)
}

pub fn read_double(value: Option<&Value>) -> f64 {
    let parsed = coerce::to_float(value, DOUBLE_UNPARSABLE, None, None);
    if parsed.is_finite() {
        parsed
    } else {
        DOUBLE_UNPARSABLE
    }
}

pub fn read_bool(value: Option<&Value>) -> bool {
    coerce::to_bool(value, false)
}

pub fn read_text(value: Option<&Value>) -> String {
    coerce::to_text(value, "")
}

pub fn read_bool_or_null(value: Option<&Value>) -> Option<bool> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::Bool(flag)) => Some(*flag),
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return None;
            }
            match trimmed.to_ascii_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        }
        Some(item) => Some(coerce::to_bool(Some(item), false)),
    }
}

pub fn read_double_text(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    coerce::format_float(value)
}
