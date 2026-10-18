use serde_json::Value;

use super::model_rules::STATE_VERSION;

pub struct SchemaError(pub String);

impl std::fmt::Display for SchemaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub fn validate(json: &str) -> Result<(), SchemaError> {
    let value: Value =
        serde_json::from_str(json).map_err(|error| SchemaError(format!("$: {error}")))?;
    validate_root(&value)
}

fn validate_root(root: &Value) -> Result<(), SchemaError> {
    require_shape(
        root,
        "$",
        &[
            "Version",
            "Settings",
            "Sites",
            "Credentials",
            "Meta",
            "Results",
            "LastRun",
            "Verify",
            "History",
        ],
    )?;
    require_int32(
        required(root, "Version", "$")?,
        "$.Version",
        Some(STATE_VERSION),
    )?;
    validate_settings(required(root, "Settings", "$")?, "$.Settings")?;
    validate_array(required(root, "Sites", "$")?, "$.Sites", validate_site)?;
    validate_map(
        required(root, "Credentials", "$")?,
        "$.Credentials",
        validate_credential,
        false,
    )?;
    validate_map(required(root, "Meta", "$")?, "$.Meta", validate_meta, false)?;
    validate_map(
        required(root, "Results", "$")?,
        "$.Results",
        validate_result,
        true,
    )?;
    validate_last_run(required(root, "LastRun", "$")?, "$.LastRun")?;
    validate_verify(required(root, "Verify", "$")?, "$.Verify")?;
    validate_map(
        required(root, "History", "$")?,
        "$.History",
        validate_history,
        false,
    )?;
    Ok(())
}

fn validate_settings(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(
        value,
        path,
        &[
            "Concurrency",
            "TimeoutMs",
            "Notify",
            "SkipManualOnly",
            "Theme",
            "Language",
        ],
    )?;
    require_int32(
        required(value, "Concurrency", path)?,
        &format!("{path}.Concurrency"),
        None,
    )?;
    require_int32(
        required(value, "TimeoutMs", path)?,
        &format!("{path}.TimeoutMs"),
        None,
    )?;
    require_boolean(required(value, "Notify", path)?, &format!("{path}.Notify"))?;
    require_boolean(
        required(value, "SkipManualOnly", path)?,
        &format!("{path}.SkipManualOnly"),
    )?;
    require_string(required(value, "Theme", path)?, &format!("{path}.Theme"))?;
    require_string(
        required(value, "Language", path)?,
        &format!("{path}.Language"),
    )?;
    Ok(())
}

fn validate_site(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(
        value,
        path,
        &[
            "Id", "Name", "Host", "Origin", "Entry", "Tags", "Bookmark", "Snapshot",
        ],
    )?;
    require_string(required(value, "Id", path)?, &format!("{path}.Id"))?;
    require_string(required(value, "Name", path)?, &format!("{path}.Name"))?;
    require_string(required(value, "Host", path)?, &format!("{path}.Host"))?;
    require_string(required(value, "Origin", path)?, &format!("{path}.Origin"))?;
    require_string(required(value, "Entry", path)?, &format!("{path}.Entry"))?;
    validate_string_array(required(value, "Tags", path)?, &format!("{path}.Tags"))?;
    require_string(
        required(value, "Bookmark", path)?,
        &format!("{path}.Bookmark"),
    )?;
    validate_snapshot(
        required(value, "Snapshot", path)?,
        &format!("{path}.Snapshot"),
    )?;
    Ok(())
}

fn validate_snapshot(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(
        value,
        path,
        &[
            "SystemName",
            "Version",
            "CheckinEnabled",
            "Turnstile",
            "QuotaPerUnit",
            "DisplayInCurrency",
            "DisplayType",
            "ExchangeRate",
        ],
    )?;
    require_string(
        required(value, "SystemName", path)?,
        &format!("{path}.SystemName"),
    )?;
    require_optional_string(value, "Version", path)?;
    require_optional_boolean(value, "CheckinEnabled", path)?;
    require_boolean(
        required(value, "Turnstile", path)?,
        &format!("{path}.Turnstile"),
    )?;
    require_int32(
        required(value, "QuotaPerUnit", path)?,
        &format!("{path}.QuotaPerUnit"),
        None,
    )?;
    require_boolean(
        required(value, "DisplayInCurrency", path)?,
        &format!("{path}.DisplayInCurrency"),
    )?;
    require_string(
        required(value, "DisplayType", path)?,
        &format!("{path}.DisplayType"),
    )?;
    require_double(
        required(value, "ExchangeRate", path)?,
        &format!("{path}.ExchangeRate"),
    )?;
    Ok(())
}

