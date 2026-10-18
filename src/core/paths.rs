use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_NAME: &str = "config.json";

#[derive(Debug, Clone)]
pub struct Paths {
    pub app_directory: PathBuf,
    pub data_directory: PathBuf,
    pub config_path: PathBuf,
}

impl Paths {
    pub fn bind() -> Self {
        let data_directory = resolve_data_directory();
        Self {
            app_directory: resolve_app_directory(),
            config_path: data_directory.join(CONFIG_NAME),
            data_directory,
        }
    }
}

pub fn resolve_app_directory() -> PathBuf {
    match env::current_exe() {
        Ok(path) => match path.parent() {
            Some(parent) => absolute(parent),
            None => absolute(Path::new(".")),
        },
        Err(_) => absolute(Path::new(".")),
    }
}

pub fn resolve_data_directory() -> PathBuf {
    for candidate in candidates() {
        if is_writable(&candidate) {
            return absolute(&candidate);
        }
    }
    absolute(&env::temp_dir().join("ApiCheckinHelper"))
}

pub fn absolute(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) => rest.to_string(),
        None => text.into_owned(),
    }
}

fn candidates() -> Vec<PathBuf> {
    let mut result: Vec<PathBuf> = Vec::new();
    if let Ok(override_path) = env::var("CHECKIN_DATA_DIR") {
        if !override_path.trim().is_empty() {
            result.push(PathBuf::from(override_path));
        }
    }
    if let Ok(root) = env::var("LOCALAPPDATA") {
        if !root.is_empty() {
            result.push(PathBuf::from(root).join("ApiCheckinHelper"));
        }
    }
    if let Ok(root) = env::var("APPDATA") {
        if !root.is_empty() {
            result.push(PathBuf::from(root).join("ApiCheckinHelper"));
        }
    }
    if let Ok(profile) = env::var("USERPROFILE") {
        if !profile.is_empty() {
            result.push(PathBuf::from(profile).join("ApiCheckinHelper"));
        }
    }
    result.push(env::temp_dir().join("ApiCheckinHelper"));
    result
}

fn is_writable(directory: &Path) -> bool {
    if fs::create_dir_all(directory).is_err() {
        return false;
    }
    let probe = directory.join(format!(".write-probe-{}", std::process::id()));
    if fs::write(&probe, "1").is_err() {
        return false;
    }
    fs::remove_file(&probe).is_ok()
}

pub fn ensure_data_directory(directory: &Path) -> std::io::Result<()> {
    fs::create_dir_all(directory)
}
