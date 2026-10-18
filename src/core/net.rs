use std::collections::BTreeMap;
use std::io::Read;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::cancel::CancelToken;
use super::checkin;
use super::error::{AppError, AppResult, NetworkFault};
use super::net_errors;
use super::network_policy;

pub const DEFAULT_TIMEOUT_MS: i32 = 15_000;
pub const MAX_RETRY: u32 = 2;
pub const MAX_REDIRECTS: u32 = 5;
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub const RETRY_BACKOFF_MS: u64 = 350;
pub const CONNECT_TIMEOUT_SECONDS: u64 = 10;

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

pub const DEFAULT_HEADERS: [(&str, &str); 2] = [
    ("Accept", "application/json, text/plain, */*"),
    ("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8"),
];

pub const RETRYABLE_STATUS: [u16; 7] = [408, 425, 429, 500, 502, 503, 504];

pub const CROSS_ORIGIN_UNSAFE_HEADERS: [&str; 6] = [
    "Authorization",
    "Cookie",
    "Proxy-Authorization",
    "Origin",
    "Referer",
    "New-Api-User",
];

#[derive(Debug, Clone, Default)]
pub struct NetResponse {
    pub status: i32,
    pub text: String,
    pub content_type: String,
    pub data: Option<Value>,
    pub cookies: BTreeMap<String, String>,
    pub elapsed_ms: i64,
}

impl NetResponse {
    pub fn looks_html(&self) -> bool {
        self.content_type.to_ascii_lowercase().contains("text/html")
            || self.text.trim_start().starts_with('<')
    }

    pub fn cookie(&self, name: &str) -> Option<String> {
        self.cookies
            .get(name)
            .filter(|value| !value.is_empty())
            .cloned()
    }

    pub fn synth(status: i32, data: Option<Value>, text: Option<&str>, content_type: &str) -> Self {
        let body = match text {
            Some(value) => value.to_string(),
            None => data
                .as_ref()
                .map(|item| item.to_string())
                .unwrap_or_default(),
        };
        Self {
            status,
            text: body,
            content_type: content_type.to_string(),
            data,
            cookies: BTreeMap::new(),
            elapsed_ms: 0,
        }
    }
}

pub struct NetClient {
    agent: ureq::Agent,
}

fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .user_agent(USER_AGENT)
        .resolver(|netloc: &str| resolve_safe(netloc))
        .build()
}

fn resolve_safe(netloc: &str) -> std::io::Result<Vec<SocketAddr>> {
    let (host, port) = split_netloc(netloc);
    if !network_policy::is_valid_host(host) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "目标地址被安全策略阻止",
        ));
    }
    let resolved = std::net::ToSocketAddrs::to_socket_addrs(&(host, port))?;
    let allowed: Vec<SocketAddr> = resolved
        .filter(|item| network_policy::is_routable(item.ip()))
        .collect();
    if allowed.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "目标地址被安全策略阻止",
        ));
    }
    Ok(allowed)
}

fn split_netloc(netloc: &str) -> (&str, u16) {
    if let Some(stripped) = netloc.strip_prefix('[') {
        if let Some(close) = stripped.find(']') {
            let host = &stripped[..close];
            let rest = &stripped[close + 1..];
            let port = rest
                .strip_prefix(':')
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(443);
            return (host, port);
        }
    }
    match netloc.rsplit_once(':') {
        Some((host, port)) => match port.parse::<u16>() {
            Ok(value) => (host, value),
            Err(_) => (netloc, 443),
        },
        None => (netloc, 443),
    }
}

impl NetClient {
    pub fn new() -> Self {
        Self {
            agent: build_agent(),
        }
    }

    pub fn with_agent(agent: ureq::Agent) -> Self {
        Self { agent }
    }

    fn resolved(&self) -> ureq::Agent {
        self.agent.clone()
    }

    pub fn get(
        &self,
        url: &str,
        timeout_ms: i32,
        headers: Option<&BTreeMap<String, String>>,
        deadline: &CancelToken,
    ) -> AppResult<NetResponse> {
        self.send("GET", url, timeout_ms, headers, None, deadline)
    }

    pub fn post(
        &self,
        url: &str,
        timeout_ms: i32,
        headers: Option<&BTreeMap<String, String>>,
        body: Option<&Value>,
        deadline: &CancelToken,
    ) -> AppResult<NetResponse> {
        self.send("POST", url, timeout_ms, headers, body, deadline)
    }

