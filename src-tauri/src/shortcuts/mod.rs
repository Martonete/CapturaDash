//! Atajo global para abrir/cerrar el panel.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::state::AppState;
use crate::windows::panel;

/// Combinaciones reservadas por Windows o de uso universal que no se aceptan.
const RESERVED: &[&str] = &[
    "ALT+TAB", "ALT+F4", "CONTROL+ALT+DELETE", "CTRL+ALT+DELETE", "CONTROL+ESCAPE", "CTRL+ESCAPE",
    "CONTROL+C", "CTRL+C", "CONTROL+V", "CTRL+V", "CONTROL+X", "CTRL+X", "CONTROL+Z", "CTRL+Z",
    "CONTROL+A", "CTRL+A", "CONTROL+S", "CTRL+S",
];

/// Mantener apretado el atajo hace que el sistema lo repita; dentro de este
/// intervalo las repeticiones se ignoran para que el panel no parpadee.
const REPEAT_GUARD: Duration = Duration::from_millis(300);
static LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);

fn on_pressed(app: &AppHandle) {
    let mut last = LAST_TOGGLE.lock().unwrap_or_else(|e| e.into_inner());
    if last.is_some_and(|t| t.elapsed() < REPEAT_GUARD) {
        return;
    }
    *last = Some(Instant::now());
    drop(last);
    panel::toggle(app);
}

pub fn validate(accel: &str) -> Result<Shortcut, String> {
    let norm = accel.replace(' ', "").to_uppercase();
    if norm.contains("SUPER") || norm.contains("META") || norm.contains("WIN") {
        return Err("Las combinaciones con la tecla Windows están reservadas por el sistema".into());
    }
    if RESERVED.contains(&norm.as_str()) {
        return Err("Esa combinación está reservada por Windows o es de uso común".into());
    }
    let has_modifier = ["CTRL", "CONTROL", "ALT", "SHIFT"].iter().any(|m| norm.contains(m));
    let is_function_key = norm.split('+').last().is_some_and(|k| k.starts_with('F') && k.len() <= 3);
    if !has_modifier && !is_function_key {
        return Err("El atajo necesita al menos un modificador (Ctrl, Alt o Shift)".into());
    }
    accel.parse::<Shortcut>().map_err(|e| format!("Atajo inválido: {e}"))
}

/// Registra `accel` reemplazando el anterior. Si falla (p. ej., otra app ya
/// lo usa) se restaura el anterior y se devuelve el error.
pub fn register(app: &AppHandle, accel: &str) -> Result<(), String> {
    let shortcut = validate(accel)?;
    let gs = app.global_shortcut();
    let state = app.state::<AppState>();
    let previous = state.shortcut.lock().take();
    if let Some(prev) = &previous {
        let _ = gs.unregister(prev.as_str());
    }
    let result = gs.on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            on_pressed(app);
        }
    });
    match result {
        Ok(()) => {
            log::info!("atajo global registrado: {accel}");
            *state.shortcut.lock() = Some(accel.to_string());
            Ok(())
        }
        Err(e) => {
            if let Some(prev) = previous {
                let restored = gs.on_shortcut(prev.as_str(), |app, _s, event| {
                    if event.state == ShortcutState::Pressed {
                        on_pressed(app);
                    }
                });
                if restored.is_ok() {
                    *state.shortcut.lock() = Some(prev);
                }
            }
            Err(format!("No se pudo registrar el atajo (¿lo usa otra aplicación?): {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate;

    #[test]
    fn validation() {
        assert!(validate("Alt+Shift+V").is_ok());
        assert!(validate("Ctrl+Alt+Space").is_ok());
        assert!(validate("F9").is_ok());
        assert!(validate("V").is_err());
        assert!(validate("Super+V").is_err());
        assert!(validate("Ctrl+C").is_err());
        assert!(validate("Alt+Tab").is_err());
    }
}
