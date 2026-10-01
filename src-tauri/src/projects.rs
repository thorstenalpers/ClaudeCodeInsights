//! Projects as Claude Code sees them: the registrations in `~/.claude.json`
//! joined with the transcript folders and the figures already in the database.

use crate::analysis::cost::{self, ModelTokens};
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
    /// What the project calls itself — see [`display_name`].
    pub name: String,
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
    /// The same tokens split by model, so the window can price the mix.
    pub by_model: Vec<ModelTokens>,
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

/// What a project is called, as opposed to which folder it sits in.
///
/// The last folder is not a name: `src-tauri`, `app` and `src` sit in every
/// second checkout, and a list keyed by them merges projects that share
/// nothing. So the name is read out of what the checkout says about itself —
/// the manifest at its repository root — and a directory below that root keeps
/// the part of the path that tells it apart, `claude-admin/src-tauri`.
///
/// A directory that is gone falls back to the last two path components, which
/// is all that is left to go on.
pub fn display_name(path: &str) -> String {
    let dir = Path::new(path);
    if !dir.is_dir() {
        return short_path(path);
    }

    let root = git_root(dir);
    let anchor: &Path = root.as_deref().unwrap_or(dir);
    let base = manifest_name(anchor)
        .or_else(|| last_component(anchor))
        .unwrap_or_else(|| short_path(path));

    let below = dir
        .strip_prefix(anchor)
        .ok()
        .map(|rest| rest.to_string_lossy().replace('\\', "/"))
        .filter(|rest| !rest.is_empty());

    match below {
        Some(rest) => format!("{base}/{rest}"),
        None => base,
    }
}

/// The last two path components: enough to tell two projects apart without
/// putting the whole directory layout on screen.
fn short_path(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let parts: Vec<&str> = unified.split('/').filter(|part| !part.is_empty()).collect();
    match parts.len() {
        0 => "unknown".to_owned(),
        1 => parts[0].to_owned(),
        n => format!("{}/{}", parts[n - 2], parts[n - 1]),
    }
}

fn last_component(dir: &Path) -> Option<String> {
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// `.git` is a directory in a checkout and a file in a worktree or submodule,
/// so both count.
fn git_root(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(Path::to_path_buf)
}

/// The name the ecosystem's own manifest gives the project, in the order the
/// manifests are worth trusting.
fn manifest_name(dir: &Path) -> Option<String> {
    json_name(&dir.join("package.json"))
        .or_else(|| toml_name(&dir.join("Cargo.toml"), "package"))
        .or_else(|| toml_name(&dir.join("pyproject.toml"), "project"))
        .or_else(|| toml_name(&dir.join("pyproject.toml"), "tool.poetry"))
        .or_else(|| go_module(&dir.join("go.mod")))
        .or_else(|| solution_or_project_name(dir))
        .or_else(|| json_name(&dir.join("composer.json")))
}

/// The last segment only: `@acme/dashboard` and `acme/dashboard` are read as
/// `dashboard`, because the scope is the same for every package of one vendor.
fn unscoped(name: &str) -> Option<String> {
    let trimmed = name.trim().trim_start_matches('@');
    let tail = trimmed.rsplit('/').next().unwrap_or(trimmed).trim();
    (!tail.is_empty()).then(|| tail.to_owned())
}

fn json_name(file: &Path) -> Option<String> {
    let text = fs::read_to_string(file).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    unscoped(value.get("name")?.as_str()?)
}

/// A hand-rolled read of one `name` key, rather than a TOML dependency for a
/// single line: the manifests that matter here all spell it `name = "..."`.
fn toml_name(file: &Path, section: &str) -> Option<String> {
    let text = fs::read_to_string(file).ok()?;
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            inside = header.trim() == section;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(value) = line
            .strip_prefix("name")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('='))
        {
            return unscoped(value.trim().trim_matches(['"', '\'']));
        }
    }
    None
}

fn go_module(file: &Path) -> Option<String> {
    let text = fs::read_to_string(file).ok()?;
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("module "))?;
    unscoped(line.trim_start_matches("module "))
}

