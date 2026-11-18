//! System tray controller, menu routing, and close-to-tray lifecycle policy.
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::reminders::DueDateSummaryCounts;
use crate::settings::ShellPreferencesDto;

pub const TRAY_ID: &str = "triage-mail-tray";
pub const TRAY_NAVIGATE_EVENT: &str = "tray://navigate";
pub const MENU_OPEN_INBOX: &str = "open_inbox";
pub const MENU_DUE_DATES: &str = "due_dates";
pub const MENU_SUMMARY_STATUS: &str = "summary_status";
pub const MENU_QUIT: &str = "quit";

pub struct TrayController<R: Runtime = tauri::Wry> {
    summary_item: MenuItem<R>,
    _tray: TrayIcon<R>,
}

impl<R: Runtime> TrayController<R> {
    pub fn update_summary(&self, counts: DueDateSummaryCounts) {
        let _ = self
            .summary_item
            .set_text(format_summary_label(counts.overdue, counts.due_today));
    }
}

pub fn format_summary_label(overdue: usize, due_today: usize) -> String {
    format!("Due Today: {due_today} · Overdue: {overdue}")
}

pub fn menu_route_for_id(id: &str) -> Option<&'static str> {
    match id {
        MENU_OPEN_INBOX => Some("/"),
        MENU_DUE_DATES => Some("/due-dates"),
        _ => None,
    }
}

pub fn should_prevent_close(
    quitting: bool,
    service_ready: bool,
    prefs: ShellPreferencesDto,
) -> bool {
    !quitting && service_ready && prefs.close_to_tray
}

pub fn show_and_focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let visible = window.is_visible().unwrap_or(false);
        let minimized = window.is_minimized().unwrap_or(false);
        let focused = window.is_focused().unwrap_or(false);
        if !visible || minimized {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        } else if focused {
            let _ = window.hide();
        } else {
            let _ = window.set_focus();
        }
    }
}

pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayController<R>> {
    let open_inbox = MenuItem::with_id(app, MENU_OPEN_INBOX, "Open Inbox", true, None::<&str>)?;
    let due_dates = MenuItem::with_id(
        app,
        MENU_DUE_DATES,
        "Due Dates & Boletos",
        true,
        None::<&str>,
    )?;
    let summary_item = MenuItem::with_id(
        app,
        MENU_SUMMARY_STATUS,
        format_summary_label(0, 0),
        false,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Completely", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&open_inbox, &due_dates, &summary_item, &separator, &quit],
    )?;

    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("triage-mail")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == MENU_QUIT {
                crate::runtime::request_quit(app);
                return;
            }
            if let Some(route) = menu_route_for_id(id) {
                show_and_focus_main_window(app);
                let _ = app.emit(TRAY_NAVIGATE_EVENT, route);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(TrayController {
        summary_item,
        _tray: tray,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_prevention_requires_unlocked_service_enabled_pref_and_not_quitting() {
        let enabled = ShellPreferencesDto {
            close_to_tray: true,
            minimize_to_tray: false,
        };
        let disabled = ShellPreferencesDto {
            close_to_tray: false,
            minimize_to_tray: true,
        };
        // Pre-unlock closes immediately
        assert!(!should_prevent_close(false, false, enabled));
        // Unlocked with close_to_tray=true prevents close and hides to tray
        assert!(should_prevent_close(false, true, enabled));
        // Explicit quit always permits exit
        assert!(!should_prevent_close(true, true, enabled));
        // Unlocked with close_to_tray=false permits exit
        assert!(!should_prevent_close(false, true, disabled));
    }

    #[test]
    fn formats_summary_and_resolves_allowlisted_navigation_routes() {
        assert_eq!(format_summary_label(2, 5), "Due Today: 5 · Overdue: 2");
        assert_eq!(menu_route_for_id(MENU_OPEN_INBOX), Some("/"));
        assert_eq!(menu_route_for_id(MENU_DUE_DATES), Some("/due-dates"));
        assert_eq!(menu_route_for_id(MENU_QUIT), None);
        assert_eq!(menu_route_for_id(MENU_SUMMARY_STATUS), None);
    }
}
