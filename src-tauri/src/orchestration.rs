//! What a project is meant to get done, and what is available to do it with.
//!
//! The tasks are the app's own list — features, stories, whatever the user
//! writes down — and the only table here that is not derived from transcripts.
//! Skills and agent definitions are read from disk where Claude Code keeps
//! them, never copied and never written to.

use crate::error::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// One thing to be done in a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default)]
    pub id: i64,
    pub project_path: String,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    /// `open`, `running`, `waiting`, `deferred` or `done`.
    #[serde(default = "open_state")]
    pub state: String,
    #[serde(default)]
    pub position: i64,
    #[serde(default)]
    pub created_ts: String,
    /// The session that worked on it, once one has.
    #[serde(default)]
    pub session_id: Option<String>,
}

fn open_state() -> String {
    "open".to_owned()
}

pub fn list_tasks(conn: &Connection, project: Option<&str>) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_path, title, notes, state, position, created_ts, session_id
         FROM tasks
         WHERE ?1 IS NULL OR project_path = ?1
         ORDER BY project_path, position, id",
    )?;

    let rows = stmt.query_map([project], |row| {
        Ok(Task {
            id: row.get(0)?,
            project_path: row.get(1)?,
            title: row.get(2)?,
            notes: row.get(3)?,
            state: row.get(4)?,
            position: row.get(5)?,
            created_ts: row.get(6)?,
            session_id: row.get(7)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}

/// Adds a task to the end of a project's list.
pub fn add_task(conn: &Connection, project: &str, title: &str) -> Result<Task> {
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM tasks WHERE project_path = ?1",
        [project],
        |row| row.get(0),
    )?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO tasks (project_path, title, position, created_ts) VALUES (?1, ?2, ?3, ?4)",
        params![project, title.trim(), next, now],
    )?;

    Ok(Task {
        id: conn.last_insert_rowid(),
        project_path: project.to_owned(),
        title: title.trim().to_owned(),
        notes: String::new(),
        state: open_state(),
        position: next,
        created_ts: now,
        session_id: None,
    })
}

/// Writes back whatever the window changed about a task.
pub fn update_task(conn: &Connection, task: &Task) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET title = ?2, notes = ?3, state = ?4, position = ?5, session_id = ?6
         WHERE id = ?1",
        params![
            task.id,
            task.title,
            task.notes,
            task.state,
            task.position,
            task.session_id
        ],
    )?;
    Ok(())
}

pub fn remove_task(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
    Ok(())
}

/// A skill or an agent definition, as it lies on disk.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub name: String,
    /// The one-line description from the front matter, where there is one.
    pub description: String,
    pub path: String,
    /// `user` for the ones in the home folder, otherwise the project they
    /// belong to.
    pub scope: String,
}

/// Reads `name` and `description` out of a markdown front matter block.
fn front_matter(text: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut description = None;
    let mut inside = false;

    for line in text.lines() {
        if line.trim() == "---" {
            if inside {
                break;
            }
            inside = true;
            continue;
        }
        if !inside {
            break;
        }
        if let Some(rest) = line.strip_prefix("name:") {
            name = Some(rest.trim().trim_matches('"').to_owned());
        } else if let Some(rest) = line.strip_prefix("description:") {
            description = Some(rest.trim().trim_matches('"').to_owned());
        }
    }
    (name, description)
}

fn read_definitions(dir: &Path, file: &str, scope: &str, into: &mut Vec<Definition>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // A skill is a folder with a SKILL.md; an agent is a bare markdown file.
        let doc = if path.is_dir() {
            path.join(file)
        } else if path.extension().is_some_and(|ext| ext == "md") {
            path.clone()
        } else {
            continue;
        };

        let Ok(text) = fs::read_to_string(&doc) else {
            continue;
        };
        let (name, description) = front_matter(&text);
        let fallback = doc
            .parent()
            .filter(|_| path.is_dir())
            .and_then(|parent| parent.file_name())
            .or_else(|| path.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();

        into.push(Definition {
            name: name.unwrap_or(fallback),
            description: description.unwrap_or_default(),
            path: doc.to_string_lossy().into_owned(),
            scope: scope.to_owned(),
        });
    }
}

fn home() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude"))
}

/// The skills Claude Code would find: the user's own, plus a project's.
pub fn skills(project: Option<&str>) -> Vec<Definition> {
    let mut found = Vec::new();
    if let Some(root) = home() {
        read_definitions(&root.join("skills"), "SKILL.md", "user", &mut found);
    }
    if let Some(project) = project {
        let dir = Path::new(project).join(".claude").join("skills");
        read_definitions(&dir, "SKILL.md", project, &mut found);
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// The agent definitions, read the same way.
pub fn agents(project: Option<&str>) -> Vec<Definition> {
    let mut found = Vec::new();
    if let Some(root) = home() {
        read_definitions(&root.join("agents"), "AGENT.md", "user", &mut found);
    }
    if let Some(project) = project {
        let dir = Path::new(project).join(".claude").join("agents");
        read_definitions(&dir, "AGENT.md", project, &mut found);
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
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
    fn tasks_keep_the_order_they_were_added_in() {
        let conn = db();
        add_task(&conn, "C:/a", "First").unwrap();
        add_task(&conn, "C:/a", "Second").unwrap();
        add_task(&conn, "C:/b", "Elsewhere").unwrap();

        let mine = list_tasks(&conn, Some("C:/a")).unwrap();
        assert_eq!(
            mine.iter()
                .map(|task| task.title.as_str())
                .collect::<Vec<_>>(),
            ["First", "Second"]
        );
        assert_eq!(list_tasks(&conn, None).unwrap().len(), 3, "all projects");
    }

    #[test]
    fn a_task_remembers_the_session_that_took_it() {
        let conn = db();
        let mut task = add_task(&conn, "C:/a", "Write the parser").unwrap();
        task.state = "running".to_owned();
        task.session_id = Some("s1".to_owned());
        update_task(&conn, &task).unwrap();

        let stored = &list_tasks(&conn, Some("C:/a")).unwrap()[0];
        assert_eq!(stored.state, "running");
        assert_eq!(stored.session_id.as_deref(), Some("s1"));

        remove_task(&conn, task.id).unwrap();
        assert!(list_tasks(&conn, Some("C:/a")).unwrap().is_empty());
    }

    #[test]
    fn front_matter_is_read_where_there_is_one() {
        let (name, description) = front_matter("---\nname: probe\ndescription: A test\n---\nBody");
        assert_eq!(name.as_deref(), Some("probe"));
        assert_eq!(description.as_deref(), Some("A test"));

        let (bare, _) = front_matter("# Just a heading\n");
        assert!(bare.is_none(), "a file without front matter keeps its name");
    }
}
