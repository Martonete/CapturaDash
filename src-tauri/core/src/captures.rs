//! Historial de capturas: consultas, favoritos, borrado y retención.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::db::Db;
use crate::error::{CoreError, Result};
use crate::models::{Capture, CaptureFiles};

const SELECT: &str = "SELECT c.id, c.hash, c.mime, c.width, c.height, c.thumb_width, c.thumb_height,
        c.byte_size, c.source, c.favorite, c.created_at,
        (SELECT COUNT(DISTINCT o.board_id) FROM board_objects o WHERE o.capture_id = c.id)
     FROM captures c";

fn from_row(r: &Row) -> rusqlite::Result<Capture> {
    Ok(Capture {
        id: r.get(0)?,
        hash: r.get(1)?,
        mime: r.get(2)?,
        width: r.get(3)?,
        height: r.get(4)?,
        thumb_width: r.get(5)?,
        thumb_height: r.get(6)?,
        byte_size: r.get::<_, i64>(7)? as u64,
        source: r.get(8)?,
        favorite: r.get(9)?,
        created_at: r.get(10)?,
        usage_count: r.get(11)?,
    })
}

/// Fila nueva lista para insertar (los archivos ya están escritos en disco).
pub struct NewCapture<'a> {
    pub id: &'a str,
    pub hash: &'a str,
    pub file_name: &'a str,
    pub thumb_name: &'a str,
    pub mime: &'a str,
    pub width: u32,
    pub height: u32,
    pub thumb_width: u32,
    pub thumb_height: u32,
    pub byte_size: u64,
    pub source: &'a str,
    pub created_at: i64,
}

/// Archivos a borrar del disco después de confirmar una transacción.
#[derive(Debug, Default)]
pub struct Removed {
    pub ids: Vec<String>,
    pub files: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Favorites,
}

impl Db {
    pub fn list_captures(&self, filter: Filter, limit: u32, offset: u32) -> Result<Vec<Capture>> {
        let sql = match filter {
            Filter::All => format!("{SELECT} ORDER BY c.created_at DESC, c.id DESC LIMIT ?1 OFFSET ?2"),
            Filter::Favorites => format!(
                "{SELECT} WHERE c.favorite = 1 ORDER BY c.created_at DESC, c.id DESC LIMIT ?1 OFFSET ?2"
            ),
        };
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params![limit, offset], from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_capture(&self, id: &str) -> Result<Capture> {
        get(&self.conn(), id)?.ok_or_else(|| CoreError::NotFound(format!("captura {id}")))
    }

    pub fn find_capture_by_hash(&self, hash: &str) -> Result<Option<Capture>> {
        let conn = self.conn();
        let sql = format!("{SELECT} WHERE c.hash = ?1");
        Ok(conn.query_row(&sql, [hash], from_row).optional()?)
    }

    pub fn capture_files(&self, id: &str) -> Result<CaptureFiles> {
        self.conn()
            .query_row(
                "SELECT file_name, thumb_name, mime FROM captures WHERE id = ?1",
                [id],
                |r| Ok(CaptureFiles { file_name: r.get(0)?, thumb_name: r.get(1)?, mime: r.get(2)? }),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("captura {id}")))
    }

