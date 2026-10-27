//! Implementación Win32 del portapapeles.

use std::cell::RefCell;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use canvasclip_core::imaging::ImagePayload;
use windows_sys::Win32::Foundation::{GlobalFree, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::{CF_DIB, CF_DIBV5, CF_UNICODETEXT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, KillTimer, RegisterClassExW,
    SetTimer, HWND_MESSAGE, MSG, WM_CLIPBOARDUPDATE, WM_TIMER, WNDCLASSEXW,
};

use super::ClipContent;

/// Secuencia del portapapeles generada por nuestra última escritura; el
/// monitor la ignora para no volver a registrar lo que CanvasClip copió.
static OWN_SEQ: AtomicU32 = AtomicU32::new(0);
/// Ventana de solo mensajes del listener (dueña del portapapeles al escribir).
static LISTENER_HWND: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(null_mut());

/// Espera tras WM_CLIPBOARDUPDATE antes de leer: agrupa ráfagas de
/// actualizaciones (algunas apps escriben varios formatos por separado) y le
/// da tiempo al dueño a terminar.
const SETTLE_MS: u32 = 120;
const TIMER_SETTLE: usize = 1;
const TIMER_RETRY: usize = 2;
const RETRY_MS: u32 = 400;
const MAX_RETRIES: u32 = 5;

type OnImage = Box<dyn Fn(ImagePayload) + Send>;

struct Listener {
    on_image: OnImage,
    last_seq: u32,
    retries: u32,
}

thread_local! {
    static LISTENER: RefCell<Option<Listener>> = const { RefCell::new(None) };
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn png_format() -> u32 {
    let name = wide("PNG");
    unsafe { RegisterClipboardFormatW(name.as_ptr()) }
}

/// Formatos que las apps usan para pedir que su contenido no se registre
/// (gestores de contraseñas, etc.).
fn excluded() -> bool {
    ["ExcludeClipboardContentFromMonitorProcessing", "Clipboard Viewer Ignore"]
        .iter()
        .any(|n| {
            let name = wide(n);
            unsafe {
                let f = RegisterClipboardFormatW(name.as_ptr());
                f != 0 && IsClipboardFormatAvailable(f) != 0
            }
        })
}

/// ¿El portapapeles tiene una imagen "pura"? Office y otras apps adjuntan una
/// versión en imagen al copiar texto o celdas; si hay texto, es una copia de
/// texto y no se registra.
unsafe fn is_plain_image() -> bool {
    let has_image = IsClipboardFormatAvailable(png_format()) != 0
        || IsClipboardFormatAvailable(CF_DIBV5 as u32) != 0
        || IsClipboardFormatAvailable(CF_DIB as u32) != 0;
    has_image && IsClipboardFormatAvailable(CF_UNICODETEXT as u32) == 0
}

pub fn start_listener(on_image: OnImage) {
    thread::Builder::new()
        .name("canvasclip-clipboard".into())
        .spawn(move || unsafe { run_listener(on_image) })
        .expect("hilo del portapapeles");
}

unsafe fn run_listener(on_image: OnImage) {
    let class = wide("CanvasClipClipboardListener");
    let hinstance = GetModuleHandleW(null());
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class.as_ptr(),
        hIconSm: null_mut(),
    };
    RegisterClassExW(&wc);
    let hwnd = CreateWindowExW(
        0,
        class.as_ptr(),
        null(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        null_mut(),
        hinstance,
        null(),
    );
    if hwnd.is_null() {
        log::error!("no se pudo crear la ventana del listener del portapapeles");
        return;
    }
    LISTENER.with(|l| {
        *l.borrow_mut() = Some(Listener {
            on_image,
            // No registrar lo que ya estaba en el portapapeles al arrancar.
            last_seq: GetClipboardSequenceNumber(),
            retries: 0,
        })
    });
    LISTENER_HWND.store(hwnd, Ordering::SeqCst);
    if AddClipboardFormatListener(hwnd) == 0 {
        log::error!("AddClipboardFormatListener falló");
        return;
    }
    log::info!("monitor del portapapeles activo");

    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
        DispatchMessageW(&msg);
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CLIPBOARDUPDATE => {
            // Reinicia el temporizador: se procesa una sola vez por ráfaga.
            SetTimer(hwnd, TIMER_SETTLE, SETTLE_MS, None);
            0
        }
        WM_TIMER if wparam == TIMER_SETTLE || wparam == TIMER_RETRY => {
            KillTimer(hwnd, wparam);
            if wparam == TIMER_SETTLE {
                LISTENER.with(|l| {
                    if let Some(l) = l.borrow_mut().as_mut() {
                        l.retries = 0;
                    }
                });
            }
            on_settled(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_settled(hwnd: HWND) {
    LISTENER.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some(l) = guard.as_mut() else { return };
        let seq = GetClipboardSequenceNumber();
        if seq == l.last_seq || seq == OWN_SEQ.load(Ordering::SeqCst) {
            return;
        }
        if !is_plain_image() || excluded() {
            // Texto, archivos, etc.: el MVP no los registra.
            l.last_seq = seq;
            return;
        }
        match read_image(hwnd) {
            Ok(Some(payload)) => {
                l.last_seq = seq;
                (l.on_image)(payload);
            }
            Ok(None) => l.last_seq = seq,
            Err(Busy) => {
                // Otra aplicación tiene el portapapeles abierto: reintentar.
                if l.retries < MAX_RETRIES {
                    l.retries += 1;
                    SetTimer(hwnd, TIMER_RETRY, RETRY_MS, None);
                } else {
                    log::warn!("portapapeles ocupado; captura omitida tras {MAX_RETRIES} reintentos");
                    l.last_seq = seq;
                }
            }
        }
    });
}

struct Busy;

/// Abre el portapapeles con reintentos cortos (puede estar bloqueado por otra app).
unsafe fn open(hwnd: HWND) -> Result<ClipboardGuard, Busy> {
    for attempt in 0..8 {
        if OpenClipboard(hwnd) != 0 {
            return Ok(ClipboardGuard);
        }
        thread::sleep(Duration::from_millis(15 * (attempt + 1)));
    }
    Err(Busy)
}

/// Cierra el portapapeles al salir del ámbito, incluso ante un error.
struct ClipboardGuard;

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

unsafe fn read_global(format: u32) -> Option<Vec<u8>> {
    let handle: HANDLE = GetClipboardData(format);
    if handle.is_null() {
        return None;
    }
    let size = GlobalSize(handle);
    let ptr = GlobalLock(handle) as *const u8;
    if ptr.is_null() || size == 0 {
        return None;
    }
    let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
    GlobalUnlock(handle);
    Some(bytes)
}

unsafe fn read_image(hwnd: HWND) -> Result<Option<ImagePayload>, Busy> {
    let _guard = open(hwnd)?;
    let png = png_format();
    if IsClipboardFormatAvailable(png) != 0 {
        if let Some(bytes) = read_global(png) {
            return Ok(Some(ImagePayload::Png(bytes)));
        }
    }
    for f in [CF_DIBV5 as u32, CF_DIB as u32] {
        if IsClipboardFormatAvailable(f) != 0 {
            if let Some(bytes) = read_global(f) {
                return Ok(Some(ImagePayload::Dib(bytes)));
            }
        }
    }
    Ok(None)
}

pub fn read() -> Result<ClipContent, String> {
    unsafe {
        if is_plain_image() {
            if let Some(img) = read_image(LISTENER_HWND.load(Ordering::SeqCst)).map_err(|_| busy_msg())? {
                return Ok(ClipContent::Image(img));
            }
        }
        if IsClipboardFormatAvailable(CF_UNICODETEXT as u32) != 0 {
            let _guard = open(LISTENER_HWND.load(Ordering::SeqCst)).map_err(|_| busy_msg())?;
            if let Some(bytes) = read_global(CF_UNICODETEXT as u32) {
                let units: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .take_while(|&u| u != 0)
                    .collect();
                return Ok(ClipContent::Text(String::from_utf16_lossy(&units)));
            }
        }
    }
    Ok(ClipContent::Empty)
}

fn busy_msg() -> String {
    "El portapapeles está ocupado por otra aplicación. Probá de nuevo.".into()
}

unsafe fn set_global(format: u32, bytes: &[u8]) -> bool {
    let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
    if h.is_null() {
        return false;
    }
    let ptr = GlobalLock(h) as *mut u8;
    if ptr.is_null() {
        GlobalFree(h);
        return false;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    GlobalUnlock(h);
    if SetClipboardData(format, h).is_null() {
        // Si SetClipboardData falla, la memoria sigue siendo nuestra.
        GlobalFree(h);
        return false;
    }
    true
}

unsafe fn write(items: &[(u32, &[u8])]) -> Result<(), String> {
    let ok = {
        let _guard = open(LISTENER_HWND.load(Ordering::SeqCst)).map_err(|_| busy_msg())?;
        EmptyClipboard();
        // Al menos un formato debe quedar escrito.
        items.iter().fold(false, |acc, (f, b)| set_global(*f, b) || acc)
    };
    // Tras cerrar, la secuencia queda fija: el monitor la reconocerá como propia.
    OWN_SEQ.store(GetClipboardSequenceNumber(), Ordering::SeqCst);
    if ok {
        Ok(())
    } else {
        Err("No se pudo escribir en el portapapeles".into())
    }
}

pub fn write_image(png: &[u8], dib: &[u8]) -> Result<(), String> {
    unsafe { write(&[(png_format(), png), (CF_DIB as u32, dib)]) }
}

pub fn write_text(text: &str) -> Result<(), String> {
    let bytes: Vec<u8> = text
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(|u| u.to_le_bytes())
        .collect();
    unsafe { write(&[(CF_UNICODETEXT as u32, &bytes)]) }
}