fn validate_credential(value: &Value, path: &str) -> Result<(), SchemaError> {
    if value.is_array() {
        return validate_array(value, path, validate_credential);
    }
    require_shape(value, path, &["Type", "Token", "UserId", "GroupId"])?;
    require_string(required(value, "Type", path)?, &format!("{path}.Type"))?;
    require_string(required(value, "Token", path)?, &format!("{path}.Token"))?;
    require_int32(
        required(value, "UserId", path)?,
        &format!("{path}.UserId"),
        None,
    )?;
    require_optional_string(value, "GroupId", path)?;
    Ok(())
}

fn validate_meta(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(
        value,
        path,
        &[
            "SystemName",
            "Version",
            "CheckinEnabled",
            "Turnstile",
            "QuotaPerUnit",
            "DisplayInCurrency",
            "DisplayType",
            "ExchangeRate",
            "Account",
            "UserId",
            "ProbedAt",
        ],
    )?;
    require_string(
        required(value, "SystemName", path)?,
        &format!("{path}.SystemName"),
    )?;
    require_string(
        required(value, "Version", path)?,
        &format!("{path}.Version"),
    )?;
    require_optional_boolean(value, "CheckinEnabled", path)?;
    require_boolean(
        required(value, "Turnstile", path)?,
        &format!("{path}.Turnstile"),
    )?;
    require_int32(
        required(value, "QuotaPerUnit", path)?,
        &format!("{path}.QuotaPerUnit"),
        None,
    )?;
    require_boolean(
        required(value, "DisplayInCurrency", path)?,
        &format!("{path}.DisplayInCurrency"),
    )?;
    require_string(
        required(value, "DisplayType", path)?,
        &format!("{path}.DisplayType"),
    )?;
    require_double(
        required(value, "ExchangeRate", path)?,
        &format!("{path}.ExchangeRate"),
    )?;
    require_string(
        required(value, "Account", path)?,
        &format!("{path}.Account"),
    )?;
    require_int32(
        required(value, "UserId", path)?,
        &format!("{path}.UserId"),
        None,
    )?;
    require_int64(
        required(value, "ProbedAt", path)?,
        &format!("{path}.ProbedAt"),
    )?;
    Ok(())
}

fn validate_result(value: &Value, path: &str) -> Result<(), SchemaError> {
    if value.is_null() {
        return Ok(());
    }
    require_shape(
        value,
        path,
        &[
            "Id",
            "State",
            "Message",
            "Quota",
            "Path",
            "HttpStatus",
            "At",
        ],
    )?;
    require_string(required(value, "Id", path)?, &format!("{path}.Id"))?;
    require_string(required(value, "State", path)?, &format!("{path}.State"))?;
    require_string(
        required(value, "Message", path)?,
        &format!("{path}.Message"),
    )?;
    require_int64(required(value, "Quota", path)?, &format!("{path}.Quota"))?;
    require_string(required(value, "Path", path)?, &format!("{path}.Path"))?;
    require_int32(
        required(value, "HttpStatus", path)?,
        &format!("{path}.HttpStatus"),
        None,
    )?;
    require_int64(required(value, "At", path)?, &format!("{path}.At"))?;
    Ok(())
}

fn validate_last_run(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(value, path, &["Day", "At"])?;
    require_string(required(value, "Day", path)?, &format!("{path}.Day"))?;
    require_int64(required(value, "At", path)?, &format!("{path}.At"))?;
    Ok(())
}

fn validate_verify(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(value, path, &["At", "Total", "Ok", "Failed", "Items"])?;
    require_int64(required(value, "At", path)?, &format!("{path}.At"))?;
    require_int32(
        required(value, "Total", path)?,
        &format!("{path}.Total"),
        None,
    )?;
    require_int32(required(value, "Ok", path)?, &format!("{path}.Ok"), None)?;
    require_int32(
        required(value, "Failed", path)?,
        &format!("{path}.Failed"),
        None,
    )?;
    validate_array(
        required(value, "Items", path)?,
        &format!("{path}.Items"),
        validate_verify_entry,
    )?;
    Ok(())
}

fn validate_verify_entry(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(value, path, &["Id", "Name", "Host", "Ok", "Message"])?;
    require_string(required(value, "Id", path)?, &format!("{path}.Id"))?;
    require_string(required(value, "Name", path)?, &format!("{path}.Name"))?;
    require_string(required(value, "Host", path)?, &format!("{path}.Host"))?;
    require_boolean(required(value, "Ok", path)?, &format!("{path}.Ok"))?;
    require_string(
        required(value, "Message", path)?,
        &format!("{path}.Message"),
    )?;
    Ok(())
}

