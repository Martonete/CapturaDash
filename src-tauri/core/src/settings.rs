//! Preferencias persistentes. Se guardan como un único JSON en `settings`
//! para que agregar una preferencia nueva no requiera migraciones.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;

const KEY_APP: &str = "app";
const KEY_LAST_BOARD: &str = "lastBoardId";
const KEY_FIRST_RUN_DONE: &str = "firstRunDone";

pub const MIN_HISTORY: u32 = 20;
pub const MAX_HISTORY: u32 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // General
    pub launch_at_startup: bool,
    pub start_minimized: bool,
    pub monitoring: bool,
    // Panel
    pub shortcut: String,
    pub edge_enabled: bool,
    pub edge_delay_ms: u32,
    pub panel_animations: bool,
    /// "auto": se cierra al hacer clic afuera o al alejar el cursor.
    /// "manual": solo con Escape, el atajo o el botón.
    pub close_behavior: String,
    // Capturas
    pub history_limit: u32,
    pub notifications: bool,
    // Apariencia
    /// "comfortable" | "compact"
    pub density: String,
    /// "off" | "subtle" | "full"
    pub effects: String,
    pub reduce_motion: bool,
    /// "dots" | "none"
    pub grid: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            launch_at_startup: true,
            start_minimized: true,
            monitoring: true,
            shortcut: "Alt+Shift+V".into(),
            edge_enabled: true,
            edge_delay_ms: 300,
            panel_animations: true,
            close_behavior: "auto".into(),
            history_limit: 500,
            notifications: true,
            density: "comfortable".into(),
            effects: "subtle".into(),
            reduce_motion: false,
            grid: "dots".into(),
        }
    }
}

impl Settings {
    /// Corrige valores fuera de rango o desconocidos.
    pub fn normalized(mut self) -> Self {
        let d = Settings::default();
        self.history_limit = self.history_limit.clamp(MIN_HISTORY, MAX_HISTORY);
        self.edge_delay_ms = self.edge_delay_ms.clamp(100, 2000);
        if !matches!(self.close_behavior.as_str(), "auto" | "manual") {
            self.close_behavior = d.close_behavior;
        }
        if !matches!(self.density.as_str(), "comfortable" | "compact") {
            self.density = d.density;
        }
        if !matches!(self.effects.as_str(), "off" | "subtle" | "full") {
            self.effects = d.effects;
        }
        if !matches!(self.grid.as_str(), "dots" | "none") {
            self.grid = d.grid;
        }
        if self.shortcut.trim().is_empty() {
            self.shortcut = d.shortcut;
        }
        self
    }
}

impl Db {
    fn get_kv(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
            .optional()?)
    }

    fn set_kv(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<Settings> {
        Ok(match self.get_kv(KEY_APP)? {
            // Campos faltantes toman su valor por defecto (serde default).
            Some(json) => serde_json::from_str::<Settings>(&json).unwrap_or_default().normalized(),
            None => Settings::default(),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        let json = serde_json::to_string(s).expect("settings serializables");
        self.set_kv(KEY_APP, &json)
    }

    pub fn last_board_id(&self) -> Result<Option<String>> {
        self.get_kv(KEY_LAST_BOARD)
    }

    pub fn set_last_board_id(&self, id: &str) -> Result<()> {
        self.set_kv(KEY_LAST_BOARD, id)
    }

    /// Devuelve true solo la primera vez que se llama en esta base de datos.
    pub fn take_first_run(&self) -> Result<bool> {
        if self.get_kv(KEY_FIRST_RUN_DONE)?.is_some() {
            return Ok(false);
        }
        self.set_kv(KEY_FIRST_RUN_DONE, "1")?;
        Ok(true)
    }
}
