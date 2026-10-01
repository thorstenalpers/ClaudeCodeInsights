//! What RTK, the CLI proxy, has saved.
//!
//! RTK records every command it filtered in a SQLite database of its own. That
//! database is foreign territory like the transcripts: it is opened read-only,
//! never migrated, and never written to.

use crate::analysis::same_path;
use crate::error::Result;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

/// How many recent parse failures travel to the window. They are a tail worth
/// reading, not a second history.
const FAILURE_LIMIT: i64 = 50;

/// The names the binary goes by, in the order worth trying inside a directory.
#[cfg(windows)]
const BINARIES: &[&str] = &["rtk.exe", "rtk.cmd", "rtk.bat", "rtk"];
#[cfg(not(windows))]
const BINARIES: &[&str] = &["rtk"];

/// Where RTK keeps what it has recorded.
pub fn history_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("rtk")
        .join("history.db")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    /// Named whether or not it exists: it is the answer to "where is this read
    /// from", which a reader should not have to guess at.
    pub history: String,
    pub history_exists: bool,
}

/// The configured binary, or the first `rtk` on PATH.
pub fn locate(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|value| !value.trim().is_empty()) {
        let path = PathBuf::from(path.trim());
        return path.is_file().then_some(path);
    }

    std::env::split_paths(&std::env::var_os("PATH")?)
        .flat_map(|dir| BINARIES.iter().map(move |name| dir.join(name)))
        .find(|path| path.is_file())
}

pub fn status(configured: Option<&str>) -> Status {
    let path = locate(configured);
    let history = history_path();

    Status {
        found: path.is_some(),
        version: path.as_deref().and_then(version_of),
        path: path.map(|value| value.to_string_lossy().into_owned()),
        history_exists: history.is_file(),
        history: history.to_string_lossy().into_owned(),
    }
}

/// What `rtk --version` answers, or none when it will not say.
fn version_of(binary: &std::path::Path) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub commands: i64,
    /// What the raw output would have cost the model's context.
    pub input_tokens: i64,
    /// What reached it after RTK had filtered.
    pub output_tokens: i64,
    pub saved_tokens: i64,
    pub exec_time_ms: i64,
    pub failures: i64,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
}

/// A row of either breakdown: by the filter that ran, or by the project it ran
/// in. The same figures answer both questions, so they share a shape.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub name: String,
    pub calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub saved_tokens: i64,
    pub exec_time_ms: i64,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub date: String,
    pub calls: i64,
    pub input_tokens: i64,
    pub saved_tokens: i64,
}

/// A command RTK could not parse and had to run raw.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub ts: String,
    pub command: String,
    pub message: String,
    /// It still ran, unfiltered — a failure that cost tokens rather than work.
    pub recovered: bool,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub summary: Summary,
    pub filters: Vec<Group>,
    pub projects: Vec<Group>,
    pub days: Vec<Day>,
    pub failures: Vec<Failure>,
}

/// One command, as RTK's history stores it.
#[derive(Debug, Default)]
pub struct Recorded {
    pub timestamp: String,
    pub rtk_cmd: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub saved_tokens: i64,
    pub exec_time_ms: i64,
    pub project_path: String,
}

