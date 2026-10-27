use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Error de base de datos: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Error de archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("No se pudo procesar la imagen: {0}")]
    Image(#[from] image::ImageError),
    #[error("{0}")]
    Invalid(String),
    #[error("No encontrado: {0}")]
    NotFound(String),
    #[error("La captura se usa en {0} pizarrón(es)")]
    InUse(u32),
}

impl CoreError {
    /// Distingue "disco lleno" para poder avisarlo con un mensaje claro.
    pub fn is_disk_full(&self) -> bool {
        match self {
            CoreError::Io(e) => {
                e.kind() == std::io::ErrorKind::StorageFull
                    // ERROR_DISK_FULL (112) / ERROR_HANDLE_DISK_FULL (39) en Windows
                    || matches!(e.raw_os_error(), Some(112) | Some(39))
            }
            CoreError::Db(rusqlite::Error::SqliteFailure(f, _)) => {
                f.code == rusqlite::ErrorCode::DiskFull
            }
            _ => false,
        }
    }

    /// Mensaje apto para mostrar al usuario.
    pub fn user_message(&self) -> String {
        if self.is_disk_full() {
            "No hay espacio suficiente en el disco.".to_string()
        } else {
            self.to_string()
        }
    }
}

// Los comandos de Tauri devuelven el error como texto legible.
impl Serialize for CoreError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.user_message())
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
