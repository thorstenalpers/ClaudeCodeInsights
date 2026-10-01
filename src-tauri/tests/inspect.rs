//! What the window shows, printed, so it can be stepped through.
//!
//! Every test here reads this machine's own database and transcripts, which is
//! why all of them are `#[ignore]`: there is no fixture to make them pass
//! anywhere else, and a green CI must not depend on what happens to be scanned.
//! They exist to be run one at a time, with a breakpoint in the query they call:
//!
//! ```text
//! cargo test --test inspect -- --ignored --nocapture
//! cargo test --test inspect one_session_in_full -- --ignored --nocapture
//! ```
//!
//! `SESSION=<id>` picks the session the detail test opens; without it the newest
//! one is taken. Nothing here writes: the database is opened the way the app
//! opens it, and only ever read from.

use claude_admin_lib::analysis::sessions::{self, SessionQuery};
use claude_admin_lib::analysis::{rhythm, series, transcript, usage};
use claude_admin_lib::{commands, paths, storage};
use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The scanned database, or `None` when nothing has been scanned on this
/// machine yet — a reason to skip rather than to fail.
fn database() -> Option<Connection> {
    let path = paths::database_path();
    if !path.is_file() {
        eprintln!("skipped: no database at {}", path.display());
        return None;
    }
    Some(storage::open(&path).expect("opening the database"))
}

macro_rules! db {
    () => {
        match database() {
            Some(conn) => conn,
            None => return,
        }
    };
}

fn cut(text: &str, width: usize) -> String {
    let single_line = text.replace('\n', " ");
    match single_line.char_indices().nth(width) {
        Some((at, _)) => format!("{}…", &single_line[..at]),
        None => single_line,
    }
}

#[test]
#[ignore = "reads this machine's own database"]
fn sessions_on_this_machine() {
    let conn = db!();

    let page = sessions::query(
        &conn,
        &SessionQuery {
            page_size: 40,
            ..SessionQuery::default()
        },
    )
    .expect("listing sessions");

    eprintln!("{} sessions, newest {} shown", page.total, page.rows.len());
    for row in &page.rows {
        let compacted = match (row.compact_auto, row.compact_manual) {
            (0, 0) => String::new(),
            (auto, manual) => format!(" [{auto} overflow, {manual} asked]"),
        };
        eprintln!(
            "{:8} {:26} {:22} {:14} {:>5} turns {:>9} tokens {:>4} files  {}{}",
            &row.session_id[..8.min(row.session_id.len())],
            cut(row.topic.as_deref().unwrap_or("—"), 25),
            cut(row.project_name.as_deref().unwrap_or("—"), 21),
            row.activity,
            row.turn_count,
            row.input_tokens + row.output_tokens + row.cache_read_tokens + row.cache_write_tokens,
            row.files_touched,
            row.last_ts.as_deref().unwrap_or("—"),
            compacted,
        );
    }

    assert_eq!(page.rows.len() as i64, page.total.min(40));
}

