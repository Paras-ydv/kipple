//! kipple desktop app (SPEC §15): the main window, the menu bar item and its
//! popover window, and the commands the frontend calls.

mod commands;

use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, Position, WindowEvent};

const MAIN: &str = "main";
const TRAY: &str = "tray";
const TRAY_WIDTH: f64 = 380.0;

/// Shows the main window on a route (`#/review`), from the popover.
#[tauri::command]
fn show_main(app: AppHandle, route: String) -> Result<(), String> {
    let win = app
        .get_webview_window(MAIN)
        .ok_or_else(|| "The main window is missing.".to_owned())?;
    if route.starts_with("#/")
        && route
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "#/-_".contains(c))
    {
        let js = format!("window.location.hash = {route:?};");
        win.eval(&js).map_err(|e| e.to_string())?;
    }
    win.show().map_err(|e| e.to_string())?;
    win.unminimize().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_tray(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(TRAY) {
        win.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Opens the popover under the tray icon, or hides it when it is open.
fn toggle_popover(app: &AppHandle, anchor: Option<tauri::Rect>) {
    let Some(win) = app.get_webview_window(TRAY) else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return;
    }
    if let Some(rect) = anchor {
        let scale = win.scale_factor().unwrap_or(1.0);
        let pos: PhysicalPosition<f64> = rect.position.to_physical(scale);
        let size: tauri::PhysicalSize<f64> = rect.size.to_physical(scale);
        let x = pos.x + size.width / 2.0 - TRAY_WIDTH * scale / 2.0;
        let y = pos.y + size.height + 6.0 * scale;
        let _ = win.set_position(Position::Physical(PhysicalPosition::new(
            x.round() as i32,
            y.round() as i32,
        )));
    }
    let _ = win.show();
    let _ = win.set_focus();
}

/// Starts the app. Returns when the app exits.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::integrations,
            commands::scan,
            commands::items,
            commands::projects,
            commands::explain,
            commands::plan,
            commands::execute,
            commands::quarantine,
            commands::restore,
            commands::purge,
            commands::history,
            commands::doctor,
            commands::report,
            commands::cancel,
            commands::volume,
            commands::journal,
            commands::reveal,
            show_main,
            hide_tray,
        ])
        .setup(|app| {
            let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            TrayIconBuilder::with_id("kipple")
                .icon(icon)
                .icon_as_template(true)
                .tooltip("kipple")
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        toggle_popover(tray.app_handle(), Some(rect));
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // The popover closes when it loses focus, like a menu.
            WindowEvent::Focused(false) if window.label() == TRAY => {
                let _ = window.hide();
            }
            // Closing the main window keeps kipple in the menu bar.
            WindowEvent::CloseRequested { api, .. } if window.label() == MAIN => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
}
