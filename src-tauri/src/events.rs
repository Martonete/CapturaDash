//! Nombres y cargas de los eventos que Rust emite hacia las ventanas.

use canvasclip_core::models::{BoardObject, Capture};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const CAPTURES_CHANGED: &str = "captures:changed";
pub const SETTINGS_CHANGED: &str = "settings:changed";
pub const BOARDS_CHANGED: &str = "boards:changed";
pub const BOARD_OBJECTS_INSERTED: &str = "board:objects-inserted";
pub const PANEL_OPENED: &str = "panel:opened";
pub const PANEL_CLOSE: &str = "panel:close";
pub const TOAST_SHOW: &str = "toast:show";
pub const MAIN_NAVIGATE: &str = "main:navigate";
pub const APP_FLUSH: &str = "app:flush";

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CapturesChanged {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added: Option<Capture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<Capture>,
    pub removed: Vec<String>,
    /// La lista cambió de forma masiva: volver a cargarla completa.
    pub reset: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectsInserted {
    pub board_id: String,
    pub objects: Vec<BoardObject>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToastPayload {
    /// "capture" | "info" | "error"
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture: Option<Capture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub fn captures_changed(app: &AppHandle, payload: CapturesChanged) {
    if let Err(e) = app.emit(CAPTURES_CHANGED, payload) {
        log::warn!("no se pudo emitir {CAPTURES_CHANGED}: {e}");
    }
}

pub fn boards_changed(app: &AppHandle) {
    let _ = app.emit(BOARDS_CHANGED, ());
}
