//! Projects as Claude Code sees them: the registrations in `~/.claude.json`
//! joined with the transcript folders and the figures already in the database.

use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRow {
    pub path: String,
    pub registered: bool,
    pub dir_exists: bool,
    /// Set when several registrations point at the same directory, e.g. the
    /// same path once with `\` and once with `/`. All twins share the value.
    pub duplicate_group: Option<String>,
    pub transcript_dir: Option<String>,
    pub transcript_files: i64,
    pub transcript_bytes: i64,
    pub sessions: i64,
    pub turns: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub last_ts: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsReport {
    pub config_path: String,
    pub config_exists: bool,
    pub projects: Vec<ProjectRow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptFile {
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOutcome {
    pub files_deleted: usize,
    pub bytes_freed: i64,
    pub failed: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteOutcome {
    pub backup_path: String,
}

/// How Claude Code names a project's transcript folder: every character that is
/// not ASCII alphanumeric becomes `-`.
fn encode_project_dir(path: &str) -> String {
    path.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Key under which two registrations count as the same directory.
fn normalize(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let trimmed = unified.trim_end_matches('/');
    if cfg!(windows) {
        trimmed.to_lowercase()
    } else {
        trimmed.to_owned()
    }
}

#[derive(Default, Clone)]
struct DbStats {
    sessions: i64,
    turns: i64,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    last_ts: Option<String>,
}

fn db_stats_by_project(conn: &Connection) -> Result<HashMap<String, DbStats>> {
    let mut stmt = conn.prepare(
        "SELECT s.project_path,
                COUNT(DISTINCT s.session_id),
                COUNT(t.id),
                COALESCE(SUM(t.input_tokens), 0),
                COALESCE(SUM(t.output_tokens), 0),
                COALESCE(SUM(t.cache_read_tokens), 0),
                COALESCE(SUM(t.cache_write_tokens), 0),
                MAX(t.ts_utc)
         FROM sessions s
         JOIN turns t ON t.session_id = s.session_id
         WHERE s.project_path IS NOT NULL
         GROUP BY s.project_path",
    )?;

    let mut map: HashMap<String, DbStats> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            DbStats {
                sessions: row.get(1)?,
                turns: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                cache_read_tokens: row.get(5)?,
                cache_write_tokens: row.get(6)?,
                last_ts: row.get(7)?,
            },
        ))
    })?;

    for row in rows {
        let (path, stats) = row?;
        let entry = map.entry(normalize(&path)).or_default();
        // Two cwd spellings of one directory land in the same bucket.
        entry.sessions += stats.sessions;
        entry.turns += stats.turns;
        entry.input_tokens += stats.input_tokens;
        entry.output_tokens += stats.output_tokens;
        entry.cache_read_tokens += stats.cache_read_tokens;
        entry.cache_write_tokens += stats.cache_write_tokens;
        if stats.last_ts > entry.last_ts {
            entry.last_ts = stats.last_ts;
        }
    }
    Ok(map)
}

/// A transcript folder name mapped back to the real path, via any session that
/// was scanned out of it. The encoding is lossy, so this is the only way back.
fn project_path_by_dir(conn: &Connection) -> Result<HashMap<String, String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT transcript_path, project_path
         FROM sessions
         WHERE transcript_path IS NOT NULL AND project_path IS NOT NULL",
    )?;

    let mut map = HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (transcript, project) = row?;
        if let Some(parent) = Path::new(&transcript).parent() {
            map.entry(parent.to_string_lossy().into_owned())
                .or_insert(project);
        }
    }
    Ok(map)
}

fn jsonl_files(dir: &Path) -> Vec<TranscriptFile> {
    let mut files: Vec<TranscriptFile> = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
        })
        .map(|e| TranscriptFile {
            path: e.path().to_string_lossy().into_owned(),
            name: e
                .path()
                .strip_prefix(dir)
                .unwrap_or(e.path())
                .to_string_lossy()
                .into_owned(),
            size_bytes: e.metadata().map(|m| m.len() as i64).unwrap_or(0),
        })
        .collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    files
}

