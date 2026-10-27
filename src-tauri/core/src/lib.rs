//! Núcleo de CanvasClip, independiente de Tauri y de Windows: se puede
//! compilar y probar en cualquier plataforma (`cargo test -p canvasclip-core`).

pub mod boards;
pub mod captures;
pub mod db;
pub mod error;
pub mod imaging;
pub mod ingest;
pub mod maintenance;
pub mod models;
pub mod settings;
pub mod storage;

pub use db::Db;
pub use error::{CoreError, Result};
pub use storage::Storage;

#[cfg(test)]
mod tests;