#[test]
#[ignore = "reads this machine's own transcripts"]
fn one_session_in_full() {
    let conn = db!();

    let wanted = std::env::var("SESSION").unwrap_or_default();
    let page = sessions::query(
        &conn,
        &SessionQuery {
            page_size: 500,
            ..SessionQuery::default()
        },
    )
    .expect("listing sessions");

    let Some(row) = page
        .rows
        .iter()
        .find(|row| !wanted.is_empty() && row.session_id.starts_with(&wanted))
        .or_else(|| page.rows.first())
    else {
        eprintln!("skipped: nothing scanned yet");
        return;
    };

    eprintln!("session      {}", row.session_id);
    eprintln!("topic        {}", row.topic.as_deref().unwrap_or("—"));
    eprintln!(
        "project      {}",
        row.project_name.as_deref().unwrap_or("—")
    );
    eprintln!(
        "directory    {}",
        row.project_path.as_deref().unwrap_or("—")
    );
    eprintln!("branch       {}", row.git_branch.as_deref().unwrap_or("—"));
    eprintln!("model        {}", row.model.as_deref().unwrap_or("—"));
    eprintln!("activity     {} {:?}", row.activity, row.profile);
    eprintln!("tags         {:?}", row.tags);
    eprintln!(
        "span         {} → {} ({} min, {} turns)",
        row.first_ts.as_deref().unwrap_or("—"),
        row.last_ts.as_deref().unwrap_or("—"),
        row.duration_minutes,
        row.turn_count,
    );

    // Paged the way the detail view pages it, so a breakpoint here sees the
    // same call the window makes. A transcript deleted since the scan is a
    // state the window can be in too, so it is reported rather than fatal.
    let transcript = match transcript::load(&conn, &row.session_id, 0, 200) {
        Ok(page) => page,
        Err(error) => {
            eprintln!("transcript   unreadable: {error:#}");
            return;
        }
    };
    eprintln!(
        "transcript   {} turns from {}",
        transcript.total,
        transcript.path.as_deref().unwrap_or("nowhere"),
    );

    for turn in &transcript.turns {
        eprintln!(
            "  [{:>3}] {:9} {:20} {}",
            turn.index,
            turn.role,
            turn.timestamp.as_deref().unwrap_or("—"),
            cut(turn.text.as_deref().unwrap_or(""), 90),
        );
        if let Some(thinking) = &turn.thinking {
            eprintln!("        thinking  {}", cut(thinking, 90));
        }
        for call in &turn.tool_calls {
            eprintln!(
                "        {} {:16} in {} out {}",
                if call.is_error { "✗" } else { "·" },
                call.name,
                cut(&call.input, 60),
                cut(call.result.as_deref().unwrap_or("—"), 60),
            );
        }
    }

    assert_eq!(transcript.turns.len(), transcript.total.min(200));
}

/// Everything the projects page knows, through the commands it calls.
///
/// Nothing is re-derived here: `list_projects`, `preview_project_transcripts`
/// and `get_project_settings` are the three the window invokes, opening their
/// own connection the way they do in the running app. Each row is printed as
/// the JSON that crosses to the window, so a field added to `ProjectRow` shows
/// up without anyone remembering to print it.
///
/// The registration is Claude Code's own; it is shown the way the settings
/// dialog shows it, shortened because it carries the whole prompt history.
#[test]
#[ignore = "reads this machine's own registrations and checkouts"]
fn every_project_in_full() {
    let report = commands::projects::list_projects().expect("list_projects");

    eprintln!("config       {}", report.config_path);
    eprintln!("exists       {}", report.config_exists);
    eprintln!("projects     {}", report.projects.len());

    for row in &report.projects {
        eprintln!("\n=== {} ===", row.name);
        eprintln!(
            "{}",
            serde_json::to_string_pretty(row).expect("serialising the row")
        );

        let files = commands::projects::preview_project_transcripts(row.path.clone())
            .expect("preview_project_transcripts");
        eprintln!("transcripts  {} files", files.len());
        for file in &files {
            eprintln!("  {:>10} B  {}", file.size_bytes, file.name);
        }
        // The list and the delete dialog must see the same files, or the
        // dialog names something other than what disappears.
        assert_eq!(
            files.len() as i64,
            row.transcript_files,
            "{}: the row counts {} transcripts, the delete preview finds {}",
            row.path,
            row.transcript_files,
            files.len(),
        );

        if row.registered {
            match commands::projects::get_project_settings(row.path.clone()) {
                Ok(settings) => {
                    let keys: Vec<String> = serde_json::from_str::<serde_json::Value>(&settings)
                        .ok()
                        .and_then(|value| {
                            value
                                .as_object()
                                .map(|entry| entry.keys().cloned().collect())
                        })
                        .unwrap_or_default();
                    eprintln!("settings     {} keys: {}", keys.len(), keys.join(", "));
                    eprintln!("             {}", cut(&settings, 600));
                }
                Err(error) => eprintln!("settings     unreadable: {error}"),
            }
        }
    }

    // Two registrations of one directory — `~/.claude.json` holds the same
    // place once with each separator — are one project and share a name by
    // design; only different directories under one name are the fault, and
    // avoiding those is the whole reason the name comes from the manifest.
    let mut by_name: HashMap<&str, Vec<String>> = HashMap::new();
    for row in &report.projects {
        let place = row
            .path
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_lowercase();
        let places = by_name.entry(&row.name).or_default();
        if !places.contains(&place) {
            places.push(place);
        }
    }

    let collisions: Vec<_> = by_name
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .collect();
    for (name, paths) in &collisions {
        eprintln!("\nshared name {name}: {paths:?}");
    }
    assert!(
        collisions.is_empty(),
        "{} project names cover more than one directory",
        collisions.len()
    );
}

