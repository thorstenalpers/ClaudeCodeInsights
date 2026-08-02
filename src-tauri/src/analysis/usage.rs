//! The three roll-ups the Tools, Cost and Agents pages ask for.
//!
//! Each is a single grouped query. Nothing is materialised: the tables they
//! read are already the materialised form, and a second cache would be one more
//! thing to invalidate after a scan.

use crate::analysis::cost::{self, ModelTokens};
use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRow {
    pub name: String,
    pub calls: i64,
    pub sessions: i64,
    /// The turns that reached for this tool, shared out over their calls.
    pub by_model: Vec<ModelTokens>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRow {
    pub model: String,
    pub turns: i64,
    pub sessions: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRow {
    pub agent_type: String,
    pub runs: i64,
    pub total_tokens: i64,
    pub total_duration_ms: i64,
    pub tool_use_count: i64,
    pub last_ts: Option<String>,
    /// The subagent's own turns, so its cost is priced per model rather than
    /// from the blended total the transcript reports.
    pub by_model: Vec<ModelTokens>,
}

/// Tool calls per tool, with the number of sessions that reached for it.
///
/// Counted from `session_tool_counts` rather than `turn_tools`: it is the same
/// figure one join earlier, and it carries the per-session grouping this needs.
pub fn tools(conn: &Connection) -> Result<Vec<ToolRow>> {
    let mut stmt = conn.prepare(
        "SELECT tool_name, SUM(calls), COUNT(DISTINCT session_id)
         FROM session_tool_counts
         GROUP BY tool_name
         ORDER BY SUM(calls) DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ToolRow {
            name: row.get(0)?,
            calls: row.get(1)?,
            sessions: row.get(2)?,
            by_model: Vec::new(),
        })
    })?;

    let mut rows = rows.collect::<Result<Vec<_>, _>>()?;
    let mut split = cost::by_tool(conn)?;
    for row in &mut rows {
        row.by_model = split.remove(&row.name).unwrap_or_default();
    }
    Ok(rows)
}

