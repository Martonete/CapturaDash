//! Tipos compartidos con el frontend (serializados en camelCase).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub id: String,
    pub hash: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub thumb_width: u32,
    pub thumb_height: u32,
    pub byte_size: u64,
    pub source: String,
    pub favorite: bool,
    pub created_at: i64,
    /// Cantidad de pizarrones distintos que usan esta captura.
    pub usage_count: u32,
}

/// Rutas internas de una captura; nunca se exponen al frontend.
#[derive(Debug, Clone)]
pub struct CaptureFiles {
    pub file_name: String,
    pub thumb_name: String,
    pub mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub camera_x: f64,
    pub camera_y: f64,
    pub zoom: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub capture_id: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardSummary {
    #[serde(flatten)]
    pub board: Board,
    pub object_count: u32,
    /// Hasta `PREVIEW_LIMIT` objetos para dibujar la miniatura del pizarrón.
    pub preview: Vec<PreviewItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoardObject {
    pub id: String,
    pub board_id: String,
    /// 'image' | 'note'
    pub kind: String,
    pub capture_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z: i64,
    pub group_id: Option<String>,
    #[serde(default)]
    pub props: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub board_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardDocument {
    pub board: Board,
    pub objects: Vec<BoardObject>,
    pub groups: Vec<Group>,
}

/// Cambios incrementales de un pizarrón; se aplican en una sola transacción.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardChanges {
    pub board_id: String,
    #[serde(default)]
    pub upsert_objects: Vec<BoardObject>,
    #[serde(default)]
    pub delete_objects: Vec<String>,
    #[serde(default)]
    pub upsert_groups: Vec<Group>,
    #[serde(default)]
    pub delete_groups: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReport {
    /// Objetos descartados porque su captura ya no existe.
    pub skipped_objects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub data_dir: String,
    pub capture_count: u32,
    pub protected_count: u32,
    pub bytes: u64,
}