/// .NET names itself after a file rather than inside one. A solution outranks
/// a project file: it is the thing the whole folder is about.
fn solution_or_project_name(dir: &Path) -> Option<String> {
    let mut solutions = Vec::new();
    let mut projects = Vec::new();

    for entry in fs::read_dir(dir).ok()?.filter_map(Result::ok) {
        let path = entry.path();
        let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let stem = path.file_stem()?.to_string_lossy().into_owned();
        match extension.to_ascii_lowercase().as_str() {
            "sln" | "slnx" => solutions.push(stem),
            "csproj" | "fsproj" | "vbproj" => projects.push(stem),
            _ => {}
        }
    }

    solutions.sort();
    projects.sort();
    solutions
        .into_iter()
        .next()
        .or_else(|| projects.into_iter().next())
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
    by_model: Vec<ModelTokens>,
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
                by_model: Vec::new(),
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

    for (path, models) in cost::by_project(conn)? {
        let entry = map.entry(normalize(&path)).or_default();
        for tokens in models {
            match entry
                .by_model
                .iter_mut()
                .find(|entry| entry.model == tokens.model)
            {
                Some(existing) => {
                    existing.input_tokens += tokens.input_tokens;
                    existing.output_tokens += tokens.output_tokens;
                    existing.cache_read_tokens += tokens.cache_read_tokens;
                    existing.cache_write_tokens += tokens.cache_write_tokens;
                }
                None => entry.by_model.push(tokens),
            }
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

/// One directory per project, named after the project path — a layout only
/// Claude Code uses. Codex files a rollout under the day it ran, so a project's
/// transcripts are not a folder there and are not looked for as one.
fn transcript_dir_for(path: &str) -> Option<PathBuf> {
    let encoded = encode_project_dir(path);
    let by_encoding = crate::paths::claude_scan_roots()
        .iter()
        .map(|root| root.join(&encoded))
        .find(|dir| dir.is_dir());

    // A row discovered from disk carries the transcript folder as its path,
    // not a project directory — encoding that again finds nothing, which is
    // what made deleting such a row fail.
    by_encoding.or_else(|| {
        let candidate = Path::new(path);
        let inside_a_root = crate::paths::claude_scan_roots()
            .iter()
            .any(|root| candidate.starts_with(root));
        (inside_a_root && candidate.is_dir()).then(|| candidate.to_path_buf())
    })
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
            name: display_name(path),
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
            by_model: db.by_model,
        });
    }

    // Transcript folders no registration points at: usually projects whose
    // registration was removed, but whose history is still on disk. Claude
    // Code's roots only — a folder is a project there, whereas under Codex it
    // is a date, and every rollout already names the directory it ran in.
    for root in crate::paths::claude_scan_roots() {
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
                name: display_name(&display_path),
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
                by_model: db.by_model,
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

/// Hands `~/.claude.json` to whatever the system opens `.json` with.
///
/// The path is worked out here rather than taken from the window, so the only
/// file this can ever open is Claude Code's own configuration.
pub fn open_config() -> Result<()> {
    let config_path = crate::paths::claude_config_path()?;
    if !config_path.is_file() {
        bail!("{} does not exist", config_path.display());
    }

    #[cfg(windows)]
    std::process::Command::new("explorer.exe")
        .arg(&config_path)
        .spawn()?;

    #[cfg(not(windows))]
    std::process::Command::new("xdg-open")
        .arg(&config_path)
        .spawn()?;

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

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "claude-admin-test-{}-{}-{name}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_name_comes_from_the_manifest_not_the_folder() {
        let root = temp_dir("repo");
        fs::write(root.join("package.json"), r#"{"name":"@acme/dashboard"}"#).unwrap();
        assert_eq!(display_name(&root.to_string_lossy()), "dashboard");

        let cargo = temp_dir("crate");
        fs::write(
            cargo.join("Cargo.toml"),
            "[workspace]\nname = \"wrong\"\n\n[package]\nname = \"claude-admin\"\n",
        )
        .unwrap();
        assert_eq!(display_name(&cargo.to_string_lossy()), "claude-admin");

        let dotnet = temp_dir("dotnet");
        fs::write(dotnet.join("Widgets.sln"), "").unwrap();
        fs::write(dotnet.join("Widgets.Core.csproj"), "").unwrap();
        assert_eq!(display_name(&dotnet.to_string_lossy()), "Widgets");
    }

    #[test]
    fn a_folder_inside_a_repository_is_named_after_the_repository() {
        // The case this exists for: `src-tauri` under two different checkouts
        // used to read as one project.
        let root = temp_dir("repo");
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("package.json"), r#"{"name":"claude-admin"}"#).unwrap();
        let inner = root.join("src-tauri");
        fs::create_dir_all(&inner).unwrap();

        assert_eq!(
            display_name(&inner.to_string_lossy()),
            "claude-admin/src-tauri"
        );
    }

    #[test]
    fn a_directory_that_is_gone_keeps_its_last_two_components() {
        assert_eq!(
            display_name(r"C:\Sources\MyApp\nowhere-at-all"),
            "MyApp/nowhere-at-all"
        );
        assert_eq!(display_name(""), "unknown");
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
