use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;

use super::paths;

pub const MAX_BYTES: u64 = 512 * 1024;

pub struct AppLog {
    gate: Mutex<LogState>,
}

struct LogState {
    path: PathBuf,
}

impl AppLog {
    pub fn bind() -> Self {
        let path = match std::panic::catch_unwind(paths::resolve_data_directory) {
            Ok(directory) => directory.join("app.log"),
            Err(_) => std::env::temp_dir().join("api-checkin-helper.log"),
        };
        Self {
            gate: Mutex::new(LogState { path }),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.gate
            .lock()
            .map(|state| state.path.clone())
            .unwrap_or_else(|_| std::env::temp_dir().join("api-checkin-helper.log"))
    }

    pub fn info(&self, message: &str) {
        self.write("INFO", message, None);
    }

    pub fn warn(&self, message: &str) {
        self.write("WARN", message, None);
    }

    pub fn error(&self, message: &str, detail: Option<&str>) {
        self.write("ERROR", message, detail);
    }

    pub fn handle_unhandled(&self, kind: &str, message: &str, detail: &str) {
        self.error(&format!("未捕获异常 {kind}: {message}"), Some(detail));
    }

    fn write(&self, level: &str, message: &str, detail: Option<&str>) {
        let mut line = format!("{} [{}] {}", timestamp(), level, message);
        if let Some(text) = detail {
            if !text.is_empty() {
                line.push('\n');
                line.push_str(text);
            }
        }
        let redacted = redact(&line);
        let Ok(mut state) = self.gate.lock() else {
            return;
        };
        if state.path.as_os_str().is_empty() {
            return;
        }
        if let Ok(meta) = fs::metadata(&state.path) {
            if meta.len() > MAX_BYTES {
                let _ = fs::remove_file(&state.path);
            }
        }
        if append(&state.path, &redacted).is_err() {
            let fallback = std::env::temp_dir().join("api-checkin-helper.log");
            if state.path != fallback {
                state.path = fallback.clone();
                let _ = append(&fallback, &redacted);
            }
        }
    }
}

fn append(path: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\r\n")?;
    Ok(())
}

pub fn timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|item| item.as_secs() as i64)
        .unwrap_or(0);
    format_local(seconds)
}

pub fn format_local(unix_seconds: i64) -> String {
    let offset = local_offset_seconds(unix_seconds);
    let shifted = unix_seconds + offset;
    let days = shifted.div_euclid(86_400);
    let seconds_of_day = shifted.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

pub fn local_offset_seconds(unix_seconds: i64) -> i64 {
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::Time::{
        FileTimeToSystemTime, SystemTimeToFileTime, TzSpecificLocalTimeToSystemTime,
    };

    let epoch = unix_seconds.saturating_add(11_644_473_600);
    let file_time = FILETIME {
        dwLowDateTime: (epoch as u64 & 0xFFFF_FFFF) as u32,
        dwHighDateTime: ((epoch as u64 >> 32) & 0xFFFF_FFFF) as u32,
    };
    let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let mut local: SYSTEMTIME = unsafe { std::mem::zeroed() };
    let mut local_time: FILETIME = unsafe { std::mem::zeroed() };
    unsafe {
        if FileTimeToSystemTime(&file_time, &mut utc) == 0 {
            return 0;
        }
        if TzSpecificLocalTimeToSystemTime(std::ptr::null(), &utc, &mut local) == 0 {
            return 0;
        }
        if SystemTimeToFileTime(&local, &mut local_time) == 0 {
            return 0;
        }
    }
    let value = ((local_time.dwHighDateTime as u64) << 32) | local_time.dwLowDateTime as u64;
    (value as i64) - epoch
}

pub fn redact(value: &str) -> String {
    let bearer = Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+").expect("bearer pattern");
    let query = Regex::new(r"(?i)([?&](?:authorization|cookie|password|passwd|pwd|access[_-]?token|refresh[_-]?token|token|secret|api[_-]?key|code|state)=)[^&\s]+")
        .expect("query pattern");
    let named = Regex::new(r#"(?i)\b(authorization|cookie|set-cookie|password|passwd|pwd|access[_-]?token|refresh[_-]?token|token|secret|api[_-]?key)("?\s*[:=]\s*"?)[^\s";,}]+"#)
        .expect("named pattern");
    let first = bearer.replace_all(value, "Bearer [REDACTED]");
    let second = query.replace_all(&first, "${1}[REDACTED]");
    named
        .replace_all(&second, "${1}${2}[REDACTED]")
        .into_owned()
}
