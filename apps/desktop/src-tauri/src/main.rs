#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let (status, _, _) = triage_mail_desktop_lib::workspace_bridge_health();
    if status != "ok" {
        std::process::exit(1);
    }
}
