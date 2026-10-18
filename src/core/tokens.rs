use super::auth;
use super::model::Credential;

pub fn mask_token(token: Option<&str>) -> String {
    mask(&auth::strip_bearer(token.unwrap_or("")), 4, 4)
}

pub fn describe(credential: Option<&Credential>) -> String {
    match credential {
        None => "未配置".to_string(),
        Some(item) if item.token.is_empty() => "未配置".to_string(),
        Some(item) => format!("令牌 {}", mask_token(Some(&item.token))),
    }
}

fn mask(value: &str, head_keep: usize, tail_keep: usize) -> String {
    let characters: Vec<char> = value.chars().collect();
    if characters.is_empty() {
        return String::new();
    }
    if characters.len() <= head_keep + tail_keep {
        return "•".repeat(4.max(characters.len()));
    }
    let prefix: String = if head_keep > 0 {
        characters[..head_keep].iter().collect()
    } else {
        String::new()
    };
    let suffix: String = if tail_keep > 0 {
        characters[characters.len() - tail_keep..].iter().collect()
    } else {
        String::new()
    };
    let dots = 16.min(4.max(characters.len() - head_keep - tail_keep));
    format!("{prefix}{}{suffix}", "•".repeat(dots))
}
