//! Pizarrones, objetos (imágenes y notas) y grupos.

use std::collections::HashMap;

use rusqlite::{params, OptionalExtension, Row, Transaction};

use crate::db::{new_id, now_ms, Db};
use crate::error::{CoreError, Result};
use crate::models::{
    Board, BoardChanges, BoardDocument, BoardObject, BoardSummary, Group, PreviewItem, SaveReport,
};

pub const DEFAULT_BOARD_NAME: &str = "General";
const PREVIEW_LIMIT: usize = 24;
/// Separación entre objetos al ubicar uno nuevo automáticamente.
const PLACEMENT_GAP: f64 = 40.0;

const BOARD_COLS: &str = "id, name, created_at, updated_at, camera_x, camera_y, zoom";
const OBJECT_COLS: &str =
    "id, board_id, kind, capture_id, x, y, width, height, z, group_id, props, created_at, updated_at";

fn board_from_row(r: &Row) -> rusqlite::Result<Board> {
    Ok(Board {
        id: r.get(0)?,
        name: r.get(1)?,
        created_at: r.get(2)?,
        updated_at: r.get(3)?,
        camera_x: r.get(4)?,
        camera_y: r.get(5)?,
        zoom: r.get(6)?,
    })
}

fn object_from_row(r: &Row) -> rusqlite::Result<BoardObject> {
    let props: String = r.get(10)?;
    Ok(BoardObject {
        id: r.get(0)?,
        board_id: r.get(1)?,
        kind: r.get(2)?,
        capture_id: r.get(3)?,
        x: r.get(4)?,
        y: r.get(5)?,
        width: r.get(6)?,
        height: r.get(7)?,
        z: r.get(8)?,
        group_id: r.get(9)?,
        props: serde_json::from_str(&props).unwrap_or(serde_json::Value::Object(Default::default())),
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

fn clean_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CoreError::Invalid("El nombre no puede estar vacío".into()));
    }
    Ok(name.chars().take(80).collect())
}

fn valid_geometry(o: &BoardObject) -> bool {
    [o.x, o.y, o.width, o.height].iter().all(|v| v.is_finite())
        && o.width > 0.0
        && o.height > 0.0
        && (o.kind == "image" && o.capture_id.is_some() || o.kind == "note")
}

impl Db {
    /// Garantiza que exista al menos un pizarrón; devuelve el id del primero.
    pub fn ensure_default_board(&self) -> Result<String> {
        let existing: Option<String> = self
            .conn()
            .query_row("SELECT id FROM boards ORDER BY created_at LIMIT 1", [], |r| r.get(0))
            .optional()?;
        match existing {
            Some(id) => Ok(id),
            None => Ok(self.create_board(DEFAULT_BOARD_NAME)?.id),
        }
    }

    pub fn board_exists(&self, id: &str) -> Result<bool> {
        Ok(self
            .conn()
            .query_row("SELECT 1 FROM boards WHERE id = ?1", [id], |_| Ok(()))
            .optional()?
            .is_some())
    }

