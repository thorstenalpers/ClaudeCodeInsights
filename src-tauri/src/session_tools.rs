//! What a session the app starts may ask the app itself.
//!
//! These tools live in this process — no second binary, nothing registered in
//! the user's config, nothing left behind when the run ends. A session gets two
//! of them, both scoped to the project it was started in and both reads: what
//! the transcripts say that project has cost, and what its task list still has
//! on it.

use crate::orchestration;
use crate::paths;
use crate::storage;
use claude_agent_sdk_rs::{
    McpServerConfig, McpToolResultContent, SdkMcpTool, ToolHandler, ToolResult,
    create_sdk_mcp_server,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// The name the session knows the server by; its tools arrive as
/// `mcp__insights__<tool>`.
pub const SERVER: &str = "insights";

/// Every tool here, as the permission callback sees it named. Both only read,
/// and read the app's own database at that, which is what lets the `readsFree`
/// rule pass them without asking.
pub const READ_ONLY: [&str; 2] = [
    "mcp__insights__project_usage",
    "mcp__insights__project_tasks",
];

/// The figures the scan has for one project.
fn project_usage(conn: &Connection, project: &str) -> anyhow::Result<Value> {
    // Two spellings of one directory are one project, the same way the projects
    // page treats them.
    let same_path = "rtrim(replace(lower(s.project_path), '\\', '/'), '/')
                     = rtrim(replace(lower(?1), '\\', '/'), '/')";

    let usage = conn.query_row(
        &format!(
            "SELECT COUNT(DISTINCT s.session_id),
                    COUNT(t.id),
                    COALESCE(SUM(t.input_tokens), 0),
                    COALESCE(SUM(t.output_tokens), 0),
                    COALESCE(SUM(t.cache_read_tokens), 0),
                    COALESCE(SUM(t.cache_write_tokens), 0),
                    MAX(t.ts_utc)
             FROM sessions s
             JOIN turns t ON t.session_id = s.session_id
             WHERE {same_path}"
        ),
        [project],
        |row| {
            Ok(json!({
                "project": project,
                "sessions": row.get::<_, i64>(0)?,
                "turns": row.get::<_, i64>(1)?,
                "inputTokens": row.get::<_, i64>(2)?,
                "outputTokens": row.get::<_, i64>(3)?,
                "cacheReadTokens": row.get::<_, i64>(4)?,
                "cacheWriteTokens": row.get::<_, i64>(5)?,
                "lastActivity": row.get::<_, Option<String>>(6)?,
            }))
        },
    )?;
    Ok(usage)
}

fn project_tasks(conn: &Connection, project: &str) -> anyhow::Result<Value> {
    Ok(serde_json::to_value(orchestration::list_tasks(
        conn,
        Some(project),
    )?)?)
}

type Answer = Pin<Box<dyn Future<Output = claude_agent_sdk_rs::Result<ToolResult>> + Send>>;

struct Query {
    project: String,
    read: fn(&Connection, &str) -> anyhow::Result<Value>,
}

impl ToolHandler for Query {
    fn handle(&self, _arguments: Value) -> Answer {
        let project = self.project.clone();
        let read = self.read;
        Box::pin(async move {
            // rusqlite blocks, and the runtime this lands on is the one carrying
            // the session's messages.
            let outcome = tokio::task::spawn_blocking(move || {
                read(&storage::open(&paths::database_path())?, &project)
            })
            .await;

            // A failure is told to the session rather than raised: the model can
            // work on without the figures, but not without an answer.
            Ok(match outcome {
                Ok(Ok(value)) => text(value.to_string(), false),
                Ok(Err(error)) => text(format!("{error:#}"), true),
                Err(error) => text(error.to_string(), true),
            })
        })
    }
}

fn text(text: String, is_error: bool) -> ToolResult {
    ToolResult {
        content: vec![McpToolResultContent::Text { text }],
        is_error,
    }
}

fn tool(
    name: &str,
    description: &str,
    project: &str,
    read: fn(&Connection, &str) -> anyhow::Result<Value>,
) -> SdkMcpTool {
    SdkMcpTool {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema: json!({ "type": "object", "properties": {} }),
        handler: Arc::new(Query {
            project: project.to_owned(),
            read,
        }),
    }
}

fn tools(project: &str) -> Vec<SdkMcpTool> {
    vec![
        tool(
            "project_usage",
            "Sessions, turns and token totals recorded for this project, \
             with the time of its last recorded activity.",
            project,
            project_usage,
        ),
        tool(
            "project_tasks",
            "The task list for this project: title, notes and state of each.",
            project,
            project_tasks,
        ),
    ]
}

/// The server one run is given, bound to the project that run works in.
pub fn server(project: &str) -> McpServerConfig {
    McpServerConfig::Sdk(create_sdk_mcp_server(
        SERVER,
        env!("CARGO_PKG_VERSION"),
        tools(project),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn usage_is_zero_for_a_project_nothing_was_recorded_for() {
        let usage = project_usage(&db(), "C:/nowhere").unwrap();
        assert_eq!(usage["sessions"], 0);
        assert_eq!(usage["turns"], 0);
        assert!(usage["lastActivity"].is_null());
    }

    #[test]
    fn usage_counts_both_spellings_of_the_same_directory() {
        let conn = db();
        for (session, project) in [("s1", "C:\\Sources\\App"), ("s2", "c:/sources/app/")] {
            conn.execute(
                "INSERT INTO sessions (session_id, project_path) VALUES (?1, ?2)",
                (session, project),
            )
            .unwrap();
            conn.execute(
                "INSERT INTO turns (session_id, ts_utc, input_tokens, output_tokens, scan_file_id)
                 VALUES (?1, '2026-01-01T00:00:00Z', 10, 5, 1)",
                [session],
            )
            .unwrap();
        }

        let usage = project_usage(&conn, "C:/Sources/App").unwrap();
        assert_eq!(usage["sessions"], 2);
        assert_eq!(usage["turns"], 2);
        assert_eq!(usage["inputTokens"], 20);
        assert_eq!(usage["lastActivity"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn tasks_come_back_for_the_project_that_was_asked_for() {
        let conn = db();
        orchestration::add_task(&conn, "C:/a", "Write the parser").unwrap();
        orchestration::add_task(&conn, "C:/b", "Elsewhere").unwrap();

        let tasks = project_tasks(&conn, "C:/a").unwrap();
        assert_eq!(tasks.as_array().unwrap().len(), 1);
        assert_eq!(tasks[0]["title"], "Write the parser");
        assert_eq!(tasks[0]["state"], "open");
    }

    #[test]
    fn every_tool_the_server_offers_is_named_as_read_only() {
        // The list the permission gate consults has to keep up with the server:
        // a tool missing from it would quietly start asking, and a tool left on
        // it after being removed would quietly stop.
        let offered: Vec<String> = tools("C:/a")
            .iter()
            .map(|tool| format!("mcp__{SERVER}__{}", tool.name))
            .collect();
        assert_eq!(offered, READ_ONLY);
    }
}
