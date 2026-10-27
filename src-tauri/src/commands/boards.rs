use canvasclip_core::models::{Board, BoardChanges, BoardDocument, BoardSummary, SaveReport};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::{core_err, CmdResult};
use crate::events::{self, ObjectsInserted, BOARD_OBJECTS_INSERTED};
use crate::state::AppState;
use crate::windows::MAIN;

/// Tamaño máximo con el que una captura entra al pizarrón (unidades del mundo).
const MAX_INITIAL_SIDE: f64 = 900.0;

#[tauri::command]
pub async fn list_boards(state: State<'_, AppState>) -> CmdResult<Vec<BoardSummary>> {
    state.db.list_boards().map_err(core_err)
}

#[tauri::command]
pub async fn create_board(app: AppHandle, state: State<'_, AppState>, name: String) -> CmdResult<Board> {
    let b = state.db.create_board(&name).map_err(core_err)?;
    events::boards_changed(&app);
    Ok(b)
}

#[tauri::command]
pub async fn rename_board(app: AppHandle, state: State<'_, AppState>, id: String, name: String) -> CmdResult<Board> {
    let b = state.db.rename_board(&id, &name).map_err(core_err)?;
    events::boards_changed(&app);
    Ok(b)
}

#[tauri::command]
pub async fn duplicate_board(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<Board> {
    let b = state.db.duplicate_board(&id).map_err(core_err)?;
    events::boards_changed(&app);
    Ok(b)
}

/// Devuelve el pizarrón activo después del borrado.
#[tauri::command]
pub async fn delete_board(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<String> {
    let fallback = state.db.delete_board(&id).map_err(core_err)?;
    let active = {
        let mut active = state.active_board.write();
        if *active == id {
            *active = fallback.clone();
        }
        active.clone()
    };
    state.db.set_last_board_id(&active).map_err(core_err)?;
    events::boards_changed(&app);
    Ok(active)
}

#[tauri::command]
pub async fn load_board(state: State<'_, AppState>, id: String) -> CmdResult<BoardDocument> {
    state.db.load_board(&id).map_err(core_err)
}

#[tauri::command]
pub async fn save_board_changes(state: State<'_, AppState>, changes: BoardChanges) -> CmdResult<SaveReport> {
    let report = state.db.save_board_changes(&changes).map_err(core_err)?;
    if !report.skipped_objects.is_empty() {
        log::warn!("objetos descartados al guardar: {:?}", report.skipped_objects);
    }
    Ok(report)
}

#[tauri::command]
pub async fn save_camera(state: State<'_, AppState>, id: String, x: f64, y: f64, zoom: f64) -> CmdResult<()> {
    state.db.save_camera(&id, x, y, zoom).map_err(core_err)
}

#[tauri::command]
pub async fn get_active_board(state: State<'_, AppState>) -> CmdResult<String> {
    Ok(state.active_board.read().clone())
}

#[tauri::command]
pub async fn set_active_board(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    if !state.db.board_exists(&id).map_err(core_err)? {
        return Err("El pizarrón no existe".into());
    }
    *state.active_board.write() = id.clone();
    state.db.set_last_board_id(&id).map_err(core_err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedToBoard {
    pub board_id: String,
    pub board_name: String,
}

/// Agrega una captura a un pizarrón (el activo si no se indica). Se guarda en
/// la base de inmediato —no depende de que la ventana principal esté abierta—
/// y se avisa a la ventana principal para que la muestre y la ubique en vista.
#[tauri::command]
pub async fn add_capture_to_board(
    app: AppHandle,
    state: State<'_, AppState>,
    capture_id: String,
    board_id: Option<String>,
) -> CmdResult<AddedToBoard> {
    let board_id = board_id.unwrap_or_else(|| state.active_board.read().clone());
    let board = state.db.get_board(&board_id).map_err(core_err)?;
    let capture = state.db.get_capture(&capture_id).map_err(core_err)?;

    // Tamaño "1:1" en pantalla: píxeles físicos / escala de la pantalla.
    let scale = app
        .get_webview_window(MAIN)
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0)
        .max(1.0);
    let (mut w, mut h) = (capture.width as f64 / scale, capture.height as f64 / scale);
    let shrink = (MAX_INITIAL_SIDE / w.max(h)).min(1.0);
    w *= shrink;
    h *= shrink;

    let obj = state.db.insert_capture_object(&board_id, &capture_id, w, h).map_err(core_err)?;
    let _ = app.emit_to(MAIN, BOARD_OBJECTS_INSERTED, ObjectsInserted { board_id: board_id.clone(), objects: vec![obj] });
    let updated = state.db.get_capture(&capture_id).map_err(core_err)?;
    events::captures_changed(&app, events::CapturesChanged { updated: Some(updated), ..Default::default() });
    Ok(AddedToBoard { board_id, board_name: board.name })
}
