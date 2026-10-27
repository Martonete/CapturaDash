//! Icono de la bandeja del sistema.

use std::time::Duration;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::events::APP_FLUSH;
use crate::state::AppState;
use crate::windows::{self, panel};

pub fn build(app: &AppHandle, monitoring: bool) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir pizarrón", true, None::<&str>)?;
    let show_panel = MenuItem::with_id(app, "panel", "Mostrar historial", true, None::<&str>)?;
    let monitor = CheckMenuItem::with_id(app, "monitor", "Captura automática", true, monitoring, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Ajustes…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Salir de CanvasClip", true, None::<&str>)?;
    let sep = || PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(app, &[&open, &show_panel, &sep()?, &monitor, &settings, &sep()?, &quit_item])?;
    *app.state::<AppState>().tray_monitor_item.lock() = Some(monitor);

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("CanvasClip")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => windows::show_main(app, Some("board")),
            "panel" => panel::open(app, true),
            "settings" => windows::show_main(app, Some("settings")),
            "monitor" => {
                let enabled = !app.state::<AppState>().settings.read().monitoring;
                if let Err(e) = crate::commands::settings::set_monitoring(app, enabled) {
                    log::warn!("no se pudo cambiar el monitoreo: {e}");
                }
            }
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                windows::show_main(tray.app_handle(), None);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn set_monitoring_checked(app: &AppHandle, checked: bool) {
    if let Some(item) = app.state::<AppState>().tray_monitor_item.lock().as_ref() {
        let _ = item.set_checked(checked);
    }
}

/// Salida ordenada: la interfaz guarda cambios pendientes antes de cerrar.
pub fn quit(app: &AppHandle) {
    let _ = app.emit(APP_FLUSH, ());
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        app.exit(0);
    });
}
