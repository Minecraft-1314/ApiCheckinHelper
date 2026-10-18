use std::io;

use super::error::{AppError, NetworkFault};

pub fn offline_state(error: &NetworkFault) -> String {
    match error.kind.as_str() {
        "dns" | "connect" | "refused" | "timeout" | "reset" => "offline".to_string(),
        _ => "failed".to_string(),
    }
}

pub fn classify(error: &io::Error) -> NetworkFault {
    let text = error.to_string();
    let kind = error.kind();
    match kind {
        io::ErrorKind::NotFound => NetworkFault::new("域名解析失败", "dns", true),
        io::ErrorKind::ConnectionRefused => NetworkFault::new("对方拒绝连接", "refused", true),
        io::ErrorKind::ConnectionReset => NetworkFault::new("连接被中断", "reset", true),
        io::ErrorKind::ConnectionAborted => NetworkFault::new("连接被中断", "reset", true),
        io::ErrorKind::TimedOut => NetworkFault::new("连接超时", "timeout", true),
        _ => classify_text(&text),
    }
}

pub fn classify_text(text: &str) -> NetworkFault {
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("certificate") || lowered.contains("tls") || lowered.contains("ssl") {
        return NetworkFault::new("证书校验失败，请检查系统时间或代理设置", "ssl", false);
    }
    if lowered.contains("dns")
        || lowered.contains("name or service not known")
        || lowered.contains("no such host")
        || lowered.contains("failed to lookup address")
    {
        return NetworkFault::new("域名解析失败", "dns", true);
    }
    if lowered.contains("refused") {
        return NetworkFault::new("对方拒绝连接", "refused", true);
    }
    if lowered.contains("timed out") || lowered.contains("timeout") {
        return NetworkFault::new("连接超时", "timeout", true);
    }
    if lowered.contains("unreachable") {
        return NetworkFault::new("网络不可达", "connect", true);
    }
    if lowered.contains("reset") {
        return NetworkFault::new("连接被中断", "reset", true);
    }
    NetworkFault::offline()
}

pub fn blocked(message: &str) -> AppError {
    AppError::Network(NetworkFault::new(message, "blocked", false))
}
