//! Conexión SQLite y migraciones versionadas (`PRAGMA user_version`).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::{Mutex, MutexGuard};
use rusqlite::Connection;

use crate::error::Result;

/// Migraciones en orden. El índice + 1 es la versión que dejan aplicada.
/// Nunca editar una migración publicada: agregar una nueva al final.
const MIGRATIONS: &[&str] = &[include_str!("../../migrations/0001_init.sql")];

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        // WAL + synchronous=FULL: las escrituras confirmadas sobreviven a un
        // cierre inesperado o un corte de energía. El volumen de escrituras es
        // bajo (guardado con debounce), así que el costo es despreciable.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = FULL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        migrate(&mut conn)?;
        Ok(Db { conn: Mutex::new(conn) })
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock()
    }

    /// Verificación rápida de integridad al arrancar.
    pub fn quick_check(&self) -> Result<bool> {
        let res: String = self.conn().query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        Ok(res == "ok")
    }

    /// Copia consistente de la base completa a otra ruta (para mover el almacenamiento).
    pub fn vacuum_into(&self, dest: &Path) -> Result<()> {
        let dest = dest.to_string_lossy().to_string();
        self.conn().execute("VACUUM INTO ?1", [dest])?;
        Ok(())
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current: usize = conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let version = i + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
        tx.commit()?;
        log::info!("migración de base de datos aplicada: v{version}");
    }
    Ok(())
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn new_id() -> String {
    // UUIDv7: ordenable por tiempo, útil para depurar y para índices.
    uuid::Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let db = Db::open(&path).unwrap();
            let v: i64 = db.conn().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
            assert_eq!(v as usize, MIGRATIONS.len());
        }
        // Reabrir no debe volver a aplicar migraciones.
        let db = Db::open(&path).unwrap();
        assert!(db.quick_check().unwrap());
    }
}
