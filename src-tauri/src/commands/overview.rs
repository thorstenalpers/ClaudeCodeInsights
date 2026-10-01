use super::db;
use crate::error::Result;
use serde::Serialize;

/// What the Overview page needs for its headline figures.
///
/// Cost is deliberately absent: it is derived from the token counts and the
/// price table at display time, so changing a rate never means rescanning.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub sessions: i64,
    pub turns: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub active_days: i64,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
}

#[tauri::command]
pub fn get_overview() -> Result<Overview> {
    let overview = db()?.query_row(
        r#"
        SELECT
            -- Sessions holding no turns are title-only leftovers; counting
            -- them here would disagree with the sessions list.
            (SELECT COUNT(*) FROM sessions WHERE turn_count > 0),
            COUNT(*),
            COALESCE(SUM(input_tokens), 0),
            COALESCE(SUM(output_tokens), 0),
            COALESCE(SUM(cache_read_tokens), 0),
            COALESCE(SUM(cache_write_tokens), 0),
            COUNT(DISTINCT substr(ts_utc, 1, 10)),
            MIN(ts_utc),
            MAX(ts_utc)
        FROM turns
        "#,
        [],
        |row| {
            Ok(Overview {
                sessions: row.get(0)?,
                turns: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                cache_read_tokens: row.get(4)?,
                cache_write_tokens: row.get(5)?,
                active_days: row.get(6)?,
                first_ts: row.get(7)?,
                last_ts: row.get(8)?,
            })
        },
    )?;

    Ok(overview)
}
