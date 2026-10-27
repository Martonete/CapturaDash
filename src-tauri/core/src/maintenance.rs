//! Reconciliación al arrancar y traslado de la carpeta de datos.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::db::Db;
use crate::error::{CoreError, Result};
use crate::imaging;
use crate::storage::{self, is_tmp_file, write_atomic, Storage, DB_FILE};

#[derive(Debug, Default, PartialEq)]
pub struct ReconcileReport {
    pub tmp_removed: u32,
    pub orphans_removed: u32,
    pub missing_rows_removed: u32,
    pub missing_referenced: u32,
    pub thumbs_rebuilt: u32,
}

/// Deja disco y base consistentes después de un cierre inesperado:
/// * borra `.tmp` de escrituras interrumpidas;
/// * borra archivos que ninguna fila referencia (con más de `grace` de antigüedad);
/// * elimina filas sin original que nadie usa; las usadas se conservan
///   (el pizarrón muestra un marcador) para no perder la disposición;
/// * regenera miniaturas faltantes desde el original.
pub fn reconcile(db: &Db, storage: &Storage, grace: Duration) -> Result<ReconcileReport> {
    let mut report = ReconcileReport::default();
    let rows = db.all_capture_files()?;
    let mut known_originals = HashSet::new();
    let mut known_thumbs = HashSet::new();

    for (id, files, protected) in &rows {
        known_originals.insert(files.file_name.clone());
        known_thumbs.insert(files.thumb_name.clone());
        let original = storage.original_path(&files.file_name)?;
        if !original.exists() {
            if *protected {
                report.missing_referenced += 1;
                log::warn!("original faltante para captura en uso {id}");
            } else if db.delete_capture(id, false).is_ok() {
                Storage::remove_quiet(&storage.thumb_path(&files.thumb_name)?);
                report.missing_rows_removed += 1;
            }
            continue;
        }
        let thumb = storage.thumb_path(&files.thumb_name)?;
        if !thumb.exists() {
            match rebuild_thumb(db, storage, id, &original) {
                Ok(name) => {
                    known_thumbs.insert(name);
                    report.thumbs_rebuilt += 1;
                }
                Err(e) => log::warn!("no se pudo regenerar la miniatura de {id}: {e}"),
            }
        }
    }

    let now = SystemTime::now();
    for (dir, known) in [(storage.originals_dir(), &known_originals), (storage.thumbs_dir(), &known_thumbs)] {
        for entry in fs::read_dir(&dir)?.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if !meta.is_file() {
                continue;
            }
            let old_enough = meta
                .modified()
                .ok()
                .and_then(|m| now.duration_since(m).ok())
                .is_none_or(|age| age >= grace);
            if is_tmp_file(&path) {
                if old_enough {
                    Storage::remove_quiet(&path);
                    report.tmp_removed += 1;
                }
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !known.contains(&name) && old_enough {
                Storage::remove_quiet(&path);
                report.orphans_removed += 1;
            }
        }
    }
    Ok(report)
}

fn rebuild_thumb(db: &Db, storage: &Storage, id: &str, original: &Path) -> Result<String> {
    let rgba = imaging::load_rgba(&fs::read(original)?)?;
    let opaque = rgba.pixels().all(|p| p.0[3] == 255);
    let thumb = imaging::make_thumbnail(&rgba, opaque)?;
    let name = format!("{id}.{}", thumb.ext);
    write_atomic(&storage.thumb_path(&name)?, &thumb.bytes)?;
    db.update_thumb(id, &name, thumb.width, thumb.height)?;
    Ok(name)
}

/// Copia todos los datos a `dest` (base consistente vía VACUUM INTO + archivos).
/// No borra el origen: el usuario decide cuándo eliminarlo.
pub fn copy_data_dir(db: &Db, from: &Storage, dest: &Path) -> Result<Storage> {
    if dest.join(DB_FILE).exists() {
        return Err(CoreError::Invalid(
            "La carpeta de destino ya contiene datos de CanvasClip".into(),
        ));
    }
    if dest.starts_with(from.root()) || from.root().starts_with(dest) {
        return Err(CoreError::Invalid(
            "La carpeta de destino no puede estar dentro de la actual (ni al revés)".into(),
        ));
    }
    let to = Storage::new(dest)?;
    let originals = storage::copy_flat_dir(&from.originals_dir(), &to.originals_dir())?;
    storage::copy_flat_dir(&from.thumbs_dir(), &to.thumbs_dir())?;
    db.vacuum_into(&to.db_path())?;

    // Verificación: la copia abre, pasa el chequeo y tiene las mismas filas.
    let copy = Db::open(&to.db_path())?;
    let (src_count, _) = db.capture_counts()?;
    let (dst_count, _) = copy.capture_counts()?;
    if !copy.quick_check()? || src_count != dst_count || (originals as u32) < dst_count {
        return Err(CoreError::Invalid("La copia de datos no pasó la verificación".into()));
    }
    Ok(to)
}
