//! CanvasClip — punto de entrada de la aplicación de escritorio.

// En Linux/macOS (desarrollo y pruebas) la integración Win32 queda inactiva.
#![cfg_attr(not(windows), allow(dead_code))]

mod clipboard;
mod commands;
mod engine;
mod events;
mod location;
mod protocol;
mod shortcuts;
mod state;
mod tray;
mod windows;

use std::sync::Arc;
use std::time::Duration;

use canvasclip_core::{maintenance, Db, Storage};
use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::state::AppState;
use crate::windows::{panel, MAIN};

/// Argumento con el que arranca al iniciar Windows.
const MINIMIZED_ARG: &str = "--minimized";

pub fn run() {
    tauri::Builder::default()
        // Debe ser el primer plugin: una segunda instancia solo trae al frente la primera.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_main(app, None);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(2_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![MINIMIZED_ARG])))
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, protocol::handle)
        .setup(|app| {
            if let Err(e) = setup(app.handle()) {
                log::error!("error al iniciar: {e}");
                app.dialog()
                    .message(format!("CanvasClip no pudo iniciar.\n\n{e}"))
                    .kind(MessageDialogKind::Error)
                    .title("CanvasClip")
                    .blocking_show();
                return Err(e.into());
            }
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // Cerrar la ventana principal la oculta: la app sigue en la bandeja.
            (MAIN, WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            (panel::LABEL, WindowEvent::Focused(focused)) => {
                panel::on_focus_changed(window.app_handle(), *focused);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::captures::list_captures,
            commands::captures::get_capture,
            commands::captures::get_captures,
            commands::captures::set_favorite,
            commands::captures::delete_capture,
            commands::captures::copy_capture,
            commands::captures::paste_clipboard,
            commands::captures::import_image_bytes,
            commands::captures::clear_history,
            commands::captures::storage_info,
            commands::captures::copy_text,
            commands::boards::list_boards,
            commands::boards::create_board,
            commands::boards::rename_board,
            commands::boards::duplicate_board,
            commands::boards::delete_board,
            commands::boards::load_board,
            commands::boards::save_board_changes,
            commands::boards::save_camera,
            commands::boards::get_active_board,
            commands::boards::set_active_board,
            commands::boards::add_capture_to_board,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::app_info,
            commands::settings::change_data_dir,
            commands::settings::open_data_dir,
            commands::windows::panel_hide,
            commands::windows::panel_open,
            commands::windows::panel_close,
            commands::windows::show_main,
            commands::windows::toast_clicked,
            commands::windows::toast_hide,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar CanvasClip");
}

fn setup(app: &AppHandle) -> Result<(), String> {
    // 1. Datos: carpeta, base de datos y verificación de integridad.
    let dir = location::data_dir(app).map_err(|e| e.to_string())?;
    let storage = Arc::new(Storage::new(&dir).map_err(|e| format!("No se pudo usar la carpeta {}: {e}", dir.display()))?);
    let db = Arc::new(Db::open(&storage.db_path()).map_err(|e| format!("No se pudo abrir la base de datos: {e}"))?);
    match db.quick_check() {
        Ok(true) => {}
        Ok(false) => log::error!("la verificación de integridad de la base de datos falló"),
        Err(e) => log::error!("no se pudo verificar la base de datos: {e}"),
    }
    log::info!("datos en {}", dir.display());

    let settings = db.load_settings().map_err(|e| e.to_string())?;
    let first_run = db.take_first_run().map_err(|e| e.to_string())?;
    let default_board = db.ensure_default_board().map_err(|e| e.to_string())?;
    let active_board = db
        .last_board_id()
        .ok()
        .flatten()
        .filter(|id| db.board_exists(id).unwrap_or(false))
        .unwrap_or(default_board);

    // 2. Estado global y motor de capturas.
    let engine = engine::start(app.clone(), db.clone(), storage.clone());
    app.manage(AppState {
        db: db.clone(),
        storage: storage.clone(),
        settings: RwLock::new(settings.clone()),
        active_board: RwLock::new(active_board),
        engine,
        panel: Default::default(),
        shortcut: Mutex::new(None),
        tray_monitor_item: Mutex::new(None),
    });

    // Las ventanas se crean recién ahora (en la configuración tienen
    // "create": false): así su interfaz nunca invoca comandos antes de que
    // el estado exista.
    for config in app.config().app.windows.clone() {
        tauri::WebviewWindowBuilder::from_config(app, &config)
            .and_then(|b| b.build())
            .map_err(|e| format!("No se pudo crear la ventana {}: {e}", config.label))?;
    }

    // 3. Reparación de datos tras un cierre inesperado (en segundo plano).
    {
        let (db, storage) = (db.clone(), storage.clone());
        std::thread::spawn(move || match maintenance::reconcile(&db, &storage, Duration::from_secs(60)) {
            Ok(r) => log::info!("reconciliación: {r:?}"),
            Err(e) => log::error!("reconciliación falló: {e}"),
        });
    }

    // 4. Integración con el sistema.
    windows::prepare(app, &settings.effects);
    if let Err(e) = tray::build(app, settings.monitoring) {
        log::error!("no se pudo crear el ícono de la bandeja: {e}");
    }
    if let Err(e) = shortcuts::register(app, &settings.shortcut) {
        log::warn!("{e}");
    }
    if first_run && settings.launch_at_startup {
        commands::settings::sync_autostart(app, true);
    }
    clipboard::start_listener(app.clone());
    windows::edge::start(app.clone());

    // 5. Ventana principal: oculta si arrancó con Windows y así se configuró.
    let autostarted = std::env::args().any(|a| a == MINIMIZED_ARG);
    if !(autostarted && settings.start_minimized) {
        if let Some(win) = app.get_webview_window(MAIN) {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
    Ok(())
}
