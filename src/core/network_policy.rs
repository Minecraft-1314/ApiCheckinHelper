use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub const MAX_HOST_LENGTH: usize = 253;

pub const BLOCKED_HOSTS: [&str; 9] = [
    "0.0.0.0",
    "::",
    "::1",
    "169.254.169.254",
    "169.254.170.2",
    "metadata.google.internal",
    "metadata.goog",
    "localhost",
    "localhost.localdomain",
];

pub const BLOCKED_SUFFIXES: [&str; 6] = [
    ".localhost",
    ".local",
    ".internal",
    ".intranet",
    ".lan",
    ".home.arpa",
];

pub fn is_valid_host(host: &str) -> bool {
    if host.is_empty() || host.len() > MAX_HOST_LENGTH {
        return false;
    }
    let mut value = host;
    if value.len() > 2 && value.starts_with('[') && value.ends_with(']') {
        value = &value[1..value.len() - 1];
    }
    if BLOCKED_HOSTS
        .iter()
        .any(|item| item.eq_ignore_ascii_case(value))
    {
        return false;
    }
    let lowered = value.to_ascii_lowercase();
    if BLOCKED_SUFFIXES
        .iter()
        .any(|suffix| lowered.ends_with(suffix))
    {
        return false;
    }
    if value.starts_with('.') || value.ends_with('.') {
        return false;
    }
    if let Ok(address) = value.parse::<IpAddr>() {
        return is_routable(address);
    }
    if !value.contains('.') {
        return false;
    }
    for label in value.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
        for character in label.chars() {
            let allowed =
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-';
            if !allowed {
                return false;
            }
        }
    }
    true
}

pub fn format_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

pub fn validate_url(value: &str) -> Option<url::Url> {
    let parsed = url::Url::parse(value).ok()?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return None;
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    let host = parsed.host_str()?;
    let host = strip_brackets(host);
    if !is_valid_host(host) {
        return None;
    }
    Some(parsed)
}

pub fn strip_brackets(host: &str) -> &str {
    if host.len() > 2 && host.starts_with('[') && host.ends_with(']') {
        &host[1..host.len() - 1]
    } else {
        host
    }
}

pub fn to_punycode(host: &str) -> Option<String> {
    if host.is_ascii() {
        return Some(host.to_string());
    }
    idna::domain_to_ascii(host).ok()
}

pub fn is_routable(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(value) => is_public_ipv4(value.octets()),
        IpAddr::V6(value) => is_public_ipv6(value),
    }
}

fn is_public_ipv4(bytes: [u8; 4]) -> bool {
    if bytes[0] == 0 || bytes[0] == 10 || bytes[0] == 127 || bytes[0] >= 224 {
        return false;
    }
    if bytes[0] == 100 && (64..=127).contains(&bytes[1]) {
        return false;
    }
    if bytes[0] == 169 && bytes[1] == 254 {
        return false;
    }
    if bytes[0] == 172 && (16..=31).contains(&bytes[1]) {
        return false;
    }
    if bytes[0] == 192 && bytes[1] == 168 {
        return false;
    }
    if bytes[0] == 192 && bytes[1] == 0 && (bytes[2] == 0 || bytes[2] == 2) {
        return false;
    }
    if bytes[0] == 192 && bytes[1] == 88 && bytes[2] == 99 {
        return false;
    }
    if bytes[0] == 198 && (bytes[1] == 18 || bytes[1] == 19) {
        return false;
    }
    if bytes[0] == 198 && bytes[1] == 51 && bytes[2] == 100 {
        return false;
    }
    if bytes[0] == 203 && bytes[1] == 0 && bytes[2] == 113 {
        return false;
    }
    true
}

fn is_public_ipv6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_ipv4(mapped.octets());
    }
    let bytes = address.octets();
    if address.is_unspecified() || address.is_loopback() {
        return false;
    }
    if bytes[0] == 0xFF || (bytes[0] & 0xFE) == 0xFC {
        return false;
    }
    if bytes[0] == 0xFE && ((bytes[1] & 0xC0) == 0x80 || (bytes[1] & 0xC0) == 0xC0) {
        return false;
    }
    if is_documentation_v6(&bytes) {
        return false;
    }
    if is_compatible_v4(&bytes) || is_nat64(&bytes) {
        return is_public_ipv4([bytes[12], bytes[13], bytes[14], bytes[15]]);
    }
    if is_six_to_four(&bytes) {
        return is_public_ipv4([bytes[2], bytes[3], bytes[4], bytes[5]]);
    }
    if !is_teredo(&bytes) {
        return true;
    }
    let inverted = invert([bytes[4], bytes[5], bytes[6], bytes[7]]);
    is_public_ipv4(inverted)
}

fn is_compatible_v4(bytes: &[u8; 16]) -> bool {
    if bytes[..10].iter().any(|item| *item != 0) {
        return false;
    }
    bytes[10] != 0 || bytes[11] != 0
}

fn is_nat64(bytes: &[u8; 16]) -> bool {
    bytes[0] == 0x00 && bytes[1] == 0x64 && bytes[2] == 0xFF && bytes[3] == 0x9B
}

fn is_six_to_four(bytes: &[u8; 16]) -> bool {
    bytes[0] == 0x20 && bytes[1] == 0x02
}

fn is_teredo(bytes: &[u8; 16]) -> bool {
    bytes[0] == 0x20 && bytes[1] == 0x01 && bytes[2] == 0x00 && bytes[3] == 0x00
}

fn is_documentation_v6(bytes: &[u8; 16]) -> bool {
    bytes[0] == 0x20 && bytes[1] == 0x01 && bytes[2] == 0x0D && bytes[3] == 0xB8
}

fn invert(bytes: [u8; 4]) -> [u8; 4] {
    [!bytes[0], !bytes[1], !bytes[2], !bytes[3]]
}

pub fn parse_ipv4_octets(value: &str) -> Option<[u8; 4]> {
    value.parse::<Ipv4Addr>().ok().map(|item| item.octets())
}