fn validate_history(value: &Value, path: &str) -> Result<(), SchemaError> {
    require_shape(
        value,
        path,
        &[
            "Total", "Success", "Already", "Failed", "Skipped", "Quota", "QuotaUsd", "At",
        ],
    )?;
    require_int32(
        required(value, "Total", path)?,
        &format!("{path}.Total"),
        None,
    )?;
    require_int32(
        required(value, "Success", path)?,
        &format!("{path}.Success"),
        None,
    )?;
    require_int32(
        required(value, "Already", path)?,
        &format!("{path}.Already"),
        None,
    )?;
    require_int32(
        required(value, "Failed", path)?,
        &format!("{path}.Failed"),
        None,
    )?;
    require_int32(
        required(value, "Skipped", path)?,
        &format!("{path}.Skipped"),
        None,
    )?;
    require_int64(required(value, "Quota", path)?, &format!("{path}.Quota"))?;
    require_double(
        required(value, "QuotaUsd", path)?,
        &format!("{path}.QuotaUsd"),
    )?;
    require_int64(required(value, "At", path)?, &format!("{path}.At"))?;
    Ok(())
}

fn validate_map(
    value: &Value,
    path: &str,
    validate_value: fn(&Value, &str) -> Result<(), SchemaError>,
    allow_null_values: bool,
) -> Result<(), SchemaError> {
    let map = value
        .as_object()
        .ok_or_else(|| invalid(path, "字段类型应为 Object"))?;
    for (name, item) in map {
        let child_path = format!("{path}.{name}");
        if item.is_null() && !allow_null_values {
            return Err(invalid(&child_path, "不允许空值"));
        }
        validate_value(item, &child_path)?;
    }
    Ok(())
}

fn validate_array(
    value: &Value,
    path: &str,
    validate_item: fn(&Value, &str) -> Result<(), SchemaError>,
) -> Result<(), SchemaError> {
    let items = value
        .as_array()
        .ok_or_else(|| invalid(path, "字段类型应为 Array"))?;
    for (index, item) in items.iter().enumerate() {
        validate_item(item, &format!("{path}[{index}]"))?;
    }
    Ok(())
}

fn validate_string_array(value: &Value, path: &str) -> Result<(), SchemaError> {
    validate_array(value, path, |item, item_path| {
        require_string(item, item_path)
    })
}

fn required<'a>(value: &'a Value, name: &str, path: &str) -> Result<&'a Value, SchemaError> {
    value
        .get(name)
        .ok_or_else(|| invalid(path, &format!("缺少字段 {name}")))
}

fn require_shape(value: &Value, path: &str, fields: &[&str]) -> Result<(), SchemaError> {
    let map = value
        .as_object()
        .ok_or_else(|| invalid(path, "字段类型应为 Object"))?;
    for name in map.keys() {
        if !fields.contains(&name.as_str()) {
            return Err(invalid(path, &format!("未知字段 {name}")));
        }
    }
    Ok(())
}

fn require_optional_string(value: &Value, name: &str, path: &str) -> Result<(), SchemaError> {
    match value.get(name) {
        None => Ok(()),
        Some(item) => require_string(item, &format!("{path}.{name}")),
    }
}

fn require_optional_boolean(value: &Value, name: &str, path: &str) -> Result<(), SchemaError> {
    match value.get(name) {
        None => Ok(()),
        Some(item) => require_boolean(item, &format!("{path}.{name}")),
    }
}

fn require_string(value: &Value, path: &str) -> Result<(), SchemaError> {
    if value.is_string() {
        Ok(())
    } else {
        Err(invalid(path, "字段类型应为 String"))
    }
}

fn require_boolean(value: &Value, path: &str) -> Result<(), SchemaError> {
    if value.is_boolean() {
        Ok(())
    } else {
        Err(invalid(path, "字段类型应为 Boolean"))
    }
}

fn require_int32(value: &Value, path: &str, expected: Option<i32>) -> Result<(), SchemaError> {
    if !value.is_number() {
        return Err(invalid(path, "字段类型应为 Number"));
    }
    let parsed = value.as_i64().and_then(|item| i32::try_from(item).ok());
    match parsed {
        Some(number) if expected.map(|item| item == number).unwrap_or(true) => Ok(()),
        _ => Err(invalid(path, "整数字段不符合当前架构")),
    }
}

fn require_int64(value: &Value, path: &str) -> Result<(), SchemaError> {
    if !value.is_number() {
        return Err(invalid(path, "字段类型应为 Number"));
    }
    match value.as_i64() {
        Some(_) => Ok(()),
        None => Err(invalid(path, "长整数字段不符合当前架构")),
    }
}

fn require_double(value: &Value, path: &str) -> Result<(), SchemaError> {
    if !value.is_number() {
        return Err(invalid(path, "字段类型应为 Number"));
    }
    match value.as_f64() {
        Some(parsed) if parsed.is_finite() => Ok(()),
        _ => Err(invalid(path, "数值字段不符合当前架构")),
    }
}

fn invalid(path: &str, reason: &str) -> SchemaError {
    SchemaError(format!("{path}: {reason}"))
}
