//! Token counts split by model, for the roll-ups whose rows span several models.
//!
//! Cost is worked out in the window, from these counts and the price table. A
//! row that mixes Opus and Haiku therefore has to carry the mix: one blended
//! total could not be priced without guessing which model earned it.

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTokens {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
}

/// Runs a query shaped `key, model, in, out, cacheRead, cacheWrite`.
fn split(conn: &Connection, sql: &str) -> Result<HashMap<String, Vec<ModelTokens>>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            ModelTokens {
                model: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                cache_read_tokens: row.get(4)?,
                cache_write_tokens: row.get(5)?,
            },
        ))
    })?;

    let mut map: HashMap<String, Vec<ModelTokens>> = HashMap::new();
    for row in rows {
        let (key, tokens) = row?;
        map.entry(key).or_default().push(tokens);
    }
    Ok(map)
}

/// Keyed by `sessions.project_path`, still in its raw spelling — the caller
/// folds the spellings together, because only it knows the rule.
pub fn by_project(conn: &Connection) -> Result<HashMap<String, Vec<ModelTokens>>> {
    split(
        conn,
        "SELECT s.project_path,
                COALESCE(t.model, ''),
                COALESCE(SUM(t.input_tokens), 0),
                COALESCE(SUM(t.output_tokens), 0),
                COALESCE(SUM(t.cache_read_tokens), 0),
                COALESCE(SUM(t.cache_write_tokens), 0)
         FROM sessions s
         JOIN turns t ON t.session_id = s.session_id
         WHERE s.project_path IS NOT NULL
         GROUP BY s.project_path, t.model",
    )
}

/// Keyed by the activity a session was classified as.
pub fn by_activity(conn: &Connection) -> Result<HashMap<String, Vec<ModelTokens>>> {
    split(
        conn,
        "SELECT COALESCE(sa.activity, 'unknown'),
                COALESCE(t.model, ''),
                COALESCE(SUM(t.input_tokens), 0),
                COALESCE(SUM(t.output_tokens), 0),
                COALESCE(SUM(t.cache_read_tokens), 0),
                COALESCE(SUM(t.cache_write_tokens), 0)
         FROM sessions s
         JOIN turns t ON t.session_id = s.session_id
         LEFT JOIN session_activity sa ON sa.session_id = s.session_id
         GROUP BY COALESCE(sa.activity, 'unknown'), t.model",
    )
}

pub fn by_agent_type(conn: &Connection) -> Result<HashMap<String, Vec<ModelTokens>>> {
    split(
        conn,
        "SELECT COALESCE(a.agent_type, 'unknown'),
                COALESCE(t.model, ''),
                COALESCE(SUM(t.input_tokens), 0),
                COALESCE(SUM(t.output_tokens), 0),
                COALESCE(SUM(t.cache_read_tokens), 0),
                COALESCE(SUM(t.cache_write_tokens), 0)
         FROM agents a
         JOIN turns t ON t.agent_id = a.agent_id
         GROUP BY COALESCE(a.agent_type, 'unknown'), t.model",
    )
}

