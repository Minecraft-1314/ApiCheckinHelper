use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use wry::http::{header, Request, Response};
use wry::{WebContext, WebViewBuilder};

use crate::bridge::command_router::CommandRouter;
use crate::core::config_store::ConfigStore;

pub const VIRTUAL_HOST: &str = "app.localhost";
pub const ENTRY_PAGE: &str = "http://app.localhost/index.html";
pub const MAX_MESSAGE_CHARACTERS: usize = 2_000_000;

const INITIALIZATION_SCRIPT: &str = r#"(function () {
  var listeners = [];
  window.__hostPush = function (text) {
    var data = typeof text === "string" ? JSON.parse(text) : text;
    for (var index = 0; index < listeners.length; index += 1) {
      try { listeners[index]({ data: data }); } catch (error) { }
    }
  };
  var chromeRef = window.chrome || {};
  var nativeWebview = chromeRef.webview;
  var nativePost = nativeWebview && typeof nativeWebview.postMessage === "function"
    ? nativeWebview.postMessage.bind(nativeWebview)
    : null;
  window.chrome = chromeRef;
  window.chrome.webview = {
    postMessage: function (message) {
      if (!nativePost) throw new Error("bridge_unavailable");
      nativePost(typeof message === "string" ? message : JSON.stringify(message));
    },
    addEventListener: function (type, handler) {
      if (type === "message" && typeof handler === "function") listeners.push(handler);
    },
    removeEventListener: function (type, handler) {
      var index = listeners.indexOf(handler);
      if (index >= 0) listeners.splice(index, 1);
    }
  };
})();"#;

pub enum UserEvent {
    Message(String),
}

pub fn run() -> ExitCode {
    let store = ConfigStore::bind();
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let router: Arc<CommandRouter> = Arc::new(CommandRouter::new(Box::new(move |payload| {
        let _ = proxy.send_event(UserEvent::Message(payload.to_string()));
    })));
    let web_directory = web_directory();
    let icon = crate::shell::app_icon::load(store);
    let mut builder = WindowBuilder::new()
        .with_title("API Checkin Helper")
        .with_inner_size(LogicalSize::new(1240.0, 800.0))
        .with_min_inner_size(LogicalSize::new(720.0, 620.0));
    if let Some(value) = icon {
        builder = builder.with_window_icon(Some(value));
    }
    let window = match builder.build(&event_loop) {
        Ok(value) => value,
        Err(error) => {
            store
                .log()
                .handle_unhandled("WindowError", &error.to_string(), &error.to_string());
            return ExitCode::from(1);
        }
    };
    let mut context = WebContext::new(Some(user_data_folder()));
    let view = match build_view(
        &window,
        &router,
        web_directory,
        event_loop.create_proxy(),
        &mut context,
    ) {
        Ok(value) => value,
        Err(error) => {
            store
                .log()
                .handle_unhandled("WebViewError", &error.to_string(), &error.to_string());
            show_fatal(store, &error.to_string());
            return ExitCode::from(1);
        }
    };
    if let Err(error) = view.load_url(ENTRY_PAGE) {
        store
            .log()
            .handle_unhandled("NavigationError", &error.to_string(), &error.to_string());
    }
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Message(text)) => {
                if let Some(response) = dispatch(&router, &text) {
                    let _ = view.evaluate_script(&build_push(&response));
                }
            }
            Event::LoopDestroyed => {
                router.dispose();
                let _ = store.flush();
            }
            _ => {}
        }
    });
}

fn build_view(
    window: &tao::window::Window,
    router: &Arc<CommandRouter>,
    directory: PathBuf,
    proxy: EventLoopProxy<UserEvent>,
    context: &mut WebContext,
) -> wry::Result<wry::WebView> {
    let navigation_router = Arc::clone(router);
    WebViewBuilder::with_web_context(context)
        .with_custom_protocol("app".to_string(), move |_id, request| {
            protocol_handler(&directory, request)
        })
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_devtools(false)
        .with_navigation_handler(move |url| {
            if is_internal_url(&url) {
                true
            } else {
                let _ = crate::bridge::external_launcher::open(navigation_router.store(), &url);
                false
            }
        })
        .with_ipc_handler(move |request| {
            let text = request.body().clone();
            let _ = proxy.send_event(UserEvent::Message(text));
        })
        .build(window)
}

