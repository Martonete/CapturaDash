//! Ventanas de la aplicación: principal (pizarrón), panel lateral y aviso.

pub mod edge;
#[cfg(windows)]
pub mod native;
pub mod panel;
pub mod toast;

use serde::Serialize;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::events::MAIN_NAVIGATE;

pub const MAIN: &str = "main";

#[derive(Serialize, Clone)]
struct Navigate<'a> {
    view: &'a str,
}

/// Muestra y enfoca la ventana principal; `view` = "board" | "settings".
pub fn show_main(app: &AppHandle, view: Option<&str>) {
    let Some(win) = app.get_webview_window(MAIN) else { return };
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
    if let Some(view) = view {
        let _ = win.emit_to(MAIN, MAIN_NAVIGATE, Navigate { view });
    }
}

/// Muestra el panel o el aviso. Sin `activate`, en Windows se muestra sin
/// quitarle el foco a la aplicación en uso (`SW_SHOWNOACTIVATE`).
///
/// tao (el motor de ventanas de Tauri) guarda su propio estado de
/// visibilidad y omite `show`/`hide` si cree que no cambia nada; como acá
/// también se muestra de forma nativa, cada operación se hace por las dos
/// vías para que ninguna quede sin efecto.
pub fn show_aux(win: &WebviewWindow, activate: bool) {
    if activate {
        let _ = win.show();
        #[cfg(windows)]
        if let Some(h) = native::raw_hwnd(win) {
            native::show(h);
        }
        let _ = win.set_focus();
        return;
    }
    #[cfg(windows)]
    match native::raw_hwnd(win) {
        Some(h) => native::show_no_activate(h),
        None => {
            let _ = win.show();
        }
    }
    #[cfg(not(windows))]
    {
        let _ = win.show();
    }
}

pub fn hide_aux(win: &WebviewWindow) {
    #[cfg(windows)]
    if let Some(h) = native::raw_hwnd(win) {
        native::hide(h);
    }
    let _ = win.hide();
}

/// Ajustes nativos comunes al crear las ventanas.
pub fn prepare(app: &AppHandle, effects: &str) {
    for label in [MAIN, panel::LABEL, toast::LABEL] {
        if let Some(win) = app.get_webview_window(label) {
            #[cfg(windows)]
            if let Some(h) = native::raw_hwnd(&win) {
                native::round_corners(h);
            }
            if label != MAIN {
                apply_effects(&win, effects);
            }
        }
    }
}

/// Vidrio esmerilado nativo (Acrylic) para panel y aviso; "off" lo desactiva.
/// La intensidad visual se completa con la opacidad del fondo en CSS.
pub fn apply_effects(win: &WebviewWindow, effects: &str) {
    let result = if effects == "off" {
        win.set_effects(None)
    } else {
        win.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build())
    };
    if let Err(e) = result {
        log::warn!("no se pudieron aplicar efectos a {}: {e}", win.label());
    }
}

pub fn apply_effects_all(app: &AppHandle, effects: &str) {
    for label in [panel::LABEL, toast::LABEL] {
        if let Some(win) = app.get_webview_window(label) {
            apply_effects(&win, effects);
        }
    }
}
