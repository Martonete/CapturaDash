//! Disposición de archivos en disco y escritura atómica.
//!
//! ```text
//! <data_dir>/
//!   canvasclip.db        base SQLite (+ -wal / -shm)
//!   originals/<id>.png   archivos originales, nunca se modifican
//!   thumbs/<id>.jpg      miniaturas (jpg si es opaca, png si tiene alfa)
//! ```

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

pub const DB_FILE: &str = "canvasclip.db";
const ORIGINALS: &str = "originals";
const THUMBS: &str = "thumbs";
const TMP_SUFFIX: &str = ".tmp";

#[derive(Debug, Clone)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// Crea (si hace falta) la estructura de carpetas.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join(ORIGINALS))?;
        fs::create_dir_all(root.join(THUMBS))?;
        Ok(Storage { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db_path(&self) -> PathBuf {
        self.root.join(DB_FILE)
    }

    pub fn originals_dir(&self) -> PathBuf {
        self.root.join(ORIGINALS)
    }

    pub fn thumbs_dir(&self) -> PathBuf {
        self.root.join(THUMBS)
    }

    pub fn original_path(&self, name: &str) -> Result<PathBuf> {
        Ok(self.originals_dir().join(safe_name(name)?))
    }

    pub fn thumb_path(&self, name: &str) -> Result<PathBuf> {
        Ok(self.thumbs_dir().join(safe_name(name)?))
    }

    /// Borra un archivo ignorando "no existe".
    pub fn remove_quiet(path: &Path) {
        if let Err(e) = fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("no se pudo borrar {}: {e}", path.display());
            }
        }
    }

    pub fn remove_capture_files(&self, file_name: &str, thumb_name: &str) {
        if let Ok(p) = self.original_path(file_name) {
            Self::remove_quiet(&p);
        }
        if let Ok(p) = self.thumb_path(thumb_name) {
            Self::remove_quiet(&p);
        }
    }

    /// Tamaño total de originales + miniaturas, en bytes.
    pub fn used_bytes(&self) -> u64 {
        [self.originals_dir(), self.thumbs_dir()]
            .iter()
            .filter_map(|d| fs::read_dir(d).ok())
            .flatten()
            .filter_map(|e| e.ok()?.metadata().ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum()
    }
}

/// Evita que un nombre guardado en la base escape de su carpeta.
fn safe_name(name: &str) -> Result<&str> {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !name.starts_with('.');
    if ok {
        Ok(name)
    } else {
        Err(CoreError::Invalid(format!("nombre de archivo inválido: {name}")))
    }
}

/// Escribe en `<path>.tmp`, sincroniza a disco y renombra. Un cierre a mitad
/// de camino deja, como mucho, un `.tmp` huérfano que se limpia al arrancar.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = tmp_path(path);
    let result = (|| -> Result<()> {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        Storage::remove_quiet(&tmp);
    }
    result
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(TMP_SUFFIX);
    PathBuf::from(s)
}

pub fn is_tmp_file(path: &Path) -> bool {
    path.to_string_lossy().ends_with(TMP_SUFFIX)
}

/// Copia recursiva de una carpeta plana de archivos (originals/ o thumbs/).
pub fn copy_flat_dir(from: &Path, to: &Path) -> Result<u64> {
    fs::create_dir_all(to)?;
    let mut count = 0;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_file() && !is_tmp_file(&path) {
            fs::copy(&path, to.join(entry.file_name()))?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let s = Storage::new(dir.path()).unwrap();
        assert!(s.original_path("../x.png").is_err());
        assert!(s.original_path("a/b.png").is_err());
        assert!(s.original_path("..\\x.png").is_err());
        assert!(s.original_path("0192-ab.png").is_ok());
    }

    #[test]
    fn atomic_write_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.bin");
        write_atomic(&p, b"hola").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"hola");
        assert!(!tmp_path(&p).exists());
    }
}
