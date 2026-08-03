//! The three roll-ups the Tools, Cost and Agents pages ask for.
//!
//! Each is a single grouped query. Nothing is materialised: the tables they
//! read are already the materialised form, and a second cache would be one more
//! thing to invalidate after a scan.

use crate::analysis::cost::{self, ModelTokens};
use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

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

/// One derived activity, with everything the sessions behind it add up to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityRow {
    pub activity: String,
    pub sessions: i64,
    pub turns: i64,
    /// Distinct projects the activity was seen in.
    pub projects: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub by_model: Vec<ModelTokens>,
}

/// A name and how many runs fell under it, for the lists an agent row carries.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    pub name: String,
    pub runs: i64,
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
    /// The sessions that started runs of this type.
    pub sessions: i64,
    /// Which projects it ran in, and which activities those sessions were,
    /// both busiest first. Reached through `agents.parent_session_id`.
    pub projects: Vec<Tally>,
    pub activities: Vec<Tally>,
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

/// Sessions rolled up by the activity they were classified as.
///
/// A session with no row in `session_activity` is counted as `unknown` rather
/// than dropped: the totals here are meant to add up to the whole history.
///
/// `turn_count > 0` is the same bar the sessions list and the overview use. A
/// transcript whose turns were all dropped as empty leaves a session row behind
/// with nothing in it; counting those here said forty conversations where the
/// sessions page showed none.
pub fn activities(conn: &Connection) -> Result<Vec<ActivityRow>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(sa.activity, 'unknown'),
                COUNT(*),
                COALESCE(SUM(s.turn_count), 0),
                COUNT(DISTINCT s.project_path),
                COALESCE(SUM(u.input_tokens), 0),
                COALESCE(SUM(u.output_tokens), 0),
                COALESCE(SUM(u.cache_read_tokens), 0),
                COALESCE(SUM(u.cache_write_tokens), 0),
                MIN(s.first_ts),
                MAX(s.last_ts)
         FROM sessions s
         LEFT JOIN session_activity sa ON sa.session_id = s.session_id
         LEFT JOIN (SELECT session_id,
                           SUM(input_tokens) AS input_tokens,
                           SUM(output_tokens) AS output_tokens,
                           SUM(cache_read_tokens) AS cache_read_tokens,
                           SUM(cache_write_tokens) AS cache_write_tokens
                    FROM turns
                    GROUP BY session_id) u ON u.session_id = s.session_id
         WHERE s.turn_count > 0
         GROUP BY COALESCE(sa.activity, 'unknown')
         ORDER BY COUNT(*) DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ActivityRow {
            activity: row.get(0)?,
            sessions: row.get(1)?,
            turns: row.get(2)?,
            projects: row.get(3)?,
            input_tokens: row.get(4)?,
            output_tokens: row.get(5)?,
            cache_read_tokens: row.get(6)?,
            cache_write_tokens: row.get(7)?,
            first_ts: row.get(8)?,
            last_ts: row.get(9)?,
            by_model: Vec::new(),
        })
    })?;

    let mut rows = rows.collect::<Result<Vec<_>, _>>()?;
    let mut split = cost::by_activity(conn)?;
    for row in &mut rows {
        row.by_model = split.remove(&row.activity).unwrap_or_default();
    }
    Ok(rows)
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
            sessions: 0,
            projects: Vec::new(),
            activities: Vec::new(),
            by_model: Vec::new(),
        })
    })?;

    let mut rows = rows.collect::<Result<Vec<_>, _>>()?;
    let mut split = cost::by_agent_type(conn)?;
    let mut context = agent_context(conn)?;
    for row in &mut rows {
        row.by_model = split.remove(&row.agent_type).unwrap_or_default();
        if let Some(seen) = context.remove(&row.agent_type) {
            row.sessions = seen.sessions.len() as i64;
            row.projects = ranked(seen.projects);
            row.activities = ranked(seen.activities);
        }
    }
    Ok(rows)
}

#[derive(Default)]
struct AgentContext {
    sessions: HashSet<String>,
    projects: HashMap<String, i64>,
    activities: HashMap<String, i64>,
}

fn ranked(counts: HashMap<String, i64>) -> Vec<Tally> {
    let mut tallies: Vec<Tally> = counts
        .into_iter()
        .map(|(name, runs)| Tally { name, runs })
        .collect();
    // Busiest first, then by name so the order does not wobble between calls.
    tallies.sort_by(|a, b| b.runs.cmp(&a.runs).then_with(|| a.name.cmp(&b.name)));
    tallies
}