    pub fn list_boards(&self) -> Result<Vec<BoardSummary>> {
        let conn = self.conn();
        let boards: Vec<Board> = {
            let mut stmt =
                conn.prepare(&format!("SELECT {BOARD_COLS} FROM boards ORDER BY created_at"))?;
            let rows = stmt.query_map([], board_from_row)?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut stmt = conn.prepare_cached(
            "SELECT kind, x, y, width, height, capture_id, json_extract(props, '$.color')
             FROM board_objects WHERE board_id = ?1 ORDER BY z",
        )?;
        let mut out = Vec::with_capacity(boards.len());
        for board in boards {
            let items: Vec<PreviewItem> = stmt
                .query_map([&board.id], |r| {
                    Ok(PreviewItem {
                        kind: r.get(0)?,
                        x: r.get(1)?,
                        y: r.get(2)?,
                        width: r.get(3)?,
                        height: r.get(4)?,
                        capture_id: r.get(5)?,
                        color: r.get(6)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            let object_count = items.len() as u32;
            // Los últimos en orden z son los más visibles.
            let skip = items.len().saturating_sub(PREVIEW_LIMIT);
            out.push(BoardSummary {
                board,
                object_count,
                preview: items.into_iter().skip(skip).collect(),
            });
        }
        Ok(out)
    }

    pub fn get_board(&self, id: &str) -> Result<Board> {
        self.conn()
            .query_row(&format!("SELECT {BOARD_COLS} FROM boards WHERE id = ?1"), [id], board_from_row)
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("pizarrón {id}")))
    }

    pub fn create_board(&self, name: &str) -> Result<Board> {
        let name = clean_name(name)?;
        let now = now_ms();
        let board = Board {
            id: new_id(),
            name,
            created_at: now,
            updated_at: now,
            camera_x: 0.0,
            camera_y: 0.0,
            zoom: 1.0,
        };
        self.conn().execute(
            "INSERT INTO boards (id, name, created_at, updated_at, camera_x, camera_y, zoom)
             VALUES (?1, ?2, ?3, ?4, 0, 0, 1)",
            params![board.id, board.name, now, now],
        )?;
        Ok(board)
    }

    pub fn rename_board(&self, id: &str, name: &str) -> Result<Board> {
        let name = clean_name(name)?;
        let n = self.conn().execute(
            "UPDATE boards SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, name, now_ms()],
        )?;
        if n == 0 {
            return Err(CoreError::NotFound(format!("pizarrón {id}")));
        }
        self.get_board(id)
    }

    /// Duplica un pizarrón con todos sus objetos y grupos. Las capturas no se
    /// copian: el duplicado referencia los mismos archivos.
    pub fn duplicate_board(&self, id: &str) -> Result<Board> {
        let src = self.get_board(id)?;
        let doc = self.load_board(id)?;
        let now = now_ms();
        let new_board = Board {
            id: new_id(),
            name: format!("{} (copia)", src.name).chars().take(80).collect(),
            created_at: now,
            updated_at: now,
            ..src
        };
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO boards (id, name, created_at, updated_at, camera_x, camera_y, zoom)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_board.id, new_board.name, now, now,
                new_board.camera_x, new_board.camera_y, new_board.zoom
            ],
        )?;
        let mut group_map = HashMap::new();
        for g in &doc.groups {
            let gid = new_id();
            tx.execute(
                "INSERT INTO board_groups (id, board_id, created_at) VALUES (?1, ?2, ?3)",
                params![gid, new_board.id, now],
            )?;
            group_map.insert(g.id.clone(), gid);
        }
        for o in doc.objects {
            let copy = BoardObject {
                id: new_id(),
                board_id: new_board.id.clone(),
                group_id: o.group_id.as_ref().and_then(|g| group_map.get(g).cloned()),
                created_at: now,
                updated_at: now,
                ..o
            };
            upsert_object(&tx, &copy)?;
        }
        tx.commit()?;
        Ok(new_board)
    }

    /// Elimina un pizarrón (objetos y grupos en cascada). Las capturas quedan
    /// en el historial. Si era el último, se crea uno nuevo vacío.
    pub fn delete_board(&self, id: &str) -> Result<String> {
        let n = self.conn().execute("DELETE FROM boards WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(CoreError::NotFound(format!("pizarrón {id}")));
        }
        self.ensure_default_board()
    }

    pub fn load_board(&self, id: &str) -> Result<BoardDocument> {
        let board = self.get_board(id)?;
        let conn = self.conn();
        let objects = {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT {OBJECT_COLS} FROM board_objects WHERE board_id = ?1 ORDER BY z, id"
            ))?;
            let rows = stmt.query_map([id], object_from_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let groups = {
            let mut stmt = conn.prepare_cached(
                "SELECT id, board_id, created_at FROM board_groups WHERE board_id = ?1",
            )?;
            let rows = stmt.query_map([id], |r| {
                Ok(Group { id: r.get(0)?, board_id: r.get(1)?, created_at: r.get(2)? })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(BoardDocument { board, objects, groups })
    }

    /// Aplica cambios incrementales en una sola transacción: o se guarda todo
    /// o nada. Los objetos cuya captura ya no existe se descartan (en lugar
    /// de hacer fallar el guardado completo) y se informan.
    pub fn save_board_changes(&self, changes: &BoardChanges) -> Result<SaveReport> {
        let board_id = &changes.board_id;
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let exists = tx
            .query_row("SELECT 1 FROM boards WHERE id = ?1", [board_id], |_| Ok(()))
            .optional()?
            .is_some();
        if !exists {
            return Err(CoreError::NotFound(format!("pizarrón {board_id}")));
        }

        for g in &changes.upsert_groups {
            if &g.board_id != board_id {
                return Err(CoreError::Invalid("grupo de otro pizarrón".into()));
            }
            tx.execute(
                "INSERT INTO board_groups (id, board_id, created_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO NOTHING",
                params![g.id, g.board_id, g.created_at],
            )?;
        }

        let mut report = SaveReport::default();
        for o in &changes.upsert_objects {
            if &o.board_id != board_id {
                return Err(CoreError::Invalid("objeto de otro pizarrón".into()));
            }
            if !valid_geometry(o) {
                report.skipped_objects.push(o.id.clone());
                continue;
            }
            if let Some(cid) = &o.capture_id {
                let capture_ok = tx
                    .query_row("SELECT 1 FROM captures WHERE id = ?1", [cid], |_| Ok(()))
                    .optional()?
                    .is_some();
                if !capture_ok {
                    report.skipped_objects.push(o.id.clone());
                    continue;
                }
            }
            let mut o = o.clone();
            if let Some(gid) = &o.group_id {
                let group_ok = tx
                    .query_row(
                        "SELECT 1 FROM board_groups WHERE id = ?1 AND board_id = ?2",
                        params![gid, board_id],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some();
                if !group_ok {
                    o.group_id = None;
                }
            }
            upsert_object(&tx, &o)?;
        }

        for id in &changes.delete_objects {
            tx.execute(
                "DELETE FROM board_objects WHERE id = ?1 AND board_id = ?2",
                params![id, board_id],
            )?;
        }
        for id in &changes.delete_groups {
            tx.execute(
                "DELETE FROM board_groups WHERE id = ?1 AND board_id = ?2",
                params![id, board_id],
            )?;
        }
        tx.execute("UPDATE boards SET updated_at = ?2 WHERE id = ?1", params![board_id, now_ms()])?;
        tx.commit()?;
        Ok(report)
    }

    pub fn save_camera(&self, id: &str, x: f64, y: f64, zoom: f64) -> Result<()> {
        if ![x, y, zoom].iter().all(|v| v.is_finite()) || zoom <= 0.0 {
            return Err(CoreError::Invalid("cámara inválida".into()));
        }
        self.conn().execute(
            "UPDATE boards SET camera_x = ?2, camera_y = ?3, zoom = ?4 WHERE id = ?1",
            params![id, x, y, zoom],
        )?;
        Ok(())
    }

    /// Inserta una captura en un pizarrón, a la derecha del contenido existente
    /// (o en el origen si está vacío). Usado desde el panel lateral.
    pub fn insert_capture_object(
        &self,
        board_id: &str,
        capture_id: &str,
        width: f64,
        height: f64,
    ) -> Result<BoardObject> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let bounds: (Option<f64>, Option<f64>, Option<i64>) = tx.query_row(
            "SELECT MAX(x + width), MIN(y), MAX(z) FROM board_objects WHERE board_id = ?1",
            [board_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let now = now_ms();
        let obj = BoardObject {
            id: new_id(),
            board_id: board_id.to_string(),
            kind: "image".into(),
            capture_id: Some(capture_id.to_string()),
            x: bounds.0.map(|r| r + PLACEMENT_GAP).unwrap_or(-width / 2.0),
            y: bounds.1.unwrap_or(-height / 2.0),
            width,
            height,
            z: bounds.2.map(|z| z + 1).unwrap_or(0),
            group_id: None,
            props: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        };
        if !valid_geometry(&obj) {
            return Err(CoreError::Invalid("tamaño inválido".into()));
        }
        upsert_object(&tx, &obj)?;
        tx.execute("UPDATE boards SET updated_at = ?2 WHERE id = ?1", params![board_id, now])?;
        tx.commit()?;
        Ok(obj)
    }
}

fn upsert_object(tx: &Transaction, o: &BoardObject) -> Result<()> {
    let props = if o.props.is_null() { "{}".to_string() } else { o.props.to_string() };
    tx.execute(
        "INSERT INTO board_objects (id, board_id, kind, capture_id, x, y, width, height, z,
             group_id, props, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
             x = excluded.x, y = excluded.y, width = excluded.width, height = excluded.height,
             z = excluded.z, group_id = excluded.group_id, props = excluded.props,
             updated_at = excluded.updated_at
         WHERE board_objects.board_id = excluded.board_id",
        params![
            o.id, o.board_id, o.kind, o.capture_id, o.x, o.y, o.width, o.height, o.z,
            o.group_id, props, o.created_at, o.updated_at
        ],
    )?;
    Ok(())
}
