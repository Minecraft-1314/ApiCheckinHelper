use std::process::ExitCode;

use api_checkin_helper::shell::uninstall;

fn main() -> ExitCode {
    uninstall::run(std::env::args().skip(1).collect())
}
