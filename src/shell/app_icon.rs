use std::path::Path;

use tao::window::Icon;

use crate::core::config_store::ConfigStore;

pub const ICON_NAME: &str = "app.ico";

pub fn load(store: &ConfigStore) -> Option<Icon> {
    let directory = crate::core::paths::resolve_app_directory();
    let path = directory.join(ICON_NAME);
    if !path.exists() {
        return None;
    }
    match decode(&path) {
        Ok(icon) => Some(icon),
        Err(message) => {
            store.log().warn(&format!("窗口图标加载失败：{message}"));
            None
        }
    }
}

fn decode(path: &Path) -> Result<Icon, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let entries =
        ico::IconDir::read(std::io::Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let mut best: Option<&ico::IconDirEntry> = None;
    for entry in entries.entries() {
        let is_better = match best {
            None => true,
            Some(current) => {
                let candidate = entry.width() as i64 * entry.height() as i64;
                let existing = current.width() as i64 * current.height() as i64;
                candidate > existing
            }
        };
        if is_better {
            best = Some(entry);
        }
    }
    let entry = best.ok_or_else(|| "图标文件为空".to_string())?;
    let image = entry.decode().map_err(|error| error.to_string())?;
    Icon::from_rgba(image.rgba_data().to_vec(), image.width(), image.height())
        .map_err(|error| error.to_string())
}
