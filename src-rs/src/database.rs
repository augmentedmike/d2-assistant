//! SQLite persistence, schema migration, and local app settings.

use crate::d2x::requirements_from_item_record;
use crate::now_secs;
use rusqlite::OptionalExtension;
use std::fs;
use std::path::PathBuf;
use tauri::{Manager, command};

fn app_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {}", e))?;
    fs::create_dir_all(&app_dir).map_err(|e| format!("failed to create app data dir: {}", e))?;
    Ok(app_dir.join("d2a.db"))
}

/// Prefer new db name; migrate from legacy bank.db if needed.
pub(crate) fn open_app_db(app: &tauri::AppHandle) -> Result<rusqlite::Connection, String> {
    let db_path = app_db_path(app)?;
    if !db_path.exists() {
        let legacy = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to get app data dir: {}", e))?
            .join("bank.db");
        if legacy.exists() {
            let _ = fs::rename(&legacy, &db_path);
        }
    }
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| format!("db open error: {}", e))?;
    init_app_db(&conn)?;
    Ok(conn)
}

fn init_app_db(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            setting_key TEXT PRIMARY KEY,
            setting_value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("failed to initialize app settings: {e}"))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS character_save_state (
            original_path TEXT PRIMARY KEY,
            content_fingerprint TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("failed to init character save tracking: {e}"))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS character_backups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            original_path TEXT NOT NULL,
            snapshot_path TEXT NOT NULL,
            content_fingerprint TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("failed to init character backup history: {e}"))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bank_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            base_item TEXT NOT NULL,
            affixes_json TEXT NOT NULL DEFAULT '[]',
            level_req INTEGER NOT NULL DEFAULT 0,
            class_req TEXT NOT NULL DEFAULT 'Any',
            sockets INTEGER NOT NULL DEFAULT 0,
            icon_path TEXT,
            source_character TEXT NOT NULL DEFAULT '',
            banked_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            item_data BLOB,
            transferred_to TEXT,
            source_key TEXT UNIQUE,
            requirements_decoded INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| format!("failed to init bank table: {}", e))?;

    for (column, definition) in [
        ("item_data", "BLOB"),
        ("transferred_to", "TEXT"),
        ("source_key", "TEXT"),
        ("requirements_decoded", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        let exists: bool = conn
            .prepare("PRAGMA table_info(bank_items)")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| format!("failed to inspect bank schema: {e}"))?
            .iter()
            .any(|existing| existing == column);
        if !exists {
            conn.execute(
                &format!("ALTER TABLE bank_items ADD COLUMN {column} {definition}"),
                [],
            )
            .map_err(|e| format!("failed to add {column} bank field: {e}"))?;
        }
    }
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS bank_items_source_key ON bank_items(source_key)",
        [],
    )
    .map_err(|e| format!("failed to index bank source records: {e}"))?;
    conn.execute("DROP TABLE IF EXISTS notes", [])
        .map_err(|e| format!("failed to remove app-side project notes: {e}"))?;
    conn.execute(
        "UPDATE bank_items SET source_character = 'Stash import' WHERE source_character = 'ITAMZ (ATMA)'",
        [],
    )
    .map_err(|e| format!("failed to normalize bank import labels: {e}"))?;
    conn.execute(
        "UPDATE bank_items SET source_key = 'D2X:' || substr(source_key, 7) WHERE source_key LIKE 'ITAMZ:%'",
        [],
    )
    .map_err(|e| format!("failed to normalize stash identity keys: {e}"))?;
    conn.execute(
        "UPDATE bank_items SET level_req = -1 WHERE source_key LIKE 'D2X:%' AND level_req = 0 AND class_req = 'Unknown'",
        [],
    )
    .map_err(|e| format!("failed to mark unknown bank requirements: {e}"))?;
    let pending_requirements = conn
        .prepare("SELECT id, item_data FROM bank_items WHERE item_data IS NOT NULL AND requirements_decoded = 0")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| format!("failed to load bank items for requirement decoding: {e}"))?;
    for (id, item_data) in pending_requirements {
        let (level_req, class_req) =
            requirements_from_item_record(&item_data).unwrap_or_else(|_| (-1, "Unknown".into()));
        conn.execute(
            "UPDATE bank_items SET level_req = ?1, class_req = ?2, requirements_decoded = 1 WHERE id = ?3",
            rusqlite::params![level_req, class_req, id],
        )
        .map_err(|e| format!("failed to save decoded bank requirements: {e}"))?;
    }

    Ok(())
}

#[command]
pub fn get_app_setting(app: tauri::AppHandle, key: String) -> Result<Option<String>, String> {
    let conn = open_app_db(&app)?;
    conn.query_row(
        "SELECT setting_value FROM app_settings WHERE setting_key = ?1",
        [key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| format!("could not load app setting: {e}"))
}

#[command]
pub fn set_app_setting(app: tauri::AppHandle, key: String, value: String) -> Result<(), String> {
    validate_setting(&key, &value).map_err(str::to_string)?;
    let conn = open_app_db(&app)?;
    conn.execute(
        "INSERT INTO app_settings (setting_key, setting_value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value, updated_at = excluded.updated_at",
        rusqlite::params![key, value, now_secs()?],
    )
    .map_err(|e| format!("could not save app setting: {e}"))?;
    Ok(())
}

fn validate_setting(key: &str, value: &str) -> Result<(), &'static str> {
    if key.trim().is_empty() || key.len() > 128 {
        return Err("setting key must be between 1 and 128 characters");
    }
    if value.len() > 4096 {
        return Err("setting value is too long");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_require_bounded_nonempty_keys_and_values() {
        assert!(validate_setting("save-folder", "/games/d2").is_ok());
        assert!(validate_setting("  ", "value").is_err());
        assert!(validate_setting(&"k".repeat(129), "value").is_err());
        assert!(validate_setting("save-folder", &"x".repeat(4097)).is_err());
    }

    #[test]
    fn schema_initialization_is_safe_to_repeat() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        init_app_db(&conn).unwrap();
        init_app_db(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('app_settings', 'bank_items', 'character_backups', 'character_save_state')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
    }
}
