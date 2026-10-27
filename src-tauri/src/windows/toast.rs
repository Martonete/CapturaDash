//! Aviso pequeño y discreto en la esquina inferior derecha ("Captura guardada").
//! Se muestra sin activar la ventana para no interrumpir lo que se está haciendo.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use canvasclip_core::models::Capture;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

use crate::events::{ToastPayload, TOAST_SHOW};

pub const LABEL: &str = "toast";
const WIDTH: f64 = 300.0;
const HEIGHT: f64 = 72.0;
const MARGIN: f64 = 14.0;
const VISIBLE_FOR: Duration = Duration::from_millis(2600);

static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn show_capture(app: &AppHandle, capture: Capture) {
    show(app, ToastPayload { kind: "capture", capture: Some(capture), message: None });
}

pub fn show_message(app: &AppHandle, kind: &'static str, message: String) {
    show(app, ToastPayload { kind, capture: None, message: Some(message) });
}

fn show(app: &AppHandle, payload: ToastPayload) {
    let Some(win) = app.get_webview_window(LABEL) else { return };
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|c| app.monitor_from_point(c.x, c.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let s = m.scale_factor();
        let wa = m.work_area();
        let (w, h) = ((WIDTH * s).round() as i32, (HEIGHT * s).round() as i32);
        let margin = (MARGIN * s).round() as i32;
        let x = wa.position.x + wa.size.width as i32 - w - margin;
        let y = wa.position.y + wa.size.height as i32 - h - margin;
        let _ = win.set_position(PhysicalPosition::new(x, y));
        let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
    }
    let _ = win.emit_to(LABEL, TOAST_SHOW, payload);

    #[cfg(windows)]
    let shown = super::native::raw_hwnd(&win).map(super::native::show_no_activate).is_some();
    #[cfg(not(windows))]
    let shown = false;
    if !shown {
        let _ = win.show();
    }

    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(VISIBLE_FOR);
        if GENERATION.load(Ordering::SeqCst) == generation {
            hide(&app);
        }
    });
}

pub fn hide(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.hide();
    }
}
