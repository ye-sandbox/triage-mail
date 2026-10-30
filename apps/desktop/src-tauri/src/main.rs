#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "native-runtime")]
    triage_mail_desktop_lib::runtime::run();
}