    pub fn insert_capture(&self, c: &NewCapture) -> Result<()> {
        self.conn().execute(
            "INSERT INTO captures (id, hash, file_name, thumb_name, mime, width, height,
                 thumb_width, thumb_height, byte_size, source, favorite, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12)",
            params![
                c.id, c.hash, c.file_name, c.thumb_name, c.mime, c.width, c.height,
                c.thumb_width, c.thumb_height, c.byte_size as i64, c.source, c.created_at
            ],
        )?;
        Ok(())
    }

    pub fn update_thumb(&self, id: &str, thumb_name: &str, w: u32, h: u32) -> Result<()> {
        self.conn().execute(
            "UPDATE captures SET thumb_name = ?2, thumb_width = ?3, thumb_height = ?4 WHERE id = ?1",
            params![id, thumb_name, w, h],
        )?;
        Ok(())
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<Capture> {
        let n = self
            .conn()
            .execute("UPDATE captures SET favorite = ?2 WHERE id = ?1", params![id, favorite])?;
        if n == 0 {
            return Err(CoreError::NotFound(format!("captura {id}")));
        }
        self.get_capture(id)
    }

    /// Borra una captura del historial. Si se usa en pizarrones y `force` es
    /// falso devuelve `InUse`; con `force` elimina también esas referencias.
    /// Devuelve los ids de pizarrones afectados.
    pub fn delete_capture(&self, id: &str, force: bool) -> Result<(Removed, Vec<String>)> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let boards: Vec<String> = {
            let mut stmt =
                tx.prepare("SELECT DISTINCT board_id FROM board_objects WHERE capture_id = ?1")?;
            let rows = stmt.query_map([id], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        if !boards.is_empty() && !force {
            return Err(CoreError::InUse(boards.len() as u32));
        }
        tx.execute("DELETE FROM board_objects WHERE capture_id = ?1", [id])?;
        let files = tx
            .query_row(
                "DELETE FROM captures WHERE id = ?1 RETURNING file_name, thumb_name",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound(format!("captura {id}")))?;
        tx.commit()?;
        Ok((Removed { ids: vec![id.to_string()], files: vec![files] }, boards))
    }

    /// Política de retención: conserva las `limit` capturas no protegidas más
    /// recientes. Protegidas = favoritas o usadas en algún pizarrón; nunca se
    /// borran automáticamente y no cuentan para el límite.
    pub fn enforce_retention(&self, limit: u32) -> Result<Removed> {
        self.delete_unprotected(Some(limit))
    }

    /// Limpieza manual: borra todas las capturas no protegidas.
    pub fn clear_unprotected(&self) -> Result<Removed> {
        self.delete_unprotected(None)
    }

    fn delete_unprotected(&self, keep: Option<u32>) -> Result<Removed> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let mut removed = Removed::default();
        {
            let mut stmt = tx.prepare(
                "DELETE FROM captures WHERE id IN (
                     SELECT c.id FROM captures c
                     WHERE c.favorite = 0
                       AND NOT EXISTS (SELECT 1 FROM board_objects o WHERE o.capture_id = c.id)
                     ORDER BY c.created_at DESC, c.id DESC
                     LIMIT -1 OFFSET ?1)
                 RETURNING id, file_name, thumb_name",
            )?;
            let rows = stmt.query_map([keep.unwrap_or(0)], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
            })?;
            for row in rows {
                let (id, f, t) = row?;
                removed.ids.push(id);
                removed.files.push((f, t));
            }
        }
        tx.commit()?;
        Ok(removed)
    }

    /// (total, protegidas)
    pub fn capture_counts(&self) -> Result<(u32, u32)> {
        Ok(self.conn().query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE WHEN favorite = 1
                        OR EXISTS (SELECT 1 FROM board_objects o WHERE o.capture_id = c.id)
                        THEN 1 ELSE 0 END), 0)
             FROM captures c",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    /// Todas las filas con sus archivos (para la reconciliación al arrancar).
    pub fn all_capture_files(&self) -> Result<Vec<(String, CaptureFiles, bool)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT c.id, c.file_name, c.thumb_name, c.mime,
                    EXISTS (SELECT 1 FROM board_objects o WHERE o.capture_id = c.id) OR c.favorite = 1
             FROM captures c",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                CaptureFiles { file_name: r.get(1)?, thumb_name: r.get(2)?, mime: r.get(3)? },
                r.get(4)?,
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

fn get(conn: &Connection, id: &str) -> Result<Option<Capture>> {
    let sql = format!("{SELECT} WHERE c.id = ?1");
    Ok(conn.query_row(&sql, [id], from_row).optional()?)
}
