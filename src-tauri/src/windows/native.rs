//! Pequeños envoltorios Win32 usados por el panel, el aviso y el borde.

#![cfg(windows)]

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows_sys::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_NOT_PRESENT, QUNS_PRESENTATION_MODE,
    QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, ShowWindow, SW_SHOWNOACTIVATE};

pub fn raw_hwnd<R: tauri::Runtime>(w: &tauri::WebviewWindow<R>) -> Option<HWND> {
    w.hwnd().ok().map(|h| h.0 as HWND)
}

/// Muestra la ventana sin quitarle el foco a la aplicación que se está usando.
pub fn show_no_activate(hwnd: HWND) {
    unsafe {
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

/// Esquinas redondeadas nativas en Windows 11 (en Windows 10 no hace nada).
pub fn round_corners(hwnd: HWND) {
    let pref = DWMWCP_ROUND;
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &pref as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&pref) as u32,
        );
    }
}

pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut p = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
}

pub fn mouse_button_down() -> bool {
    unsafe {
        (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_RBUTTON as i32) as u16 & 0x8000) != 0
    }
}

/// true si hay un juego/video/presentación en pantalla completa o la sesión
/// está bloqueada: en esos casos el borde no debe abrir el panel.
pub fn user_busy() -> bool {
    let mut state = 0;
    let hr = unsafe { SHQueryUserNotificationState(&mut state) };
    hr >= 0
        && matches!(
            state,
            QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE | QUNS_NOT_PRESENT
        )
}

/// Rectángulo del monitor que contiene el punto.
pub fn monitor_rect(x: i32, y: i32) -> Option<RECT> {
    unsafe {
        let mon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        if mon.is_null() {
            return None;
        }
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        (GetMonitorInfoW(mon, &mut info) != 0).then_some(info.rcMonitor)
    }
}

/// ¿Hay otro monitor inmediatamente a la derecha de este punto?
pub fn monitor_at(x: i32, y: i32) -> bool {
    unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONULL) != null_mut() }
}
