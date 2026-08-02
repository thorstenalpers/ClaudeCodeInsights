//! The time series behind every chart: turns and tokens per day, split by one
//! dimension, with the other dimensions available as filters.
//!
//! One query serves all of them. A chart of tokens over time by model, filtered
//! to two activities and one tool, is the same shape as a chart by tool filtered
//! to one model — only the grouping column and the `WHERE` clauses differ.

use anyhow::{Result, bail};
use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesQuery {
    /// One of `model`, `activity`, `tool`, `project`, `none`.
    pub group_by: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub activities: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    /// Inclusive local dates, `YYYY-MM-DD`.
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}

/// One bucket. `model` rides along with every grouping so the frontend can put
/// a price on it — cost is derived from the counts and the rate table there,
/// never stored here.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPoint {
    pub date: String,
    pub key: String,
    pub model: String,
    pub turns: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub points: Vec<SeriesPoint>,
    /// Every distinct key present, so a legend keeps its order across reloads.
    pub keys: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesFacets {
    pub models: Vec<String>,
    pub activities: Vec<String>,
    pub tools: Vec<String>,
    pub projects: Vec<String>,
}

/// The grouping expression per dimension. An allowlist, never caller text.
fn group_expression(group_by: &str) -> Result<&'static str> {
    Ok(match group_by {
        "model" => "COALESCE(t.model, 'unknown')",
        "activity" => "COALESCE(sa.activity, 'unknown')",
        "tool" => "COALESCE(tt.tool_name, 'none')",
        "project" => "COALESCE(s.project_name, 'unknown')",
        "none" => "'all'",
        other => bail!("unknown grouping '{other}'"),
    })
}

fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn load(conn: &Connection, query: &SeriesQuery) -> Result<Series> {
    let group = group_expression(&query.group_by)?;

    // The tool join multiplies rows — a turn with three tool calls appears three
    // times. That is right when grouping or filtering by tool and wrong
    // otherwise, so it is only joined when a tool is actually involved.
    let needs_tools = query.group_by == "tool" || !query.tools.is_empty();
    let tool_join = if needs_tools {
        "LEFT JOIN turn_tools tt ON tt.turn_id = t.id"
    } else {
        ""
    };

    let mut wheres = vec!["t.ts_utc IS NOT NULL".to_owned()];
    let mut binds: Vec<SqlValue> = Vec::new();

    for (values, column) in [
        (&query.models, "COALESCE(t.model, 'unknown')"),
        (&query.activities, "COALESCE(sa.activity, 'unknown')"),
        (&query.projects, "COALESCE(s.project_name, 'unknown')"),
        (&query.tools, "tt.tool_name"),
    ] {
        if values.is_empty() {
            continue;
        }
        // Placeholders come from the number of bound values, never from text.
        wheres.push(format!("{column} IN ({})", placeholders(values.len())));
        binds.extend(values.iter().map(|v| SqlValue::Text(v.clone())));
    }

    if let Some(from) = &query.from {
        wheres.push("date(t.ts_utc, 'localtime') >= ?".to_owned());
        binds.push(SqlValue::Text(from.clone()));
    }
    if let Some(to) = &query.to {
        wheres.push("date(t.ts_utc, 'localtime') <= ?".to_owned());
        binds.push(SqlValue::Text(to.clone()));
    }

    let sql = format!(
        "SELECT date(t.ts_utc, 'localtime') AS day,
                {group} AS grouping_key,
                COALESCE(t.model, 'unknown') AS model_name,
                COUNT(DISTINCT t.id),
                COALESCE(SUM(t.input_tokens), 0),
                COALESCE(SUM(t.output_tokens), 0),
                COALESCE(SUM(t.cache_read_tokens), 0),
                COALESCE(SUM(t.cache_write_tokens), 0)
         FROM turns t
         LEFT JOIN sessions s ON s.session_id = t.session_id
         LEFT JOIN session_activity sa ON sa.session_id = t.session_id
         {tool_join}
         WHERE {}
         GROUP BY day, grouping_key, model_name
         ORDER BY day",
        wheres.join(" AND ")
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(binds), |row| {
        Ok(SeriesPoint {
            date: row.get(0)?,
            key: row.get(1)?,
            model: row.get(2)?,
            turns: row.get(3)?,
            input_tokens: row.get(4)?,
            output_tokens: row.get(5)?,
            cache_read_tokens: row.get(6)?,
            cache_write_tokens: row.get(7)?,
        })
    })?;

    let points = rows.collect::<Result<Vec<_>, _>>()?;

    let mut keys: Vec<String> = points.iter().map(|p| p.key.clone()).collect();
    keys.sort();
    keys.dedup();

    Ok(Series { points, keys })
}

