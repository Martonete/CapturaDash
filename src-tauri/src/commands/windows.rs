use tauri::AppHandle;

use super::CmdResult;
use crate::windows::{self, panel, toast};

/// La interfaz del panel terminó la animación de cierre.
#[tauri::command]
pub async fn panel_hide(app: AppHandle) -> CmdResult<()> {
    panel::finish_close(&app);
    Ok(())
}

#[tauri::command]
pub async fn panel_open(app: AppHandle) -> CmdResult<()> {
    panel::open(&app, true);
    Ok(())
}

#[tauri::command]
pub async fn panel_close(app: AppHandle) -> CmdResult<()> {
    panel::request_close(&app);
    Ok(())
}

#[tauri::command]
pub async fn show_main(app: AppHandle, view: Option<String>) -> CmdResult<()> {
    panel::request_close(&app);
    windows::show_main(&app, view.as_deref());
    Ok(())
}

#[tauri::command]
pub async fn toast_clicked(app: AppHandle) -> CmdResult<()> {
    toast::hide(&app);
    panel::open(&app, true);
    Ok(())
}

#[tauri::command]
pub async fn toast_hide(app: AppHandle) -> CmdResult<()> {
    toast::hide(&app);
    Ok(())
}