    fn send(
        &self,
        method: &str,
        url: &str,
        timeout_ms: i32,
        headers: Option<&BTreeMap<String, String>>,
        body: Option<&Value>,
        deadline: &CancelToken,
    ) -> AppResult<NetResponse> {
        let mut last: Option<AppError> = None;
        for attempt in 0..=MAX_RETRY {
            deadline.throw_if_cancelled()?;
            match self.attempt(method, url, timeout_ms, headers, body, deadline) {
                Ok(response) => {
                    if RETRYABLE_STATUS.contains(&(response.status as u16)) && attempt < MAX_RETRY {
                        last = Some(AppError::Network(NetworkFault::new(
                            format!("服务端暂时不可用（{}）", response.status),
                            "server",
                            true,
                        )));
                        self.backoff(attempt, deadline)?;
                        continue;
                    }
                    return Ok(response);
                }
                Err(error) => {
                    if error.is_cancelled() {
                        return Err(error);
                    }
                    let retryable = error.fault().map(|fault| fault.retryable).unwrap_or(false);
                    if !retryable || attempt == MAX_RETRY {
                        return Err(error);
                    }
                    last = Some(error);
                    self.backoff(attempt, deadline)?;
                }
            }
        }
        Err(last.unwrap_or_else(|| {
            AppError::Network(NetworkFault::new("网络请求失败", "unknown", false))
        }))
    }

    fn backoff(&self, attempt: u32, deadline: &CancelToken) -> AppResult<()> {
        let delay = ((RETRY_BACKOFF_MS * u64::from(attempt + 1)) as f64).min(1000.0) as u64;
        let mut remaining = Duration::from_millis(delay);
        while !remaining.is_zero() {
            deadline.throw_if_cancelled()?;
            let step = remaining.min(Duration::from_millis(25));
            std::thread::sleep(step);
            remaining = remaining.saturating_sub(step);
        }
        Ok(())
    }

    fn attempt(
        &self,
        method: &str,
        url: &str,
        timeout_ms: i32,
        headers: Option<&BTreeMap<String, String>>,
        body: Option<&Value>,
        deadline: &CancelToken,
    ) -> AppResult<NetResponse> {
        deadline.throw_if_cancelled()?;
        if network_policy::validate_url(url).is_none() {
            return Err(net_errors::blocked("目标地址被安全策略阻止"));
        }
        let started = Instant::now();
        let timeout = timeout_of(timeout_ms);
        let mut current_url = url.to_string();
        let mut current_method = method.to_string();
        let mut current_body = body.cloned();
        let mut current_headers = headers.cloned();
        for redirect in 0..=MAX_REDIRECTS {
            deadline.throw_if_cancelled()?;
            if network_policy::validate_url(&current_url).is_none() {
                return Err(net_errors::blocked("目标地址被安全策略阻止"));
            }
            let agent = self.resolved();
            let mut request = agent
                .request(&current_method, &current_url)
                .timeout(timeout);
            for (name, value) in DEFAULT_HEADERS {
                request = request.set(name, value);
            }
            if let Some(map) = current_headers.as_ref() {
                for (key, value) in map {
                    if key.eq_ignore_ascii_case("content-type") {
                        continue;
                    }
                    request = request.set(key, value);
                }
            }
            let outcome = match current_body.as_ref() {
                Some(payload) => request
                    .set("Content-Type", "application/json")
                    .send_string(&payload.to_string()),
                None => request.call(),
            };
            let response = match outcome {
                Ok(value) => value,
                Err(ureq::Error::Status(_, value)) => value,
                Err(ureq::Error::Transport(transport)) => {
                    if deadline.is_cancelled() {
                        return Err(AppError::Cancelled);
                    }
                    let fault = net_errors::classify_text(&transport.to_string());
                    return Err(AppError::Network(fault));
                }
            };
            let status = i32::from(response.status());
            if !is_redirect(status) {
                return read_response(response, started, deadline);
            }
            if redirect == MAX_REDIRECTS {
                return Err(AppError::Network(NetworkFault::new(
                    "重定向次数过多",
                    "redirect",
                    false,
                )));
            }
            let location = response.header("Location").map(|item| item.to_string());
            drop(response);
            let Some(location) = location else {
                return Err(AppError::Network(NetworkFault::new(
                    "重定向次数过多",
                    "redirect",
                    false,
                )));
            };
            let base = network_policy::validate_url(&current_url)
                .ok_or_else(|| net_errors::blocked("目标地址被安全策略阻止"))?;
            let next = base.join(&location).map_err(|_| {
                AppError::Network(NetworkFault::new(
                    "重定向目标被安全策略阻止",
                    "blocked",
                    false,
                ))
            })?;
            if network_policy::validate_url(next.as_str()).is_none() {
                return Err(net_errors::blocked("重定向目标被安全策略阻止"));
            }
            if !same_origin(&current_url, &next) {
                current_headers = Some(strip_cross_origin_headers(current_headers.as_ref()));
            }
            if converts_to_get(&current_method, status) {
                current_method = "GET".to_string();
                current_body = None;
            }
            current_url = next.to_string();
        }
        Err(AppError::Network(NetworkFault::new(
            "重定向次数过多",
            "redirect",
            false,
        )))
    }
}