/// Everything the page shows, or an empty report when RTK has recorded nothing.
pub fn report() -> Result<Report> {
    let path = history_path();
    if !path.is_file() {
        return Ok(Report::default());
    }

    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    let mut statement = conn.prepare(
        "SELECT timestamp, rtk_cmd, input_tokens, output_tokens, saved_tokens,
                COALESCE(exec_time_ms, 0), COALESCE(project_path, '')
         FROM commands",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(Recorded {
                timestamp: row.get(0)?,
                rtk_cmd: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                saved_tokens: row.get(4)?,
                exec_time_ms: row.get(5)?,
                project_path: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut report = digest(&rows);
    report.summary.failures =
        conn.query_row("SELECT COUNT(*) FROM parse_failures", [], |row| row.get(0))?;
    report.failures = failures(&conn)?;
    Ok(report)
}

fn failures(conn: &Connection) -> rusqlite::Result<Vec<Failure>> {
    let mut statement = conn.prepare(
        "SELECT timestamp, raw_command, error_message, fallback_succeeded
         FROM parse_failures ORDER BY timestamp DESC LIMIT ?1",
    )?;
    let rows = statement.query_map([FAILURE_LIMIT], |row| {
        Ok(Failure {
            ts: row.get(0)?,
            command: row.get(1)?,
            message: row.get(2)?,
            recovered: row.get::<_, i64>(3)? != 0,
        })
    })?;
    rows.collect()
}

/// Folds the recorded commands into the breakdowns the page draws.
///
/// A percentage is deliberately absent everywhere: it is `saved / input` and
/// the window works it out, so a total and a row can never disagree about it.
pub fn digest(rows: &[Recorded]) -> Report {
    let mut summary = Summary::default();
    let mut filters: BTreeMap<String, Group> = BTreeMap::new();
    let mut projects: BTreeMap<String, Group> = BTreeMap::new();
    let mut days: BTreeMap<String, Day> = BTreeMap::new();

    for row in rows {
        summary.commands += 1;
        summary.input_tokens += row.input_tokens;
        summary.output_tokens += row.output_tokens;
        summary.saved_tokens += row.saved_tokens;
        summary.exec_time_ms += row.exec_time_ms;

        if summary
            .first_ts
            .as_deref()
            .is_none_or(|at| row.timestamp.as_str() < at)
        {
            summary.first_ts = Some(row.timestamp.clone());
        }
        if summary
            .last_ts
            .as_deref()
            .is_none_or(|at| row.timestamp.as_str() > at)
        {
            summary.last_ts = Some(row.timestamp.clone());
        }

        let filter = filter_of(&row.rtk_cmd);
        add(filters.entry(filter.to_owned()).or_default(), filter, row);

        let project = plain_path(&row.project_path);
        if !project.is_empty() {
            add(
                projects.entry(same_path(project)).or_default(),
                project,
                row,
            );
        }

        if let Some(date) = row.timestamp.get(..10) {
            let day = days.entry(date.to_owned()).or_default();
            day.date = date.to_owned();
            day.calls += 1;
            day.input_tokens += row.input_tokens;
            day.saved_tokens += row.saved_tokens;
        }
    }

    Report {
        summary,
        filters: ranked(filters),
        projects: ranked(projects),
        days: days.into_values().collect(),
        failures: Vec::new(),
    }
}

fn add(into: &mut Group, name: &str, row: &Recorded) {
    if into.name.is_empty() {
        into.name = name.to_owned();
    }
    into.calls += 1;
    into.input_tokens += row.input_tokens;
    into.output_tokens += row.output_tokens;
    into.saved_tokens += row.saved_tokens;
    into.exec_time_ms += row.exec_time_ms;
}

/// Biggest saving first, and among equals the one that ran more often.
fn ranked(groups: BTreeMap<String, Group>) -> Vec<Group> {
    let mut rows: Vec<Group> = groups.into_values().collect();
    rows.sort_by(|a, b| {
        b.saved_tokens
            .cmp(&a.saved_tokens)
            .then(b.calls.cmp(&a.calls))
            .then(a.name.cmp(&b.name))
    });
    rows
}

/// The filter a command went through — `git`, `cargo`, `ls`.
///
/// `proxy` and `run` count as filters like any other: those are the calls that
/// passed through unfiltered, and leaving them out would flatter the numbers.
fn filter_of(rtk_cmd: &str) -> &str {
    let mut parts = rtk_cmd.split_whitespace();
    let Some(first) = parts.next() else {
        return "";
    };
    if is_rtk(first) {
        parts.next().unwrap_or(first)
    } else {
        first
    }
}

fn is_rtk(token: &str) -> bool {
    let name = token.rsplit(['/', '\\']).next().unwrap_or(token);
    name.strip_suffix(".exe")
        .unwrap_or(name)
        .eq_ignore_ascii_case("rtk")
}

/// RTK records the extended-length spelling Windows hands it. `\\?\C:\x` and
/// `C:\x` are one directory, and only one of them belongs on screen.
fn plain_path(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorded(at: &str, cmd: &str, input: i64, saved: i64, project: &str) -> Recorded {
        Recorded {
            timestamp: at.to_owned(),
            rtk_cmd: cmd.to_owned(),
            input_tokens: input,
            output_tokens: input - saved,
            saved_tokens: saved,
            exec_time_ms: 10,
            project_path: project.to_owned(),
        }
    }

    #[test]
    fn the_filter_is_the_word_after_the_binary() {
        assert_eq!(filter_of("rtk git status"), "git");
        assert_eq!(filter_of("C:/bin/rtk.exe cargo build"), "cargo");
        assert_eq!(filter_of("rtk proxy ls -la"), "proxy");
    }

    #[test]
    fn a_command_that_never_reached_rtk_is_its_own_filter() {
        assert_eq!(filter_of("npm run dev"), "npm");
        assert_eq!(filter_of("rtk"), "rtk");
        assert_eq!(filter_of("   "), "");
    }

    #[test]
    fn the_long_path_prefix_never_reaches_the_window() {
        assert_eq!(plain_path(r"\\?\C:\Sources\App"), r"C:\Sources\App");
        assert_eq!(plain_path(r"C:\Sources\App"), r"C:\Sources\App");
    }

    #[test]
    fn the_totals_are_the_rows_added_up() {
        let report = digest(&[
            recorded("2026-08-01T10:00:00Z", "rtk git status", 100, 40, "C:/a"),
            recorded("2026-08-02T10:00:00Z", "rtk git diff", 200, 100, "C:/a"),
        ]);

        assert_eq!(report.summary.commands, 2);
        assert_eq!(report.summary.input_tokens, 300);
        assert_eq!(report.summary.saved_tokens, 140);
        assert_eq!(report.summary.output_tokens, 160);
        assert_eq!(
            report.summary.first_ts.as_deref(),
            Some("2026-08-01T10:00:00Z")
        );
        assert_eq!(
            report.summary.last_ts.as_deref(),
            Some("2026-08-02T10:00:00Z")
        );
        assert_eq!(report.days.len(), 2);
    }

    #[test]
    fn the_breakdowns_rank_by_what_was_saved() {
        let report = digest(&[
            recorded("2026-08-01T10:00:00Z", "rtk ls -la", 20, 5, "C:/a"),
            recorded("2026-08-01T11:00:00Z", "rtk git status", 100, 60, "C:/b"),
        ]);

        assert_eq!(report.filters[0].name, "git");
        assert_eq!(report.filters[0].saved_tokens, 60);
        assert_eq!(report.filters[1].name, "ls");
    }

    #[test]
    fn one_directory_spelled_two_ways_is_one_project() {
        // The same checkout reaches RTK as whatever the shell used; two rows in
        // the table would read as two projects that were never worked in.
        let report = digest(&[
            recorded(
                "2026-08-01T10:00:00Z",
                "rtk git status",
                100,
                50,
                r"\\?\C:\Sources\App",
            ),
            recorded(
                "2026-08-01T11:00:00Z",
                "rtk git diff",
                100,
                30,
                "C:/Sources/app/",
            ),
        ]);

        assert_eq!(report.projects.len(), 1);
        assert_eq!(report.projects[0].calls, 2);
        assert_eq!(report.projects[0].saved_tokens, 80);
        assert_eq!(report.projects[0].name, r"C:\Sources\App");
    }

    #[test]
    #[ignore = "reads this machine's own RTK history"]
    fn reads_the_real_history() {
        let report = report().unwrap();
        println!(
            "{} commands, {} of {} tokens saved, {} filters, {} projects, {} failures",
            report.summary.commands,
            report.summary.saved_tokens,
            report.summary.input_tokens,
            report.filters.len(),
            report.projects.len(),
            report.summary.failures,
        );
        for row in &report.filters {
            println!(
                "  {:12} {:5} calls {:8} saved",
                row.name, row.calls, row.saved_tokens
            );
        }
    }

    #[test]
    fn a_command_without_a_project_lands_in_no_project() {
        let report = digest(&[recorded("2026-08-01T10:00:00Z", "rtk ls", 10, 1, "")]);
        assert!(report.projects.is_empty());
        assert_eq!(report.filters.len(), 1);
    }

    #[test]
    fn a_configured_path_that_is_not_a_file_counts_as_missing() {
        let status = status(Some("C:/nowhere/rtk.exe"));
        assert!(!status.found);
        assert!(status.path.is_none());
        assert!(!status.history.is_empty());
    }
}
