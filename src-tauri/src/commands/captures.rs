use std::fs;

use canvasclip_core::captures::Filter;
use canvasclip_core::imaging::{self, ImagePayload};
use canvasclip_core::ingest;
use canvasclip_core::models::{Capture, StorageInfo};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::{core_err, join_err, CmdResult};
use crate::clipboard::{self, ClipContent};
use crate::events::{self, CapturesChanged};
use crate::state::AppState;

#[tauri::command]
pub async fn list_captures(
    state: State<'_, AppState>,
    favorites: bool,
    limit: u32,
    offset: u32,
) -> CmdResult<Vec<Capture>> {
    let filter = if favorites { Filter::Favorites } else { Filter::All };
    state.db.list_captures(filter, limit.min(1000), offset).map_err(core_err)
}

#[tauri::command]
pub async fn get_capture(state: State<'_, AppState>, id: String) -> CmdResult<Capture> {
    state.db.get_capture(&id).map_err(core_err)
}

/// Varias capturas a la vez (las que usa un pizarrón); ignora ids inexistentes.
#[tauri::command]
pub async fn get_captures(state: State<'_, AppState>, ids: Vec<String>) -> CmdResult<Vec<Capture>> {
    Ok(ids.iter().filter_map(|id| state.db.get_capture(id).ok()).collect())
}

#[tauri::command]
pub async fn set_favorite(app: AppHandle, state: State<'_, AppState>, id: String, favorite: bool) -> CmdResult<Capture> {
    let c = state.db.set_favorite(&id, favorite).map_err(core_err)?;
    events::captures_changed(&app, CapturesChanged { updated: Some(c.clone()), ..Default::default() });
    Ok(c)
}

/// Borra una captura del historial (y de disco). Si se usa en pizarrones,
/// requiere `force` (la interfaz pide confirmación explícita).
#[tauri::command]
pub async fn delete_capture(app: AppHandle, state: State<'_, AppState>, id: String, force: bool) -> CmdResult<()> {
    let (removed, boards) = state.db.delete_capture(&id, force).map_err(core_err)?;
    let ids = ingest::remove_files(&state.storage, removed);
    events::captures_changed(&app, CapturesChanged { removed: ids, ..Default::default() });
    if !boards.is_empty() {
        events::boards_changed(&app);
    }
    Ok(())
}

/// Vuelve a copiar una captura al portapapeles (PNG + DIB). El monitor la
/// reconoce como escritura propia y no crea un duplicado.
#[tauri::command]
pub async fn copy_capture(app: AppHandle, id: String) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let files = state.db.capture_files(&id).map_err(core_err)?;
        let path = state.storage.original_path(&files.file_name).map_err(core_err)?;
        let bytes = fs::read(path).map_err(|_| "No se encontró el archivo original".to_string())?;
        let rgba = imaging::load_rgba(&bytes).map_err(core_err)?;
        let png = if files.mime == "image/png" {
            bytes
        } else {
            imaging::encode_png(&rgba, true).map_err(core_err)?
        };
        clipboard::write_image(&png, &imaging::rgba_to_dib(&rgba))
    })
    .await
    .map_err(join_err)?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PasteResult {
    Image { capture: Capture },
    Text { text: String },
    Empty,
}

/// Ctrl+V en el pizarrón: incorpora la imagen del portapapeles al historial
/// (o devuelve la existente si ya estaba) o el texto para crear una nota.
#[tauri::command]
pub async fn paste_clipboard(app: AppHandle) -> CmdResult<PasteResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        match clipboard::read()? {
            ClipContent::Image(payload) => {
                let limit = state.settings.read().history_limit;
                let r = ingest::ingest(&state.db, &state.storage, payload, "paste", limit).map_err(core_err)?;
                if !r.duplicate {
                    events::captures_changed(
                        &app,
                        CapturesChanged { added: Some(r.capture.clone()), removed: r.pruned, ..Default::default() },
                    );
                }
                Ok(PasteResult::Image { capture: r.capture })
            }
            ClipContent::Text(text) if !text.trim().is_empty() => Ok(PasteResult::Text { text }),
            _ => Ok(PasteResult::Empty),
        }
    })
    .await
    .map_err(join_err)?
}

/// Importa archivos de imagen soltados en el pizarrón (PNG/JPEG).
#[tauri::command]
pub async fn import_image_bytes(app: AppHandle, request: tauri::ipc::Request<'_>) -> CmdResult<Capture> {
    // El archivo llega como cuerpo binario (sin serializar a JSON).
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("Se esperaba el contenido binario de la imagen".into());
    };
    let bytes = bytes.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let limit = state.settings.read().history_limit;
        let r = ingest::ingest(&state.db, &state.storage, ImagePayload::Encoded(bytes), "paste", limit)
            .map_err(core_err)?;
        if !r.duplicate {
            events::captures_changed(
                &app,
                CapturesChanged { added: Some(r.capture.clone()), removed: r.pruned, ..Default::default() },
            );
        }
        Ok(r.capture)
    })
    .await
    .map_err(join_err)?
}

/// Limpieza manual: borra todas las capturas no protegidas.
#[tauri::command]
pub async fn clear_history(app: AppHandle, state: State<'_, AppState>) -> CmdResult<u32> {
    let removed = state.db.clear_unprotected().map_err(core_err)?;
    let ids = ingest::remove_files(&state.storage, removed);
    let n = ids.len() as u32;
    events::captures_changed(&app, CapturesChanged { removed: ids, reset: true, ..Default::default() });
    Ok(n)
}

#[tauri::command]
pub async fn storage_info(state: State<'_, AppState>) -> CmdResult<StorageInfo> {
    let (count, protected) = state.db.capture_counts().map_err(core_err)?;
    Ok(StorageInfo {
        data_dir: state.storage.root().to_string_lossy().to_string(),
        capture_count: count,
        protected_count: protected,
        bytes: state.storage.used_bytes(),
    })
}

#[tauri::command]
pub async fn copy_text(text: String) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || clipboard::write_text(&text)).await.map_err(join_err)?
}
