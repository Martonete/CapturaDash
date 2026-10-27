//! Panel lateral flotante: posicionamiento, apertura con o sin foco y cierre.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::events::{PANEL_CLOSE, PANEL_OPENED};
use crate::state::AppState;

pub const LABEL: &str = "panel";
/// Ancho lógico del panel (px a 100 %); se ajusta si la pantalla es angosta.
const WIDTH: f64 = 360.0;
const MARGIN: f64 = 8.0;
/// Tiempo que espera Rust a que la interfaz termine la animación de cierre.
const CLOSE_FALLBACK: Duration = Duration::from_millis(350);
/// Pérdidas de foco inmediatamente posteriores a la apertura se ignoran
/// (rebotes de foco del sistema, la tecla del atajo al soltarse, etc.).
const BLUR_GRACE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Hidden = 0,
    /// Abierto por el borde: sin foco, se cierra al alejar el cursor.
    Hover = 1,
    /// Abierto por atajo/bandeja o el usuario hizo clic dentro.
    Focused = 2,
}

#[derive(Default)]
pub struct PanelState {
    mode: AtomicU8,
    closing: AtomicBool,
    /// El panel recibió el foco desde la última apertura. Solo entonces un
    /// "perdió el foco" significa que el usuario hizo clic afuera (evita
    /// cierres espurios si el sistema no le concede el foco al abrir).
    had_focus: AtomicBool,
    /// Se incrementa en cada apertura; invalida cierres pendientes anteriores.
    generation: AtomicU64,
    /// Rectángulo físico actual (x, y, ancho, alto) + borde derecho del monitor.
    rect: Mutex<Option<(i32, i32, i32, i32, i32)>>,
    opened_at: Mutex<Option<Instant>>,
}

impl PanelState {
    pub fn mode(&self) -> Mode {
        match self.mode.load(Ordering::SeqCst) {
            1 => Mode::Hover,
            2 => Mode::Focused,
            _ => Mode::Hidden,
        }
    }

    fn set_mode(&self, m: Mode) {
        self.mode.store(m as u8, Ordering::SeqCst);
    }

    pub fn is_open(&self) -> bool {
        self.mode() != Mode::Hidden && !self.closing.load(Ordering::SeqCst)
    }

    /// ¿El punto está sobre el panel o en el margen entre el panel y el borde?
    pub fn contains(&self, x: i32, y: i32, slack: i32) -> bool {
        match *self.rect.lock() {
            Some((px, py, w, h, edge)) => {
                x >= px - slack && x <= edge.max(px + w) && y >= py - slack && y <= py + h + slack
            }
            None => false,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OpenedPayload {
    focused: bool,
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

/// Ubica el panel en el borde derecho del área de trabajo del monitor donde
/// está el cursor (respeta la barra de tareas y el escalado de ese monitor).
fn place(app: &AppHandle, win: &WebviewWindow) -> tauri::Result<()> {
    let cursor = app.cursor_position().ok();
    let monitor = match cursor {
        Some(c) => app.monitor_from_point(c.x, c.y)?,
        None => None,
    }
    .or(app.primary_monitor()?);
    let Some(monitor) = monitor else { return Ok(()) };

    let scale = monitor.scale_factor();
    let wa = monitor.work_area();
    let (wx, wy) = (wa.position.x, wa.position.y);
    let (ww, wh) = (wa.size.width as i32, wa.size.height as i32);
    let margin = (MARGIN * scale).round() as i32;
    let width = ((WIDTH * scale).round() as i32).min(ww - 2 * margin).max(200);
    let height = (wh - 2 * margin).max(200);
    let x = wx + ww - width - margin;
    let y = wy + margin;
    let monitor_right = monitor.position().x + monitor.size().width as i32;

    // Posición antes que tamaño: al cruzar a un monitor con otro DPI, Windows
    // reescala la ventana al moverla.
    win.set_position(PhysicalPosition::new(x, y))?;
    win.set_size(PhysicalSize::new(width as u32, height as u32))?;
    *app.state::<AppState>().panel.rect.lock() = Some((x, y, width, height, monitor_right));
    Ok(())
}

pub fn open(app: &AppHandle, focus: bool) {
    let Some(win) = window(app) else {
        log::error!("ventana del panel no disponible");
        return;
    };
    log::info!("panel: abrir (con foco: {focus})");
    let state = app.state::<AppState>();
    state.panel.generation.fetch_add(1, Ordering::SeqCst);
    state.panel.closing.store(false, Ordering::SeqCst);
    state.panel.had_focus.store(false, Ordering::SeqCst);
    *state.panel.opened_at.lock() = Some(Instant::now());

    if state.panel.mode() == Mode::Hidden {
        if let Err(e) = place(app, &win) {
            log::warn!("no se pudo ubicar el panel: {e}");
        }
    }
    state.panel.set_mode(if focus { Mode::Focused } else { Mode::Hover });

    #[cfg(windows)]
    let shown_native = if !focus {
        crate::windows::native::raw_hwnd(&win).map(crate::windows::native::show_no_activate).is_some()
    } else {
        false
    };
    #[cfg(not(windows))]
    let shown_native = false;

    if !shown_native {
        if let Err(e) = win.show() {
            log::error!("no se pudo mostrar el panel: {e}");
        }
    }
    if focus {
        let _ = win.set_focus();
    }
    let _ = win.emit_to(LABEL, PANEL_OPENED, OpenedPayload { focused: focus });
}

/// Pide a la interfaz que cierre con animación; si no responde a tiempo, se oculta igual.
pub fn request_close(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.panel.mode() == Mode::Hidden || state.panel.closing.swap(true, Ordering::SeqCst) {
        return;
    }
    log::info!("panel: cerrar");
    let generation = state.panel.generation.load(Ordering::SeqCst);
    let _ = app.emit_to(LABEL, PANEL_CLOSE, ());
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(CLOSE_FALLBACK);
        let state = app.state::<AppState>();
        if state.panel.generation.load(Ordering::SeqCst) == generation
            && state.panel.closing.load(Ordering::SeqCst)
        {
            hide_now(&app);
        }
    });
}

/// La interfaz terminó su animación de cierre. Si mientras tanto el panel se
/// volvió a abrir, la orden llega tarde y se ignora.
pub fn finish_close(app: &AppHandle) {
    if app.state::<AppState>().panel.closing.load(Ordering::SeqCst) {
        hide_now(app);
    }
}

pub fn hide_now(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.panel.set_mode(Mode::Hidden);
    state.panel.closing.store(false, Ordering::SeqCst);
    if let Some(win) = window(app) {
        let _ = win.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    let open = app.state::<AppState>().panel.is_open();
    if open {
        request_close(app);
    } else {
        self::open(app, true);
    }
}

/// Cambios de foco de la ventana del panel.
pub fn on_focus_changed(app: &AppHandle, focused: bool) {
    let state = app.state::<AppState>();
    if focused {
        state.panel.had_focus.store(true, Ordering::SeqCst);
        if state.panel.mode() == Mode::Hover {
            state.panel.set_mode(Mode::Focused);
        }
    } else if state.panel.mode() == Mode::Focused
        && state.panel.had_focus.load(Ordering::SeqCst)
        && state.panel.opened_at.lock().is_none_or(|t| t.elapsed() >= BLUR_GRACE)
        && state.settings.read().close_behavior == "auto"
    {
        request_close(app);
    }
}
