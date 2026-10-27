//! Ubicación de la carpeta de datos. Se guarda fuera de la base de datos
//! (en la carpeta de configuración) porque la base vive dentro de esa carpeta.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const FILE: &str = "location.json";

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Location {
    data_dir: Option<PathBuf>,
}

fn config_file(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_config_dir()?.join(FILE))
}

pub fn default_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    app.path().app_local_data_dir()
}

/// Carpeta configurada por el usuario o la predeterminada (%LOCALAPPDATA%\<id>).
pub fn data_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    let custom = config_file(app)
        .ok()
        .and_then(|f| fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str::<Location>(&s).ok())
        .and_then(|l| l.data_dir);
    match custom {
        Some(dir) => Ok(dir),
        None => default_dir(app),
    }
}

pub fn set_data_dir(app: &AppHandle, dir: PathBuf) -> Result<(), String> {
    let file = config_file(app).map_err(|e| e.to_string())?;
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&Location { data_dir: Some(dir) }).map_err(|e| e.to_string())?;
    canvasclip_core::storage::write_atomic(&file, json.as_bytes()).map_err(|e| e.to_string())
}
