//! Motor de capturas: un hilo dedicado procesa las imágenes detectadas en el
//! portapapeles (decodificar, deduplicar, guardar, miniatura, retención) para
//! que ni el hilo del portapapeles ni la interfaz se bloqueen.

use std::sync::Arc;
use std::thread;

use canvasclip_core::imaging::ImagePayload;
use canvasclip_core::ingest::{self, IngestResult};
use canvasclip_core::{CoreError, Db, Storage};
use crossbeam_channel::{bounded, Sender, TrySendError};
use tauri::{AppHandle, Manager};

use crate::events::{self, CapturesChanged};
use crate::state::AppState;
use crate::windows::toast;

#[derive(Clone)]
pub struct EngineHandle {
    tx: Sender<ImagePayload>,
}

impl EngineHandle {
    /// Encola una imagen. Si hay una ráfaga muy grande se descartan las más
    /// nuevas en vez de acumular cientos de MB en memoria.
    pub fn submit(&self, payload: ImagePayload) {
        match self.tx.try_send(payload) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => log::warn!("cola de capturas llena; imagen descartada"),
            Err(TrySendError::Disconnected(_)) => log::error!("motor de capturas detenido"),
        }
    }
}

pub fn start(app: AppHandle, db: Arc<Db>, storage: Arc<Storage>) -> EngineHandle {
    let (tx, rx) = bounded::<ImagePayload>(16);
    thread::Builder::new()
        .name("canvasclip-ingest".into())
        .spawn(move || {
            for payload in rx {
                let limit = app.state::<AppState>().settings.read().history_limit;
                let result = ingest::ingest(&db, &storage, payload, "clipboard", limit);
                handle_result(&app, result);
            }
        })
        .expect("hilo de ingesta");
    EngineHandle { tx }
}

/// Publica el resultado de una ingesta (también usado por "pegar" en el pizarrón).
pub fn handle_result(app: &AppHandle, result: Result<IngestResult, CoreError>) {
    match result {
        Ok(r) if r.duplicate => log::debug!("captura duplicada ignorada: {}", r.capture.id),
        Ok(r) => {
            log::info!("captura guardada {} ({}x{})", r.capture.id, r.capture.width, r.capture.height);
            let notify = app.state::<AppState>().settings.read().notifications;
            if notify {
                toast::show_capture(app, r.capture.clone());
            }
            events::captures_changed(
                app,
                CapturesChanged { added: Some(r.capture), removed: r.pruned, ..Default::default() },
            );
        }
        Err(e) => {
            log::error!("no se pudo guardar la captura: {e}");
            toast::show_message(app, "error", format!("No se pudo guardar la captura. {}", e.user_message()));
        }
    }
}
