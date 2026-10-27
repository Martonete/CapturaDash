//! Protocolo `ccimg` para servir imágenes a las ventanas sin exponer rutas:
//!   `ccimg://localhost/thumb/<id>` y `ccimg://localhost/orig/<id>`
//! (en Windows: `http://ccimg.localhost/...`).
//! Los ids nunca se reutilizan, así que las respuestas se cachean indefinidamente.

use std::fs;

use tauri::http::{header, Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::state::AppState;

pub const SCHEME: &str = "ccimg";

pub fn handle<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = ctx.app_handle().clone();
    let path = request.uri().path().trim_start_matches('/').to_string();
    // Lectura de disco fuera del hilo del webview.
    tauri::async_runtime::spawn_blocking(move || {
        let response = serve(&app.state::<AppState>(), &path).unwrap_or_else(|status| {
            Response::builder().status(status).body(Vec::new()).expect("respuesta vacía")
        });
        responder.respond(response);
    });
}

fn serve(state: &AppState, path: &str) -> Result<Response<Vec<u8>>, StatusCode> {
    let (kind, id) = path.split_once('/').ok_or(StatusCode::BAD_REQUEST)?;
    let files = state.db.capture_files(id).map_err(|_| StatusCode::NOT_FOUND)?;
    let (file, mime) = match kind {
        "thumb" => {
            let mime = if files.thumb_name.ends_with(".jpg") { "image/jpeg" } else { "image/png" };
            (state.storage.thumb_path(&files.thumb_name), mime.to_string())
        }
        "orig" => (state.storage.original_path(&files.file_name), files.mime.clone()),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let bytes = fs::read(file.map_err(|_| StatusCode::BAD_REQUEST)?).map_err(|_| StatusCode::NOT_FOUND)?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