/// Where each agent type ran: the session it was started from, that session's
/// project, and what the session was doing.
///
/// A run whose parent session was never scanned keeps its type but contributes
/// no project — the row still counts it, which is why the lists can add up to
/// less than `runs`.
fn agent_context(conn: &Connection) -> Result<HashMap<String, AgentContext>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(a.agent_type, 'unknown'),
                a.parent_session_id,
                COALESCE(NULLIF(s.project_name, ''), s.project_path),
                sa.activity
         FROM agents a
         LEFT JOIN sessions s ON s.session_id = a.parent_session_id
         LEFT JOIN session_activity sa ON sa.session_id = a.parent_session_id",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    let mut map: HashMap<String, AgentContext> = HashMap::new();
    for row in rows {
        let (agent_type, session, project, activity) = row?;
        let entry = map.entry(agent_type).or_default();
        if let Some(session) = session {
            entry.sessions.insert(session);
        }
        if let Some(project) = project {
            *entry.projects.entry(project).or_insert(0) += 1;
        }
        if let Some(activity) = activity {
            *entry.activities.entry(activity).or_insert(0) += 1;
        }
    }
    Ok(map)
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
    fn activities_count_every_session_even_the_unclassified() {
        let conn = seeded();
        conn.execute_batch(
            r#"
            INSERT INTO sessions (session_id, project_path, turn_count, first_ts, last_ts)
            VALUES ('s1', 'C:/a', 2, '2026-01-01T10:00:00Z', '2026-01-02T10:00:00Z'),
                   ('s2', 'C:/b', 1, '2026-01-03T10:00:00Z', '2026-01-03T10:00:00Z'),
                   ('s3', 'C:/c', 0, NULL, NULL);

            INSERT INTO session_activity (session_id, activity, profile_json)
            VALUES ('s1', 'coding', '{}'), ('s3', 'conversation', '{}');
            "#,
        )
        .unwrap();

        let rows = activities(&conn).unwrap();
        assert!(
            rows.iter().all(|row| row.activity != "conversation"),
            "a session whose turns were all dropped is not one the sessions page shows either"
        );
        let coding = rows.iter().find(|row| row.activity == "coding").unwrap();
        assert_eq!((coding.sessions, coding.turns, coding.projects), (1, 2, 1));
        assert_eq!(coding.input_tokens, 300, "both turns of the session count");
        assert_eq!(coding.by_model.len(), 1, "priced per model, not blended");

        let unknown = rows.iter().find(|row| row.activity == "unknown").unwrap();
        assert_eq!(
            unknown.sessions, 1,
            "a session the classifier never reached is still part of the history"
        );
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
        for row in activities(&conn).unwrap() {
            println!(
                "activity {:14} sessions={:5} turns={:6} projects={:3} in={:10} last={:?}",
                row.activity, row.sessions, row.turns, row.projects, row.input_tokens, row.last_ts
            );
        }
        for row in agents(&conn).unwrap() {
            println!(
                "agent {:20} runs={:4} tokens={:9} sessions={:3} projects={:?} activities={:?}",
                row.agent_type,
                row.runs,
                row.total_tokens,
                row.sessions,
                row.projects
                    .iter()
                    .map(|entry| (entry.name.as_str(), entry.runs))
                    .collect::<Vec<_>>(),
                row.activities
                    .iter()
                    .map(|entry| (entry.name.as_str(), entry.runs))
                    .collect::<Vec<_>>()
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

    #[test]
    fn agents_carry_the_projects_and_activities_they_ran_in() {
        let conn = seeded();
        conn.execute_batch(
            r#"
            INSERT INTO sessions (session_id, project_name, project_path, turn_count)
            VALUES ('s1', 'ClaudeAdmin', 'C:/a', 2),
                   ('s2', 'Other',       'C:/b', 1);

            INSERT INTO session_activity (session_id, activity, profile_json)
            VALUES ('s1', 'coding', '{}'), ('s2', 'research', '{}');

            UPDATE agents SET parent_session_id = 's1' WHERE agent_id IN ('a1', 'a3');
            UPDATE agents SET parent_session_id = 's2' WHERE agent_id = 'a2';
            "#,
        )
        .unwrap();

        let rows = agents(&conn).unwrap();
        let explore = rows.iter().find(|row| row.agent_type == "Explore").unwrap();
        assert_eq!(explore.sessions, 2);
        assert_eq!(
            explore
                .projects
                .iter()
                .map(|entry| (entry.name.as_str(), entry.runs))
                .collect::<Vec<_>>(),
            [("ClaudeAdmin", 1), ("Other", 1)],
            "the project comes from the session that started the run"
        );
        assert_eq!(
            explore
                .activities
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["coding", "research"]
        );
    }
}