/// A turn's tokens shared out over the tool calls it made.
///
/// A turn that calls three tools cannot be charged to each of them whole, so
/// every call takes an equal share. The split is an attribution rule, not a
/// measurement — but it is one the totals survive: summed over all tools it
/// comes back to the tokens of the turns that used tools, with nothing counted
/// twice.
pub fn by_tool(conn: &Connection) -> Result<HashMap<String, Vec<ModelTokens>>> {
    split(
        conn,
        "SELECT tt.tool_name,
                COALESCE(t.model, ''),
                CAST(COALESCE(SUM(t.input_tokens * 1.0 / n.calls), 0) AS INTEGER),
                CAST(COALESCE(SUM(t.output_tokens * 1.0 / n.calls), 0) AS INTEGER),
                CAST(COALESCE(SUM(t.cache_read_tokens * 1.0 / n.calls), 0) AS INTEGER),
                CAST(COALESCE(SUM(t.cache_write_tokens * 1.0 / n.calls), 0) AS INTEGER)
         FROM turn_tools tt
         JOIN turns t ON t.id = tt.turn_id
         JOIN (SELECT turn_id, COUNT(*) AS calls FROM turn_tools GROUP BY turn_id) n
              ON n.turn_id = tt.turn_id
         GROUP BY tt.tool_name, t.model",
    )
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

            INSERT INTO sessions (session_id, project_path) VALUES ('s1', 'C:\Work'), ('s2', 'C:\Work');

            INSERT INTO turns (id, session_id, message_id, ts_utc, model, input_tokens,
                               output_tokens, cache_read_tokens, cache_write_tokens,
                               agent_id, scan_file_id)
            VALUES (1, 's1', 'm1', '2026-01-01T10:00:00Z', 'claude-opus-5',   100, 10, 1000, 5, NULL, 1),
                   (2, 's1', 'm2', '2026-01-02T10:00:00Z', 'claude-sonnet-5', 200, 20, 2000, 5, 'a1', 1),
                   (3, 's2', 'm3', '2026-01-03T10:00:00Z', 'claude-opus-5',    60,  6,  600, 0, NULL, 1);

            INSERT INTO turn_tools (turn_id, seq, tool_name)
            VALUES (1, 0, 'Read'), (1, 1, 'Edit'), (3, 0, 'Read');

            INSERT INTO agents (agent_id, agent_type, total_tokens, total_duration_ms, tool_use_count)
            VALUES ('a1', 'Explore', 2225, 1000, 3);
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn a_project_keeps_its_models_apart() {
        let map = by_project(&seeded()).unwrap();
        let mut models = map[r"C:\Work"].clone();
        models.sort_by(|a, b| a.model.cmp(&b.model));
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model, "claude-opus-5");
        assert_eq!(models[0].input_tokens, 160);
        assert_eq!(models[1].model, "claude-sonnet-5");
        assert_eq!(models[1].input_tokens, 200);
    }

    #[test]
    fn an_agent_is_priced_from_its_own_turns() {
        let map = by_agent_type(&seeded()).unwrap();
        let explore = &map["Explore"][0];
        assert_eq!(explore.model, "claude-sonnet-5");
        assert_eq!(explore.input_tokens, 200);
        assert_eq!(explore.output_tokens, 20);
    }

    #[test]
    fn a_turn_with_two_tools_is_halved_and_nothing_is_counted_twice() {
        let map = by_tool(&seeded()).unwrap();

        let read: i64 = map["Read"].iter().map(|entry| entry.input_tokens).sum();
        let edit: i64 = map["Edit"].iter().map(|entry| entry.input_tokens).sum();
        assert_eq!(edit, 50, "half of the 100 that turn 1 spent");
        assert_eq!(read, 50 + 60, "its half of turn 1, plus all of turn 3");
        assert_eq!(read + edit, 160, "the two turns that used tools, once each");
    }

    #[test]
    #[ignore = "reads the developer's own database"]
    fn against_the_real_database() {
        let conn = crate::storage::open(&crate::paths::database_path()).unwrap();

        let agents = by_agent_type(&conn).unwrap();
        println!("agent types with turns: {}", agents.len());
        for (agent_type, models) in agents.iter().take(5) {
            let input: i64 = models.iter().map(|entry| entry.input_tokens).sum();
            println!("  {agent_type:20} models={:2} input={input}", models.len());
        }

        let tools = by_tool(&conn).unwrap();
        println!("tools with attributed tokens: {}", tools.len());
        for (tool, models) in tools.iter().take(5) {
            let input: i64 = models.iter().map(|entry| entry.input_tokens).sum();
            println!("  {tool:20} models={:2} input={input}", models.len());
        }

        let projects = by_project(&conn).unwrap();
        println!("projects with a model split: {}", projects.len());
    }
}
