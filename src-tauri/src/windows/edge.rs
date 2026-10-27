//! Activación del panel al apoyar el cursor en el borde derecho de la pantalla.
//!
//! Un hilo nativo consulta la posición del cursor cada 40 ms (GetCursorPos es
//! una llamada trivial; no hay hooks globales de mouse que puedan agregar
//! latencia al sistema). Reglas para evitar aperturas accidentales:
//! * solo cuenta el borde "real": sin otro monitor a la derecha;
//! * se excluyen las esquinas (botón cerrar de ventanas maximizadas, reloj);
//! * el cursor debe permanecer quieto en la zona durante `delay`;
//! * no se abre con un botón del mouse presionado (arrastres, selección);
//! * no se abre con apps a pantalla completa ni con la sesión bloqueada;
//! * después de cerrarse, hay que salir de la zona para volver a armarlo.

use std::time::{Duration, Instant};

pub const TICK: Duration = Duration::from_millis(40);
/// Fracción superior e inferior del borde excluida.
const CORNER_FRACTION: f64 = 0.12;
/// Distancia en px físicos desde el borde que cuenta como "en el borde".
const EDGE_PX: i32 = 2;
/// Movimiento vertical máximo tolerado durante la espera.
const MAX_DRIFT_PX: i32 = 160;
/// Tiempo fuera del panel antes de cerrarlo (si se abrió por el borde).
pub const LEAVE_DELAY: Duration = Duration::from_millis(450);

#[derive(Debug, Clone, Copy)]
pub struct MonitorRect {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// ¿El punto está en la zona activa del borde derecho de este monitor?
pub fn in_zone(x: i32, y: i32, m: MonitorRect, neighbor_to_right: bool) -> bool {
    if neighbor_to_right || x < m.right - EDGE_PX {
        return false;
    }
    let h = (m.bottom - m.top) as f64;
    let top = m.top as f64 + h * CORNER_FRACTION;
    let bottom = m.bottom as f64 - h * CORNER_FRACTION;
    (y as f64) >= top && (y as f64) <= bottom
}

/// Máquina de estados de la espera en el borde (independiente de Win32 para
/// poder probarla).
#[derive(Debug, Default)]
pub struct DwellTracker {
    since: Option<(Instant, i32)>,
    /// false tras una apertura hasta que el cursor salga de la zona.
    disarmed: bool,
}

impl DwellTracker {
    /// Devuelve true cuando corresponde abrir el panel.
    pub fn update(&mut self, now: Instant, in_zone: bool, y: i32, button_down: bool, delay: Duration) -> bool {
        if !in_zone {
            self.since = None;
            self.disarmed = false;
            return false;
        }
        if self.disarmed || button_down {
            self.since = None;
            return false;
        }
        match self.since {
            None => {
                self.since = Some((now, y));
                false
            }
            Some((_, y0)) if (y - y0).abs() > MAX_DRIFT_PX => {
                // Recorrido rápido a lo largo del borde: reiniciar la espera.
                self.since = Some((now, y));
                false
            }
            Some((t0, _)) if now.duration_since(t0) >= delay => {
                self.since = None;
                self.disarmed = true;
                true
            }
            _ => false,
        }
    }

    pub fn disarm(&mut self) {
        self.since = None;
        self.disarmed = true;
    }
}

#[cfg(windows)]
pub fn start(app: tauri::AppHandle) {
    use tauri::Manager;

    use super::native;
    use super::panel::{self, Mode};
    use crate::state::AppState;

    std::thread::Builder::new()
        .name("canvasclip-edge".into())
        .spawn(move || {
            let mut tracker = DwellTracker::default();
            let mut outside_since: Option<Instant> = None;
            loop {
                std::thread::sleep(TICK);
                let state = app.state::<AppState>();
                let (enabled, delay, auto_close) = {
                    let s = state.settings.read();
                    (s.edge_enabled, Duration::from_millis(s.edge_delay_ms as u64), s.close_behavior == "auto")
                };
                let Some((x, y)) = native::cursor_pos() else { continue };
                let now = Instant::now();
                let button = native::mouse_button_down();

                match state.panel.mode() {
                    Mode::Hover => {
                        // Abierto por el borde y sin foco: cerrar al alejarse.
                        tracker.disarm();
                        let inside = state.panel.contains(x, y, 24);
                        if inside || button || !auto_close {
                            outside_since = None;
                        } else {
                            let t0 = *outside_since.get_or_insert(now);
                            if now.duration_since(t0) >= LEAVE_DELAY {
                                outside_since = None;
                                panel::request_close(&app);
                            }
                        }
                        continue;
                    }
                    Mode::Focused => {
                        tracker.disarm();
                        outside_since = None;
                        continue;
                    }
                    Mode::Hidden => outside_since = None,
                }

                if !enabled {
                    continue;
                }
                let Some(r) = native::monitor_rect(x, y) else { continue };
                let rect = MonitorRect { top: r.top, right: r.right, bottom: r.bottom };
                let zone = in_zone(x, y, rect, native::monitor_at(r.right, y));
                if tracker.update(now, zone, y, button, delay) && !native::user_busy() {
                    panel::open(&app, false);
                }
            }
        })
        .expect("hilo del borde");
}

#[cfg(not(windows))]
pub fn start(_app: tauri::AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;

    const M: MonitorRect = MonitorRect { top: 0, right: 1920, bottom: 1080 };
    const D: Duration = Duration::from_millis(300);

    #[test]
    fn zone_rules() {
        assert!(in_zone(1919, 540, M, false));
        assert!(!in_zone(1900, 540, M, false), "lejos del borde");
        assert!(!in_zone(1919, 540, M, true), "hay otro monitor a la derecha");
        assert!(!in_zone(1919, 10, M, false), "esquina superior");
        assert!(!in_zone(1919, 1075, M, false), "esquina inferior");
        // Monitor secundario a la izquierda con coordenadas negativas.
        let left = MonitorRect { top: -200, right: 0, bottom: 1240 };
        assert!(in_zone(-1, 500, left, false));
    }

    #[test]
    fn dwell_requires_time_and_rearm() {
        let mut t = DwellTracker::default();
        let t0 = Instant::now();
        assert!(!t.update(t0, true, 500, false, D));
        assert!(!t.update(t0 + Duration::from_millis(200), true, 500, false, D));
        assert!(t.update(t0 + Duration::from_millis(320), true, 505, false, D));
        // Sigue en la zona: no vuelve a disparar.
        assert!(!t.update(t0 + Duration::from_millis(900), true, 505, false, D));
        // Sale y vuelve: se rearma.
        assert!(!t.update(t0 + Duration::from_millis(950), false, 505, false, D));
        assert!(!t.update(t0 + Duration::from_millis(1000), true, 505, false, D));
        assert!(t.update(t0 + Duration::from_millis(1400), true, 505, false, D));
    }

    #[test]
    fn dwell_cancelled_by_button_or_fast_travel() {
        let mut t = DwellTracker::default();
        let t0 = Instant::now();
        t.update(t0, true, 300, false, D);
        assert!(!t.update(t0 + Duration::from_millis(400), true, 300, true, D), "botón presionado");
        t.update(t0 + Duration::from_millis(450), true, 300, false, D);
        assert!(!t.update(t0 + Duration::from_millis(800), true, 700, false, D), "deslizó por el borde");
        assert!(t.update(t0 + Duration::from_millis(1150), true, 700, false, D));
    }
}