/// Everything the filter bar can offer, so it never lists a value with no rows.
pub fn facets(conn: &Connection) -> Result<SeriesFacets> {
    let column = |sql: &str| -> Result<Vec<String>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    };

    Ok(SeriesFacets {
        models: column(
            "SELECT DISTINCT model FROM turns WHERE model IS NOT NULL AND model <> '' ORDER BY model",
        )?,
        activities: column("SELECT DISTINCT activity FROM session_activity ORDER BY activity")?,
        tools: column(
            "SELECT tool_name FROM session_tool_counts GROUP BY tool_name ORDER BY SUM(calls) DESC",
        )?,
        projects: column(
            "SELECT DISTINCT project_name FROM sessions WHERE project_name IS NOT NULL ORDER BY project_name",
        )?,
    })
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

            INSERT INTO sessions (session_id, project_name) VALUES ('s1', 'work/a'), ('s2', 'work/b');
            INSERT INTO session_activity (session_id, activity, profile_json)
            VALUES ('s1', 'coding', '{}'), ('s2', 'research', '{}');

            INSERT INTO turns (id, session_id, message_id, ts_utc, model, input_tokens,
                               output_tokens, cache_read_tokens, cache_write_tokens, scan_file_id)
            VALUES (1, 's1', 'm1', '2026-01-01T10:00:00Z', 'claude-opus-5',   100, 10, 1000, 5, 1),
                   (2, 's1', 'm2', '2026-01-01T12:00:00Z', 'claude-opus-5',   200, 20, 2000, 5, 1),
                   (3, 's2', 'm3', '2026-01-02T10:00:00Z', 'claude-sonnet-5',  50,  5,  500, 0, 1);

            INSERT INTO turn_tools (turn_id, seq, tool_name)
            VALUES (1, 0, 'Read'), (1, 1, 'Edit'), (3, 0, 'Read');

            INSERT INTO session_tool_counts (session_id, tool_name, calls)
            VALUES ('s1', 'Read', 1), ('s1', 'Edit', 1), ('s2', 'Read', 1);
            "#,
        )
        .unwrap();
        conn
    }

    fn query(group_by: &str) -> SeriesQuery {
        SeriesQuery {
            group_by: group_by.to_owned(),
            models: Vec::new(),
            activities: Vec::new(),
            tools: Vec::new(),
            projects: Vec::new(),
            from: None,
            to: None,
        }
    }

    #[test]
    fn groups_by_model_and_buckets_by_day() {
        let series = load(&seeded(), &query("model")).unwrap();
        assert_eq!(series.keys, ["claude-opus-5", "claude-sonnet-5"]);

        let opus = series
            .points
            .iter()
            .find(|p| p.key == "claude-opus-5")
            .unwrap();
        assert_eq!(opus.turns, 2, "both opus turns fall on the same day");
        assert_eq!(opus.input_tokens, 300);
    }

    #[test]
    fn filters_stack() {
        let mut q = query("activity");
        q.models = vec!["claude-opus-5".to_owned()];
        q.activities = vec!["coding".to_owned()];
        let series = load(&seeded(), &q).unwrap();

        assert_eq!(series.keys, ["coding"]);
        assert_eq!(series.points.iter().map(|p| p.turns).sum::<i64>(), 2);
    }

    #[test]
    fn a_turn_with_two_tools_is_not_counted_twice_per_tool() {
        let series = load(&seeded(), &query("tool")).unwrap();

        // Turn 1 called Read and Edit; each tool sees it once, and neither sees
        // it twice.
        for key in ["Read", "Edit"] {
            let turns: i64 = series
                .points
                .iter()
                .filter(|p| p.key == key)
                .map(|p| p.turns)
                .sum();
            assert!(turns >= 1, "{key} has turns");
        }
        let edit = series.points.iter().find(|p| p.key == "Edit").unwrap();
        assert_eq!(edit.turns, 1);
    }

    #[test]
    fn the_date_range_is_inclusive() {
        let mut q = query("none");
        q.from = Some("2026-01-02".to_owned());
        let series = load(&seeded(), &q).unwrap();
        assert_eq!(series.points.len(), 1);
        assert_eq!(series.points[0].date, "2026-01-02");
    }

    #[test]
    fn an_unknown_grouping_is_rejected_rather_than_interpolated() {
        let mut q = query("model");
        q.group_by = "t.model; DROP TABLE turns".to_owned();
        assert!(load(&seeded(), &q).is_err());
    }

    #[test]
    fn facets_only_offer_values_that_exist() {
        let facets = facets(&seeded()).unwrap();
        assert_eq!(facets.models, ["claude-opus-5", "claude-sonnet-5"]);
        assert_eq!(facets.activities, ["coding", "research"]);
        assert!(facets.tools.contains(&"Read".to_owned()));
        assert_eq!(facets.projects, ["work/a", "work/b"]);
    }
}