pub fn request_path(uri: &wry::http::Uri) -> String {
    let raw = uri.to_string();
    let authority = raw.find("://").map(|index| index + 3).unwrap_or(0);
    let after = &raw[authority..];
    let start = after.find('/').unwrap_or(after.len());
    let path = &after[start..];
    let path = match path.find(['?', '#']) {
        Some(index) => &path[..index],
        None => path,
    };
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        "index.html".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn is_internal_url(url: &str) -> bool {
    let lowered = url.to_ascii_lowercase();
    lowered.starts_with("http://app.localhost/")
        || lowered.starts_with("https://app.localhost/")
        || lowered.starts_with("app://")
        || lowered == "http://app.localhost"
        || lowered == "https://app.localhost"
}

fn build_push(payload: &str) -> String {
    let encoded = serde_json::to_string(payload).unwrap_or_else(|_| "\"\"".to_string());
    format!("window.__hostPush({encoded})")
}

fn dispatch(router: &CommandRouter, text: &str) -> Option<String> {
    let store = ConfigStore::global();
    if text.chars().count() > MAX_MESSAGE_CHARACTERS {
        return Some(error_response(0, "界面消息超过大小上限"));
    }
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            store.log().warn(&format!("界面消息格式错误：{error}"));
            return Some(error_response(0, "界面消息格式错误"));
        }
    };
    let id = value.get("id").and_then(|item| item.as_i64()).unwrap_or(0);
    let action = value
        .get("action")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    if action.chars().count() > 64 {
        return Some(error_response(id, "界面操作名称无效"));
    }
    let payload = value
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match router.execute(&action, &payload) {
        Ok(result) => Some(
            serde_json::json!({ "id": id, "ok": true, "result": result, "error": serde_json::Value::Null })
                .to_string(),
        ),
        Err(error) => Some(error_response(id, &error.message())),
    }
}

fn error_response(id: i64, message: &str) -> String {
    serde_json::json!({ "id": id, "ok": false, "result": serde_json::Value::Null, "error": message })
        .to_string()
}

fn protocol_handler(directory: &Path, request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let relative = request_path(request.uri());
    if relative.contains("..") {
        return forbidden();
    }
    let target = directory.join(&relative);
    let resolved = match target.canonicalize() {
        Ok(value) => value,
        Err(_) => return not_found(),
    };
    let root = match directory.canonicalize() {
        Ok(value) => value,
        Err(_) => return not_found(),
    };
    if !resolved.starts_with(&root) {
        return forbidden();
    }
    match std::fs::read(&resolved) {
        Ok(body) => Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, content_type(&resolved))
            .body(Cow::Owned(body))
            .unwrap_or_else(|_| not_found()),
        Err(_) => not_found(),
    }
}

fn content_type(path: &Path) -> String {
    let extension = path
        .extension()
        .map(|item| item.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn forbidden() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(403)
        .body(Cow::Borrowed(&b"forbidden"[..]))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])))
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(404)
        .body(Cow::Borrowed(&b"not found"[..]))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])))
}

fn show_fatal(store: &'static ConfigStore, detail: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let text: Vec<u16> = format!("界面无法启动，详情已写入本地日志。\n{detail}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let caption: Vec<u16> = "API 签到助手"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    store.log().error("界面无法启动", Some(detail));
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn web_directory() -> PathBuf {
    crate::core::paths::resolve_app_directory()
        .join("Ui")
        .join("Web")
}

pub fn user_data_folder() -> PathBuf {
    std::env::temp_dir()
        .join("ApiCheckinHelper")
        .join("WebView2")
}
