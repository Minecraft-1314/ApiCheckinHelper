use std::path::Path;

use crate::core::config_store::ConfigStore;
use crate::core::model_rules;
use crate::core::network_policy;

pub fn open(store: &ConfigStore, target: &str) -> bool {
    if target.trim().is_empty() {
        return false;
    }
    let value = target.trim();
    if open_data_directory(store, Some(value)) {
        return true;
    }
    let Ok(uri) = url::Url::parse(value) else {
        return false;
    };
    if uri.scheme() != "http" && uri.scheme() != "https" {
        return false;
    }
    if !uri.username().is_empty() || uri.password().is_some() {
        return false;
    }
    let Some(host) = uri.host_str() else {
        return false;
    };
    if !model_rules::is_valid_host(network_policy::strip_brackets(host)) {
        return false;
    }
    start(uri.as_str(), store)
}

pub fn open_data_directory(store: &ConfigStore, target: Option<&str>) -> bool {
    let directory = match target {
        Some(value) => value.to_string(),
        None => store.data_directory().to_string_lossy().into_owned(),
    };
    if directory.trim().is_empty() || !Path::new(&directory).is_dir() {
        store.log().warn(&format!("数据目录不存在：{directory}"));
        return false;
    }
    start(&directory, store)
}

fn start(target: &str, store: &ConfigStore) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let file: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = result as isize;
    if code <= 32 {
        store.log().error(
            "无法打开外部目标",
            Some(&format!("ShellExecuteW 返回 {code}")),
        );
        return false;
    }
    true
}
