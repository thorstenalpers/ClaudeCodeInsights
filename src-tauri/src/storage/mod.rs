pub mod schema;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Opens the database and brings it to the current schema.
///
/// WAL matters here: a scan holds a write connection open for a while, and
/// without it every read from the UI would block behind it.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    schema::migrate(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_migrates_in_memory() {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();

        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, schema::TARGET_VERSION);

        // Migrating twice must be a no-op, not an error.
        schema::migrate(&conn).unwrap();
    }

    #[test]
    fn message_id_is_unique_but_null_is_not() {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();

        let insert = "INSERT INTO turns (session_id, message_id, ts_utc, scan_file_id)
                      VALUES (?1, ?2, '2026-01-01T00:00:00Z', 1)";

        conn.execute(insert, ("s1", Some("msg-1"))).unwrap();
        assert!(conn.execute(insert, ("s1", Some("msg-1"))).is_err());

        // Turns with no message_id are the reasoning bucket; they must not
        // collide with each other.
        conn.execute(insert, ("s1", None::<String>)).unwrap();
        conn.execute(insert, ("s1", None::<String>)).unwrap();
    }
}
