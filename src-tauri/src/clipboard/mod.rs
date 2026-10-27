//! Integración con el portapapeles del sistema.
//!
//! En Windows se usa la API nativa: una ventana de solo mensajes registrada
//! con `AddClipboardFormatListener` recibe `WM_CLIPBOARDUPDATE` cada vez que
//! cambia el contenido. No hay sondeo periódico.

use canvasclip_core::imaging::ImagePayload;
use tauri::{AppHandle, Manager};

use crate::state::AppState;

#[cfg(windows)]
mod win;

pub enum ClipContent {
    Image(ImagePayload),
    Text(String),
    Empty,
}

/// Inicia el monitoreo. Las imágenes detectadas se envían al motor de ingesta
/// solo si el monitoreo está activo en la configuración.
pub fn start_listener(app: AppHandle) {
    #[cfg(windows)]
    {
        let on_image = move |payload: ImagePayload| {
            let state = app.state::<AppState>();
            if state.settings.read().monitoring {
                state.engine.submit(payload);
            }
        };
        win::start_listener(Box::new(on_image));
    }
    #[cfg(not(windows))]
    {
        let _ = app.state::<AppState>();
        log::warn!("monitoreo del portapapeles disponible solo en Windows");
    }
}

/// Copia una imagen: formato "PNG" (sin pérdida, con alfa) + CF_DIB (compatibilidad).
/// La escritura queda marcada para que el monitor no la vuelva a registrar.
pub fn write_image(png: &[u8], dib: &[u8]) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::write_image(png, dib)
    }
    #[cfg(not(windows))]
    {
        let _ = (png, dib);
        Err("Copiar imágenes solo está disponible en Windows".into())
    }
}

pub fn write_text(text: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        win::write_text(text)
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        Err("Copiar texto solo está disponible en Windows".into())
    }
}

pub fn read() -> Result<ClipContent, String> {
    #[cfg(windows)]
    {
        win::read()
    }
    #[cfg(not(windows))]
    {
        Ok(ClipContent::Empty)
    }
}
