use serde_json::Value;

pub const TRUE_WORDS: [&str; 9] = ["1", "true", "yes", "on", "y", "t", "是", "开", "启用"];
pub const FALSE_WORDS: [&str; 10] = ["0", "false", "no", "off", "n", "f", "否", "关", "禁用", ""];

pub fn to_int(value: Option<&Value>, fallback: i32, low: Option<i32>, high: Option<i32>) -> i32 {
    let number = match value {
        None | Some(Value::Null) => fallback as i64,
        Some(Value::Bool(flag)) => i64::from(*flag),
        Some(Value::Number(number)) => number_to_int(number, fallback as i64),
        Some(Value::String(text)) => parse_text(text, fallback as i64),
        _ => fallback as i64,
    };
    clamp_int(number, low, high)
}

pub fn to_long(value: Option<&Value>, fallback: i64, low: Option<i64>, high: Option<i64>) -> i64 {
    let number = match value {
        None | Some(Value::Null) => fallback,
        Some(Value::Bool(flag)) => i64::from(*flag),
        Some(Value::Number(number)) => number_to_long(number, fallback),
        Some(Value::String(text)) => parse_long(text, fallback),
        _ => fallback,
    };
    clamp_long(number, low, high)
}

pub fn to_float(value: Option<&Value>, fallback: f64, low: Option<f64>, high: Option<f64>) -> f64 {
    let number = match value {
        Some(Value::Bool(flag)) => {
            if *flag {
                1.0
            } else {
                0.0
            }
        }
        Some(Value::Number(number)) => number.as_f64().unwrap_or(fallback),
        Some(Value::String(text)) => parse_float(text, fallback),
        _ => fallback,
    };
    clamp_float(number, fallback, low, high)
}

pub fn to_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_f64().map(|item| item != 0.0).unwrap_or(fallback),
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if TRUE_WORDS
                .iter()
                .any(|word| word.eq_ignore_ascii_case(trimmed))
            {
                return true;
            }
            if FALSE_WORDS
                .iter()
                .any(|word| word.eq_ignore_ascii_case(trimmed))
            {
                return false;
            }
            fallback
        }
        _ => fallback,
    }
}

pub fn to_text(value: Option<&Value>, fallback: &str) -> String {
    match value {
        None | Some(Value::Null) => fallback.to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(flag)) => {
            if *flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Some(Value::Number(number)) => format_number(number),
        Some(_) => fallback.to_string(),
    }
}

pub fn format_number(number: &serde_json::Number) -> String {
    if let Some(text) = number.as_i64() {
        return text.to_string();
    }
    if let Some(text) = number.as_u64() {
        return text.to_string();
    }
    let value = number.as_f64().unwrap_or(0.0);
    format_float(value)
}

pub fn format_float(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let text = format!("{value:.4}");
    let trimmed = text.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    trimmed.to_string()
}

fn number_to_int(number: &serde_json::Number, fallback: i64) -> i64 {
    if let Some(value) = number.as_i64() {
        return value;
    }
    match number.as_f64() {
        Some(value) => round_away(value).unwrap_or(fallback),
        None => fallback,
    }
}

fn number_to_long(number: &serde_json::Number, fallback: i64) -> i64 {
    if let Some(value) = number.as_i64() {
        return value;
    }
    if let Some(value) = number.as_u64() {
        return i64::try_from(value).unwrap_or(i64::MAX);
    }
    match number.as_f64() {
        Some(value) => round_away(value).unwrap_or(fallback),
        None => fallback,
    }
}

fn round_away(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let rounded = value.round();
    if rounded >= 9.223_372_036_854_776e18 {
        return Some(i64::MAX);
    }
    if rounded <= -9.223_372_036_854_776e18 {
        return Some(i64::MIN);
    }
    Some(rounded as i64)
}

fn parse_text(text: &str, fallback: i64) -> i64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return fallback;
    }
    if let Ok(direct) = trimmed.parse::<i64>() {
        return direct;
    }
    match trimmed.parse::<f64>() {
        Ok(parsed) => round_away(parsed).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn parse_long(text: &str, fallback: i64) -> i64 {
    parse_text(text, fallback)
}

fn parse_float(text: &str, fallback: f64) -> f64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return fallback;
    }
    trimmed.parse::<f64>().unwrap_or(fallback)
}

fn clamp_int(number: i64, low: Option<i32>, high: Option<i32>) -> i32 {
    let bounded = number.clamp(i64::from(i32::MIN), i64::from(i32::MAX));
    let mut result = bounded as i32;
    if let Some(value) = low {
        result = result.max(value);
    }
    if let Some(value) = high {
        result = result.min(value);
    }
    result
}

fn clamp_long(number: i64, low: Option<i64>, high: Option<i64>) -> i64 {
    let mut result = number;
    if let Some(value) = low {
        result = result.max(value);
    }
    if let Some(value) = high {
        result = result.min(value);
    }
    result
}

fn clamp_float(number: f64, fallback: f64, low: Option<f64>, high: Option<f64>) -> f64 {
    let mut result = if number.is_finite() { number } else { fallback };
    if let Some(value) = low {
        result = result.max(value);
    }
    if let Some(value) = high {
        result = result.min(value);
    }
    result
}
