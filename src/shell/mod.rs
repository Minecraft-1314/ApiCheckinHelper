pub mod app_icon;
pub mod uninstall;
pub mod web_window;

use std::process::ExitCode;

pub fn run() -> ExitCode {
    web_window::run()
}