fn transcript_dir_for(path: &str) -> Option<PathBuf> {
    let encoded = encode_project_dir(path);
    crate::paths::default_scan_roots()
        .iter()
        .map(|root| root.join(&encoded))
        .find(|dir| dir.is_dir())
}

pub fn list(conn: &Connection) -> Result<ProjectsReport> {
    let config_path = crate::paths::claude_config_path()?;
    let config_exists = config_path.is_file();

    let registrations: Vec<String> = if config_exists {
        let value = read_config(&config_path)?;
        value
            .get("projects")
            .and_then(Value::as_object)
            .map(|projects| projects.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let stats = db_stats_by_project(conn)?;
    let dir_to_project = project_path_by_dir(conn)?;

    let mut normalized_counts: HashMap<String, usize> = HashMap::new();
    for path in &registrations {
        *normalized_counts.entry(normalize(path)).or_default() += 1;
    }

    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
    let mut projects = Vec::new();

    for path in &registrations {
        let key = normalize(path);
        let dir = transcript_dir_for(path);
        if let Some(d) = &dir {
            seen_dirs.insert(d.clone());
        }
        let files = dir.as_deref().map(jsonl_files).unwrap_or_default();
        let db = stats.get(&key).cloned().unwrap_or_default();

        projects.push(ProjectRow {
            path: path.clone(),
            registered: true,
            dir_exists: Path::new(path).is_dir(),
            duplicate_group: (normalized_counts[&key] > 1).then(|| key.clone()),
            transcript_dir: dir.map(|d| d.to_string_lossy().into_owned()),
            transcript_files: files.len() as i64,
            transcript_bytes: files.iter().map(|f| f.size_bytes).sum(),
            sessions: db.sessions,
            turns: db.turns,
            input_tokens: db.input_tokens,
            output_tokens: db.output_tokens,
            cache_read_tokens: db.cache_read_tokens,
            cache_write_tokens: db.cache_write_tokens,
            last_ts: db.last_ts,
        });
    }

    // Transcript folders no registration points at: usually projects whose
    // registration was removed, but whose history is still on disk.
    for root in crate::paths::default_scan_roots() {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let dir = entry.path();
            if !dir.is_dir() || seen_dirs.contains(&dir) {
                continue;
            }
            let dir_str = dir.to_string_lossy().into_owned();
            let display_path = dir_to_project
                .get(&dir_str)
                .cloned()
                .unwrap_or_else(|| dir_str.clone());
            let files = jsonl_files(&dir);
            if files.is_empty() {
                continue;
            }
            let db = stats
                .get(&normalize(&display_path))
                .cloned()
                .unwrap_or_default();

            projects.push(ProjectRow {
                dir_exists: Path::new(&display_path).is_dir(),
                path: display_path,
                registered: false,
                duplicate_group: None,
                transcript_dir: Some(dir_str),
                transcript_files: files.len() as i64,
                transcript_bytes: files.iter().map(|f| f.size_bytes).sum(),
                sessions: db.sessions,
                turns: db.turns,
                input_tokens: db.input_tokens,
                output_tokens: db.output_tokens,
                cache_read_tokens: db.cache_read_tokens,
                cache_write_tokens: db.cache_write_tokens,
                last_ts: db.last_ts,
            });
        }
    }

    projects.sort_by(|a, b| b.last_ts.cmp(&a.last_ts).then(a.path.cmp(&b.path)));

    Ok(ProjectsReport {
        config_path: config_path.to_string_lossy().into_owned(),
        config_exists,
        projects,
    })
}

pub fn transcript_files(path: &str) -> Result<Vec<TranscriptFile>> {
    Ok(transcript_dir_for(path)
        .as_deref()
        .map(jsonl_files)
        .unwrap_or_default())
}

/// Deletes a project's transcript files and drops what was scanned out of them.
///
/// The exact list must have been shown to the user beforehand; this function
/// deletes whatever is in the folder now, and reports anything it could not.
pub fn delete_transcripts(conn: &mut Connection, path: &str) -> Result<DeleteOutcome> {
    let Some(dir) = transcript_dir_for(path) else {
        bail!("no transcript folder found for '{path}'");
    };

    let mut outcome = DeleteOutcome {
        files_deleted: 0,
        bytes_freed: 0,
        failed: Vec::new(),
    };
    let mut deleted_paths = Vec::new();

    for file in jsonl_files(&dir) {
        match fs::remove_file(&file.path) {
            Ok(()) => {
                outcome.files_deleted += 1;
                outcome.bytes_freed += file.size_bytes;
                deleted_paths.push(file.path);
            }
            Err(error) => outcome.failed.push(format!("{}: {error}", file.name)),
        }
    }

    // Only the leftovers of an empty folder tree; anything still holding files
    // stays. Errors are irrelevant — the folder not being empty is expected.
    for entry in WalkDir::new(&dir)
        .contents_first(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
    {
        let _ = fs::remove_dir(entry.path());
    }

    purge_scanned_files(conn, &deleted_paths)?;
    Ok(outcome)
}

fn purge_scanned_files(conn: &mut Connection, files: &[String]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction()?;
    for path in files {
        let file_id: Option<i64> = tx
            .query_row("SELECT id FROM scan_files WHERE path = ?1", [path], |row| {
                row.get(0)
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;

        if let Some(id) = file_id {
            tx.execute(
                "DELETE FROM turn_tools WHERE turn_id IN
                    (SELECT id FROM turns WHERE scan_file_id = ?1)",
                [id],
            )?;
            tx.execute("DELETE FROM turns WHERE scan_file_id = ?1", [id])?;
            tx.execute("DELETE FROM scan_files WHERE id = ?1", [id])?;
        }
    }
    tx.execute_batch(
        "DELETE FROM sessions WHERE session_id NOT IN (SELECT DISTINCT session_id FROM turns);
         DELETE FROM session_activity WHERE session_id NOT IN (SELECT session_id FROM sessions);
         DELETE FROM session_tags WHERE session_id NOT IN (SELECT session_id FROM sessions);",
    )?;
    tx.commit()?;

    crate::ingest::scanner::recompute_derived(conn)?;
    Ok(())
}

pub fn settings_json(path: &str) -> Result<String> {
    let config_path = crate::paths::claude_config_path()?;
    let value = read_config(&config_path)?;
    let entry = value
        .get("projects")
        .and_then(|p| p.get(path))
        .with_context(|| format!("'{path}' is not registered"))?;
    Ok(serde_json::to_string_pretty(entry)?)
}

pub fn update_settings(path: &str, settings: &str) -> Result<WriteOutcome> {
    let parsed: Value = serde_json::from_str(settings).context("settings are not valid JSON")?;
    if !parsed.is_object() {
        bail!("settings must be a JSON object");
    }

    modify_config(|projects| {
        if !projects.contains_key(path) {
            bail!("'{path}' is not registered");
        }
        projects.insert(path.to_owned(), parsed);
        Ok(())
    })
}

pub fn remove_registration(path: &str) -> Result<WriteOutcome> {
    modify_config(|projects| {
        if projects.shift_remove(path).is_none() {
            bail!("'{path}' is not registered");
        }
        Ok(())
    })
}

fn read_config(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn modify_config<F>(change: F) -> Result<WriteOutcome>
where
    F: FnOnce(&mut Map<String, Value>) -> Result<()>,
{
    modify_config_at(&crate::paths::claude_config_path()?, change)
}

/// Every change to `~/.claude.json` goes through here: backup first, then an
/// atomic replace. The rest of the file is carried over untouched.
fn modify_config_at<F>(config_path: &Path, change: F) -> Result<WriteOutcome>
where
    F: FnOnce(&mut Map<String, Value>) -> Result<()>,
{
    let mut value = read_config(config_path)?;

    let projects = value
        .get_mut("projects")
        .and_then(Value::as_object_mut)
        .context("~/.claude.json has no 'projects' object")?;
    change(projects)?;

    let backup_path = backup_config(config_path)?;

    let serialized = serde_json::to_string(&value)?;
    let tmp_path = config_path.with_extension("json.claudeadmin-tmp");
    fs::write(&tmp_path, &serialized).with_context(|| format!("writing {}", tmp_path.display()))?;
    fs::rename(&tmp_path, config_path)
        .with_context(|| format!("replacing {}", config_path.display()))?;

    Ok(WriteOutcome {
        backup_path: backup_path.to_string_lossy().into_owned(),
    })
}

fn backup_config(config_path: &Path) -> Result<PathBuf> {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let backup = config_path.with_file_name(format!(".claude.json.backup-{stamp}"));
    fs::copy(config_path, &backup)
        .with_context(|| format!("backing up to {}", backup.display()))?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_like_claude_code() {
        assert_eq!(
            encode_project_dir(r"C:\Sources\ClaudeAdmin"),
            "C--Sources-ClaudeAdmin"
        );
        assert_eq!(
            encode_project_dir(r"C:\Sources\OpenTelemetryExtension.Configuration"),
            "C--Sources-OpenTelemetryExtension-Configuration"
        );
        assert_eq!(
            encode_project_dir(r"C:\Sources\AI-Agenten"),
            "C--Sources-AI-Agenten"
        );
    }

    #[test]
    fn normalize_merges_slash_variants() {
        assert_eq!(
            normalize(r"C:\Sources\MyApp"),
            normalize("C:/Sources/MyApp/")
        );
    }

    fn temp_config(content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "claude-admin-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".claude.json");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn removing_a_registration_backs_up_and_keeps_the_rest() {
        let path = temp_config(
            r#"{"numStartups":7,"projects":{"C:\\A":{"allowedTools":[]},"C:\\B":{"x":1}},"tail":true}"#,
        );

        let outcome = modify_config_at(&path, |projects| {
            assert!(projects.shift_remove("C:\\A").is_some());
            Ok(())
        })
        .unwrap();

        let backup = fs::read_to_string(&outcome.backup_path).unwrap();
        assert!(backup.contains(r#""C:\\A""#), "backup holds the old state");

        let written: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(written.get("projects").unwrap().get("C:\\A").is_none());
        assert_eq!(
            written.get("projects").unwrap().get("C:\\B").unwrap()["x"],
            1
        );
        // Everything outside `projects` survives, in its original order.
        assert_eq!(written.get("numStartups").unwrap(), 7);
        assert_eq!(written.get("tail").unwrap(), true);
        let keys: Vec<&String> = written.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["numStartups", "projects", "tail"]);
    }

    #[test]
    #[ignore = "reads the developer's own ~/.claude.json and database"]
    fn lists_the_real_projects() {
        let conn = crate::storage::open(&crate::paths::database_path()).unwrap();
        let report = list(&conn).unwrap();
        assert!(report.config_exists);
        assert!(!report.projects.is_empty());
        for row in &report.projects {
            println!(
                "{:60} reg={} dir={} dup={} files={} sessions={}",
                row.path,
                row.registered,
                row.dir_exists,
                row.duplicate_group.is_some(),
                row.transcript_files,
                row.sessions
            );
        }
    }

    #[test]
    fn a_failing_change_writes_nothing() {
        let path = temp_config(r#"{"projects":{}}"#);
        let before = fs::read_to_string(&path).unwrap();

        let result = modify_config_at(&path, |_| bail!("no"));

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        let dir = path.parent().unwrap();
        let backups = fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(".claude.json.backup-")
            })
            .count();
        assert_eq!(backups, 0, "no backup for a change that never happened");
    }
}