/// Token totals per model. Cost stays out of here on purpose — it is derived
/// from these counts and the price table at display time.
pub fn models(conn: &Connection) -> Result<Vec<ModelRow>> {
    let mut stmt = conn.prepare(
        "SELECT model,
                COUNT(*),
                COUNT(DISTINCT session_id),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(cache_read_tokens), 0),
                COALESCE(SUM(cache_write_tokens), 0),
                MIN(ts_utc),
                MAX(ts_utc)
         FROM turns
         WHERE model IS NOT NULL AND model <> ''
         GROUP BY model
         ORDER BY SUM(input_tokens + output_tokens + cache_read_tokens) DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ModelRow {
            model: row.get(0)?,
            turns: row.get(1)?,
            sessions: row.get(2)?,
            input_tokens: row.get(3)?,
            output_tokens: row.get(4)?,
            cache_read_tokens: row.get(5)?,
            cache_write_tokens: row.get(6)?,
            first_ts: row.get(7)?,
            last_ts: row.get(8)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn agents(conn: &Connection) -> Result<Vec<AgentRow>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(agent_type, 'unknown'),
                COUNT(*),
                COALESCE(SUM(total_tokens), 0),
                COALESCE(SUM(total_duration_ms), 0),
                COALESCE(SUM(tool_use_count), 0),
                MAX(completed_ts)
         FROM agents
         GROUP BY COALESCE(agent_type, 'unknown')
         ORDER BY COUNT(*) DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(AgentRow {
            agent_type: row.get(0)?,
            runs: row.get(1)?,
            total_tokens: row.get(2)?,
            total_duration_ms: row.get(3)?,
            tool_use_count: row.get(4)?,
            last_ts: row.get(5)?,
            by_model: Vec::new(),
        })
    })?;

    let mut rows = rows.collect::<Result<Vec<_>, _>>()?;
    let mut split = cost::by_agent_type(conn)?;
    for row in &mut rows {
        row.by_model = split.remove(&row.agent_type).unwrap_or_default();
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema;

    fn seeded() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO scan_files (id, path, mtime_ms, size_bytes, line_count, scanned_at)
            VALUES (1, 'f.jsonl', 0, 0, 0, '2026-01-01T00:00:00Z');

            INSERT INTO turns (session_id, message_id, ts_utc, model, input_tokens,
                               output_tokens, cache_read_tokens, cache_write_tokens, scan_file_id)
            VALUES ('s1', 'm1', '2026-01-01T10:00:00Z', 'claude-opus-5',   100, 10, 1000, 5, 1),
                   ('s1', 'm2', '2026-01-02T10:00:00Z', 'claude-opus-5',   200, 20, 2000, 5, 1),
                   ('s2', 'm3', '2026-01-03T10:00:00Z', 'claude-sonnet-5',  50,  5,  500, 0, 1);

            INSERT INTO session_tool_counts (session_id, tool_name, calls)
            VALUES ('s1', 'Read', 10), ('s1', 'Edit', 4), ('s2', 'Read', 3);

            INSERT INTO agents (agent_id, agent_type, completed_ts, total_tokens,
                                total_duration_ms, tool_use_count)
            VALUES ('a1', 'Explore', '2026-01-02T10:00:00Z', 500, 1000, 7),
                   ('a2', 'Explore', '2026-01-04T10:00:00Z', 700, 2000, 3),
                   ('a3', NULL,      '2026-01-05T10:00:00Z', 100,  500, 1);
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    #[ignore = "reads the developer's own database"]
    fn against_the_real_database() {
        let conn = crate::storage::open(&crate::paths::database_path()).unwrap();
        for row in tools(&conn).unwrap().iter().take(5) {
            println!(
                "tool {:20} calls={:6} sessions={}",
                row.name, row.calls, row.sessions
            );
        }
        for row in models(&conn).unwrap() {
            println!(
                "model {:28} turns={:5} in={:9} out={:8} cacheRead={}",
                row.model, row.turns, row.input_tokens, row.output_tokens, row.cache_read_tokens
            );
        }
        for row in agents(&conn).unwrap() {
            println!(
                "agent {:20} runs={:4} tokens={}",
                row.agent_type, row.runs, row.total_tokens
            );
        }
        let rhythm = crate::analysis::rhythm::load(&conn).unwrap();
        println!(
            "rhythm activeDays={} longest={} current={} busiest={:?}/{:?}",
            rhythm.active_days,
            rhythm.longest_streak,
            rhythm.current_streak,
            rhythm.busiest_weekday,
            rhythm.busiest_hour
        );
    }

    #[test]
    fn tools_are_ranked_by_calls_and_count_sessions() {
        let rows = tools(&seeded()).unwrap();
        assert_eq!(rows[0].name, "Read");
        assert_eq!(rows[0].calls, 13);
        assert_eq!(rows[0].sessions, 2, "Read was used in both sessions");
        assert_eq!(rows[1].name, "Edit");
        assert_eq!(rows[1].sessions, 1);
    }

    #[test]
    fn models_sum_their_tokens() {
        let rows = models(&seeded()).unwrap();
        assert_eq!(rows[0].model, "claude-opus-5");
        assert_eq!(rows[0].turns, 2);
        assert_eq!(rows[0].input_tokens, 300);
        assert_eq!(rows[0].cache_read_tokens, 3000);
        assert_eq!(rows[0].first_ts.as_deref(), Some("2026-01-01T10:00:00Z"));
        assert_eq!(rows[1].model, "claude-sonnet-5");
    }

    #[test]
    fn agents_group_by_type_and_keep_the_untyped_ones() {
        let rows = agents(&seeded()).unwrap();
        assert_eq!(rows[0].agent_type, "Explore");
        assert_eq!(rows[0].runs, 2);
        assert_eq!(rows[0].total_tokens, 1200);
        assert_eq!(rows[0].last_ts.as_deref(), Some("2026-01-04T10:00:00Z"));
        // A run whose type never made it into the transcript still counts.
        assert!(rows.iter().any(|row| row.agent_type == "unknown"));
    }
}
