use std::path::PathBuf;
use std::time::Duration;

use canvasclip_core::ingest;
use canvasclip_core::maintenance;
use canvasclip_core::settings::Settings;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use super::{core_err, CmdResult};
use crate::events::{self, CapturesChanged, APP_FLUSH, SETTINGS_CHANGED};
use crate::state::AppState;
use crate::{location, shortcuts, tray, windows};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CmdResult<Settings> {
    Ok(state.settings())
}

/// Aplica y guarda la configuración. Los efectos secundarios (atajo, inicio
/// con Windows, efectos, retención) se aplican antes de guardar; si el atajo
/// no se puede registrar, no se guarda nada y se informa el error.
#[tauri::command]
pub async fn update_settings(app: AppHandle, settings: Settings) -> CmdResult<Settings> {
    apply(&app, settings)
}

pub fn apply(app: &AppHandle, new: Settings) -> CmdResult<Settings> {
    let state = app.state::<AppState>();
    let new = new.normalized();
    let old = state.settings();

    if new.shortcut != old.shortcut {
        shortcuts::register(app, &new.shortcut)?;
    }
    if new.launch_at_startup != old.launch_at_startup {
        sync_autostart(app, new.launch_at_startup);
    }
    if new.monitoring != old.monitoring {
        tray::set_monitoring_checked(app, new.monitoring);
    }
    if new.effects != old.effects {
        windows::apply_effects_all(app, &new.effects);
    }

    state.db.save_settings(&new).map_err(core_err)?;
    *state.settings.write() = new.clone();

    if new.history_limit < old.history_limit {
        let pruned = ingest::apply_retention(&state.db, &state.storage, new.history_limit).map_err(core_err)?;
        if !pruned.is_empty() {
            events::captures_changed(app, CapturesChanged { removed: pruned, ..Default::default() });
        }
    }
    let _ = app.emit(SETTINGS_CHANGED, &new);
    Ok(new)
}

pub fn sync_autostart(app: &AppHandle, enabled: bool) {
    let al = app.autolaunch();
    let result = if enabled { al.enable() } else { al.disable() };
    if let Err(e) = result {
        log::warn!("no se pudo cambiar el inicio con Windows: {e}");
    }
}

pub fn set_monitoring(app: &AppHandle, enabled: bool) -> CmdResult<Settings> {
    let mut s = app.state::<AppState>().settings();
    s.monitoring = enabled;
    apply(app, s)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub default_data_dir: String,
    pub platform: &'static str,
}

#[tauri::command]
pub async fn app_info(app: AppHandle, state: State<'_, AppState>) -> CmdResult<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.storage.root().to_string_lossy().to_string(),
        default_data_dir: location::default_dir(&app).map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        platform: std::env::consts::OS,
    })
}

/// Copia todos los datos a otra carpeta, la deja configurada y reinicia.
/// La carpeta anterior no se borra.
#[tauri::command]
pub async fn change_data_dir(app: AppHandle, path: String) -> CmdResult<()> {
    let dest = PathBuf::from(&path);
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app2.state::<AppState>();
        maintenance::copy_data_dir(&state.db, &state.storage, &dest).map_err(core_err)?;
        location::set_data_dir(&app2, dest)
    })
    .await
    .map_err(super::join_err)??;
    let _ = app.emit(APP_FLUSH, ());
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        app.restart();
    });
    Ok(())
}

#[tauri::command]
pub async fn open_data_dir(state: State<'_, AppState>) -> CmdResult<()> {
    let dir = state.storage.root().to_path_buf();
    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(dir).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    {
        let _ = dir;
    }
    Ok(())
}