#[test]
#[ignore = "reads this machine's own database"]
fn every_roll_up_the_pages_ask_for() {
    let conn = db!();

    let tools = usage::tools(&conn).expect("tools");
    let models = usage::models(&conn).expect("models");
    let activities = usage::activities(&conn).expect("activities");
    let agents = usage::agents(&conn).expect("agents");
    let beat = rhythm::load(&conn).expect("rhythm");
    let facets = sessions::facets(&conn).expect("session facets");

    eprintln!("tools        {}", tools.len());
    for row in tools.iter().take(15) {
        let outcome = if row.answered == 0 {
            "no outcome recorded".to_owned()
        } else {
            format!(
                "{:>4} of {:>6} failed ({:.1}%)",
                row.failed,
                row.answered,
                row.failed as f64 / row.answered as f64 * 100.0
            )
        };
        eprintln!(
            "  {:24} {:>7} calls in {:>4} sessions, {outcome}",
            row.name, row.calls, row.sessions
        );
    }
    eprintln!("models       {}", models.len());
    for row in &models {
        eprintln!("  {:34} {:>6} turns", row.model, row.turns);
    }
    eprintln!("activities   {}", activities.len());
    eprintln!("agents       {}", agents.len());
    eprintln!(
        "rhythm       {} active days, streak {} (longest {}), busiest hour {:?}",
        beat.active_days, beat.current_streak, beat.longest_streak, beat.busiest_hour,
    );
    eprintln!(
        "facets       {} projects, {} branches, {} tags",
        facets.projects.len(),
        facets.branches.len(),
        facets.tags.len(),
    );

    // The history behind the cost chart, one row per day and model, which is
    // where an axis label comes from.
    let points = series::load(
        &conn,
        &series::SeriesQuery {
            group_by: "model".to_owned(),
            models: Vec::new(),
            activities: Vec::new(),
            tools: Vec::new(),
            projects: Vec::new(),
            branches: Vec::new(),
            from: None,
            to: None,
        },
    )
    .expect("series");
    eprintln!("series       {} points", points.points.len());
    for point in points.points.iter().take(10) {
        eprintln!(
            "  {} {:28} {:>9} in",
            point.date, point.key, point.input_tokens
        );
    }
}

/// Where `tauri_plugin_log`'s `LogDir` target writes, worked out from the
/// identifier in the config rather than repeated here.
fn log_dir() -> Option<PathBuf> {
    let config = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json"))
        .expect("reading tauri.conf.json");
    let identifier = serde_json::from_str::<serde_json::Value>(&config)
        .expect("parsing tauri.conf.json")
        .get("identifier")?
        .as_str()?
        .to_owned();
    Some(dirs::data_local_dir()?.join(identifier).join("logs"))
}

#[test]
#[ignore = "reads what the app last logged on this machine"]
fn the_hosts_log() {
    /// Enough to cover a scan and whatever went wrong after it.
    const TAIL: usize = 400;

    let Some(dir) = log_dir() else {
        eprintln!("skipped: no log directory for this platform");
        return;
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        eprintln!(
            "skipped: {} does not exist — the app has not run yet",
            dir.display()
        );
        return;
    };

    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();

    assert!(!files.is_empty(), "{} holds no log file", dir.display());

    for file in &files {
        let text = fs::read_to_string(file).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let from = lines.len().saturating_sub(TAIL);
        eprintln!(
            "--- {} ({} lines, last {} shown) ---",
            file.display(),
            lines.len(),
            lines.len() - from,
        );
        for line in &lines[from..] {
            eprintln!("{line}");
        }
    }
}
