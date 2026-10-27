//! Ingesta de una imagen nueva: decodificar → deduplicar → guardar original
//! y miniatura → registrar en la base → aplicar retención.
//!
//! Orden pensado para no perder ni corromper datos: primero se escriben los
//! archivos (de forma atómica) y solo después se inserta la fila. Si la
//! inserción falla se borran los archivos recién escritos.

use crate::captures::{NewCapture, Removed};
use crate::db::{new_id, now_ms, Db};
use crate::error::{CoreError, Result};
use crate::imaging::{self, ImagePayload};
use crate::models::Capture;
use crate::storage::{write_atomic, Storage};

#[derive(Debug)]
pub struct IngestResult {
    pub capture: Capture,
    /// true si la imagen ya existía (no se creó nada nuevo).
    pub duplicate: bool,
    /// Capturas eliminadas por la política de retención.
    pub pruned: Vec<String>,
}

pub fn ingest(
    db: &Db,
    storage: &Storage,
    payload: ImagePayload,
    source: &str,
    history_limit: u32,
) -> Result<IngestResult> {
    let decoded = imaging::decode(payload)?;
    let hash = imaging::content_hash(&decoded.rgba);

    if let Some(existing) = db.find_capture_by_hash(&hash)? {
        return Ok(IngestResult { capture: existing, duplicate: true, pruned: vec![] });
    }

    let id = new_id();
    let file_name = format!("{id}.{}", imaging::ext_for_mime(decoded.mime));
    let thumb = imaging::make_thumbnail(&decoded.rgba, decoded.opaque)?;
    let thumb_name = format!("{id}.{}", thumb.ext);

    let original_path = storage.original_path(&file_name)?;
    let thumb_path = storage.thumb_path(&thumb_name)?;
    write_atomic(&original_path, &decoded.original)?;
    if let Err(e) = write_atomic(&thumb_path, &thumb.bytes) {
        Storage::remove_quiet(&original_path);
        return Err(e);
    }

    let row = NewCapture {
        id: &id,
        hash: &hash,
        file_name: &file_name,
        thumb_name: &thumb_name,
        mime: decoded.mime,
        width: decoded.rgba.width(),
        height: decoded.rgba.height(),
        thumb_width: thumb.width,
        thumb_height: thumb.height,
        byte_size: decoded.original.len() as u64,
        source,
        created_at: now_ms(),
    };
    if let Err(e) = db.insert_capture(&row) {
        Storage::remove_quiet(&original_path);
        Storage::remove_quiet(&thumb_path);
        // Carrera con otra ingesta del mismo contenido: no es un error real.
        if let CoreError::Db(rusqlite::Error::SqliteFailure(f, _)) = &e {
            if f.code == rusqlite::ErrorCode::ConstraintViolation {
                if let Some(existing) = db.find_capture_by_hash(&hash)? {
                    return Ok(IngestResult { capture: existing, duplicate: true, pruned: vec![] });
                }
            }
        }
        return Err(e);
    }

    let pruned = apply_retention(db, storage, history_limit)?;
    let capture = db.get_capture(&id)?;
    Ok(IngestResult { capture, duplicate: false, pruned })
}

/// Aplica el límite y borra del disco los archivos de las filas eliminadas.
pub fn apply_retention(db: &Db, storage: &Storage, limit: u32) -> Result<Vec<String>> {
    let removed = db.enforce_retention(limit)?;
    Ok(remove_files(storage, removed))
}

/// Borra archivos de filas ya eliminadas (después del commit).
pub fn remove_files(storage: &Storage, removed: Removed) -> Vec<String> {
    for (file, thumb) in &removed.files {
        storage.remove_capture_files(file, thumb);
    }
    removed.ids
}
