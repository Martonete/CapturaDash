//! Comandos invocables desde las ventanas. Todos son `async` para no ocupar
//! el hilo principal; el trabajo pesado (decodificar imágenes, portapapeles)
//! va a `spawn_blocking`.

pub mod boards;
pub mod captures;
pub mod settings;
pub mod windows;

pub type CmdResult<T> = Result<T, String>;

pub(crate) fn core_err(e: canvasclip_core::CoreError) -> String {
    e.user_message()
}

pub(crate) fn join_err(e: tauri::Error) -> String {
    format!("Error interno: {e}")
}
