use std::sync::Arc;

use canvasclip_core::settings::Settings;
use canvasclip_core::{Db, Storage};
use parking_lot::{Mutex, RwLock};
use tauri::menu::CheckMenuItem;
use tauri::Wry;

use crate::windows::panel::PanelState;

/// Estado global compartido por comandos, hilos nativos y eventos.
pub struct AppState {
    pub db: Arc<Db>,
    pub storage: Arc<Storage>,
    pub settings: RwLock<Settings>,
    pub active_board: RwLock<String>,
    pub engine: crate::engine::EngineHandle,
    pub panel: PanelState,
    /// Atajo global registrado actualmente (para poder reemplazarlo).
    pub shortcut: Mutex<Option<String>>,
    /// Ítem "Captura automática" del menú de la bandeja, para reflejar cambios.
    pub tray_monitor_item: Mutex<Option<CheckMenuItem<Wry>>>,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }
}