impl Default for NetClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn timeout_of(timeout_ms: i32) -> Duration {
    let value = super::coerce::to_int(
        Some(&Value::from(timeout_ms)),
        DEFAULT_TIMEOUT_MS,
        Some(500),
        Some(600_000),
    );
    Duration::from_millis(value as u64)
}

fn read_response(
    response: ureq::Response,
    started: Instant,
    deadline: &CancelToken,
) -> AppResult<NetResponse> {
    let status = i32::from(response.status());
    let content_type = response.content_type().to_string();
    let charset = response.charset().to_string();
    let declared = response
        .header("Content-Length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if declared > MAX_RESPONSE_BYTES {
        return Err(AppError::Network(NetworkFault::new(
            "响应内容过大",
            "response-too-large",
            false,
        )));
    }
    let cookies = read_cookies(&response);
    let mut reader = response.into_reader();
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        deadline.throw_if_cancelled()?;
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                if buffer.len() + read > MAX_RESPONSE_BYTES {
                    return Err(AppError::Network(NetworkFault::new(
                        "响应内容过大",
                        "response-too-large",
                        false,
                    )));
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            Err(error) => {
                if deadline.is_cancelled() {
                    return Err(AppError::Cancelled);
                }
                return Err(AppError::Network(net_errors::classify(&error)));
            }
        }
    }
    let text = decode_text(&buffer, &charset);
    let data = parse_json(&text, &content_type);
    Ok(NetResponse {
        status,
        text,
        content_type,
        data,
        cookies,
        elapsed_ms: started.elapsed().as_millis() as i64,
    })
}

fn read_cookies(response: &ureq::Response) -> BTreeMap<String, String> {
    let mut cookies: BTreeMap<String, String> = BTreeMap::new();
    for raw in response.all("Set-Cookie") {
        let pair = match raw.find(';') {
            Some(index) => &raw[..index],
            None => raw,
        };
        if let Some((name, value)) = pair.split_once('=') {
            let name = name.trim();
            if !name.is_empty() {
                cookies.insert(name.to_string(), value.trim().to_string());
            }
        }
    }
    cookies
}

fn decode_text(bytes: &[u8], charset: &str) -> String {
    let normalized = charset.trim_matches('"').trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "utf-8" || normalized == "utf8" {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    match encoding_rs::Encoding::for_label(normalized.as_bytes()) {
        Some(encoding) => {
            let (text, _, _) = encoding.decode(bytes);
            text.into_owned()
        }
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

fn parse_json(text: &str, content_type: &str) -> Option<Value> {
    if text.is_empty() {
        return None;
    }
    let trimmed = text.trim_start();
    let looks_json = content_type.to_ascii_lowercase().contains("json")
        || content_type.is_empty()
        || trimmed.starts_with('{')
        || trimmed.starts_with('[');
    if !looks_json {
        return None;
    }
    serde_json::from_str(text).ok()
}

fn is_redirect(status: i32) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn converts_to_get(method: &str, status: i32) -> bool {
    let post_like = method.eq_ignore_ascii_case("POST")
        || method.eq_ignore_ascii_case("PUT")
        || method.eq_ignore_ascii_case("PATCH");
    status == 303 || (post_like && (status == 301 || status == 302))
}

fn same_origin(current: &str, next: &url::Url) -> bool {
    let Ok(current) = url::Url::parse(current) else {
        return false;
    };
    current.scheme().eq_ignore_ascii_case(next.scheme())
        && current.host_str().map(|item| item.to_ascii_lowercase())
            == next.host_str().map(|item| item.to_ascii_lowercase())
        && current.port_or_known_default() == next.port_or_known_default()
}

fn strip_cross_origin_headers(
    headers: Option<&BTreeMap<String, String>>,
) -> BTreeMap<String, String> {
    match headers {
        None => BTreeMap::new(),
        Some(map) => map
            .iter()
            .filter(|(key, _)| {
                !CROSS_ORIGIN_UNSAFE_HEADERS
                    .iter()
                    .any(|item| item.eq_ignore_ascii_case(key))
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    }
}

pub fn validate_request_url(value: &str) -> bool {
    network_policy::validate_url(value).is_some()
}

pub fn cross_origin_unsafe_header(name: &str) -> bool {
    CROSS_ORIGIN_UNSAFE_HEADERS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(name))
        || name.eq_ignore_ascii_case(checkin::USER_HEADER)
}
