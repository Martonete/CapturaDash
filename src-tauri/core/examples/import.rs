//! Herramienta de desarrollo: importa archivos PNG/JPEG a una carpeta de datos
//! de CanvasClip usando el mismo pipeline que el portapapeles.
//!
//! cargo run -p canvasclip-core --example import -- <carpeta-de-datos> <imagen>...

use canvasclip_core::imaging::ImagePayload;
use canvasclip_core::ingest::ingest;
use canvasclip_core::{Db, Storage};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("uso: import <carpeta-de-datos> <imagen>...");
    let storage = Storage::new(&dir).expect("carpeta de datos");
    let db = Db::open(&storage.db_path()).expect("base de datos");
    db.ensure_default_board().expect("pizarrón predeterminado");
    for path in args {
        let bytes = std::fs::read(&path).expect("leer imagen");
        match ingest(&db, &storage, ImagePayload::Encoded(bytes), "clipboard", 500) {
            Ok(r) if r.duplicate => println!("{path}: duplicada ({})", r.capture.id),
            Ok(r) => println!("{path}: {} {}x{}", r.capture.id, r.capture.width, r.capture.height),
            Err(e) => eprintln!("{path}: {e}"),
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
