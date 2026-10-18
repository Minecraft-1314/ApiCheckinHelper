use serde_json::Value;

use super::json;
use super::model::Site;
use super::protocol;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OAuthProvider {
    pub slug: String,
    pub name: String,
    pub kind: String,
    pub client_id: String,
    pub endpoint: String,
    pub scopes: String,
}

#[derive(Debug, Clone, Default)]
pub struct OAuthFlowResult {
    pub ok: bool,
    pub state: String,
    pub message: String,
    pub name: String,
    pub slug: String,
    pub flow_token: String,
    pub expires_at: i64,
    pub authorize_url: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Default)]
pub struct OAuthToken {
    pub ok: bool,
    pub state: String,
    pub message: String,
    pub access_token: String,
    pub access_expires_at: i64,
    pub refresh_cookie: String,
    pub account: String,
}

#[derive(Debug, Clone, Default)]
pub struct DiscoveryResult {
    pub ok: bool,
    pub exists: bool,
    pub host: String,
    pub origin: String,
    pub site_id: String,
    pub name: String,
    pub version: String,
    pub message: String,
    pub checkin_enabled: bool,
    pub turnstile: bool,
    pub password_login: bool,
    pub oauth: Vec<String>,
}

pub const KNOWN_PROVIDERS: [(&str, &str, &str, Option<&str>); 5] = [
    ("github", "GitHub", "github_oauth", Some("github_client_id")),
    (
        "discord",
        "Discord",
        "discord_oauth",
        Some("discord_client_id"),
    ),
    (
        "linuxdo",
        "LinuxDO",
        "linuxdo_oauth",
        Some("linuxdo_client_id"),
    ),
    ("telegram", "Telegram", "telegram_oauth", None),
    ("oidc", "OIDC", "oidc_enabled", Some("oidc_client_id")),
];

pub fn url_encode(value: &str) -> String {
    let mut result = String::new();
    for byte in value.as_bytes() {
        let character = *byte as char;
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '~') {
            result.push(character);
        } else if character == ' ' {
            result.push('+');
        } else {
            result.push_str(&format!("%{byte:02X}"));
        }
    }
    result
}

pub fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                result.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(value) => {
                        result.push(value);
                        index += 3;
                    }
                    Err(_) => {
                        result.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            other => {
                result.push(other);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn build_query(values: &[(&str, String)]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in values {
        parts.push(format!("{}={}", url_encode(key), url_encode(value)));
    }
    parts.join("&")
}

pub fn read_providers(status: Option<&Value>) -> Vec<OAuthProvider> {
    let mut providers: Vec<OAuthProvider> = Vec::new();
    let Some(status) = status else {
        return providers;
    };
    for (slug, name, flag, id_field) in KNOWN_PROVIDERS {
        if !json::boolean(Some(status), &[flag]) {
            continue;
        }
        let client_id = match id_field {
            None => String::new(),
            Some(field) => json::text(Some(status), &[field]),
        };
        if id_field.is_some() && client_id.is_empty() {
            continue;
        }
        if slug == "oidc" && json::text(Some(status), &["oidc_authorization_endpoint"]).is_empty() {
            continue;
        }
        providers.push(OAuthProvider {
            slug: slug.to_string(),
            name: name.to_string(),
            kind: slug.to_string(),
            client_id,
            endpoint: String::new(),
            scopes: String::new(),
        });
    }
    if let Some(custom) =
        json::get(Some(status), &["custom_oauth_providers"]).and_then(|item| item.as_array())
    {
        for entry in custom {
            let Some(map) = entry.as_object() else {
                continue;
            };
            let value = Value::Object(map.clone());
            let slug = json::text(Some(&value), &["slug"]);
            if slug.is_empty() {
                continue;
            }
            let client_id = json::text(Some(&value), &["client_id"]);
            let endpoint = json::text(Some(&value), &["authorization_endpoint"]);
            if client_id.is_empty() || endpoint.is_empty() {
                continue;
            }
            providers.push(OAuthProvider {
                slug: slug.clone(),
                name: protocol::first_non_empty(&[&json::text(Some(&value), &["name"]), &slug]),
                kind: "custom".to_string(),
                client_id,
                endpoint,
                scopes: json::text(Some(&value), &["scopes"]),
            });
        }
    }
    providers
}

pub fn build_authorize_url(
    site: &Site,
    provider: &OAuthProvider,
    state: &str,
    status: Option<&Value>,
) -> String {
    let slug = provider.slug.as_str();
    let redirect = format!("{}/oauth/{}", site.origin, slug);
    let client_id = provider.client_id.clone();
    if client_id.is_empty() {
        return String::new();
    }
    if slug == "github" {
        return format!(
            "https://github.com/login/oauth/authorize?{}",
            build_query(&[
                ("client_id", client_id),
                ("redirect_uri", redirect),
                ("state", state.to_string()),
                ("scope", "user:email".to_string()),
            ])
        );
    }
    if slug == "linuxdo" {
        return format!(
            "https://connect.linux.do/oauth2/authorize?{}",
            build_query(&[
                ("response_type", "code".to_string()),
                ("client_id", client_id),
                ("redirect_uri", redirect),
                ("state", state.to_string()),
            ])
        );
    }
    if slug == "discord" {
        return format!(
            "https://discord.com/oauth2/authorize?{}",
            build_query(&[
                ("client_id", client_id),
                ("redirect_uri", redirect),
                ("response_type", "code".to_string()),
                ("scope", "identify+openid".to_string()),
                ("state", state.to_string()),
            ])
        );
    }
    if slug == "oidc" {
        let endpoint = status
            .map(|value| json::text(Some(value), &["oidc_authorization_endpoint"]))
            .unwrap_or_default();
        if endpoint.is_empty() {
            return String::new();
        }
        return format!(
            "{endpoint}?{}",
            build_query(&[
                ("client_id", client_id),
                ("redirect_uri", redirect),
                ("response_type", "code".to_string()),
                ("scope", "openid profile email".to_string()),
                ("state", state.to_string()),
            ])
        );
    }
    if provider.endpoint.is_empty() {
        return String::new();
    }
    let mut values: Vec<(&str, String)> = vec![
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("response_type", "code".to_string()),
        ("state", state.to_string()),
    ];
    if !provider.scopes.is_empty() {
        values.push(("scope", provider.scopes.clone()));
    }
    format!("{}?{}", provider.endpoint, build_query(&values))
}
