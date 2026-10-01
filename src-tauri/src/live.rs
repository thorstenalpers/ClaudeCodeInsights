//! What Claude Code is doing right now, read off the file it is writing.
//!
//! A session's transcript is appended to while the work happens, so watching
//! the newest `.jsonl` under the scan roots is as close to live as this app
//! can get without owning the process. Nothing is written back: the transcript
//! stays foreign, read-only territory.
//!
//! Answering a question that Claude Code asks is deliberately not here. A
//! running CLI owns its own stdin; there is no channel into it from outside,
//! and faking one by writing into the transcript would lie to the file without
//! reaching the process.

use crate::error::{Error, Result};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

/// One line of a transcript, as much of it as a reader needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveTurn {
    pub session_id: String,
    pub project: Option<String>,
    pub git_branch: Option<String>,
    pub role: String,
    pub timestamp: Option<String>,
    pub text: Option<String>,
    /// The tools this line called, by name, in order.
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    /// True when the line carried a thinking block.
    pub thinking: bool,
    /// A line that reached for a tool is work on the code; one that did not is
    /// conversation. Derived here so every row can be told apart at a glance.
    pub kind: String,
    /// A line from a subagent's own thread rather than from the conversation.
    pub agent: bool,
}

/// The file being followed and how far it has been read.
struct Follow {
    path: PathBuf,
    offset: u64,
    /// Codex names the model and the directory once per turn rather than on
    /// every line, so the last ones seen are carried to the lines they apply to.
    model: Option<String>,
    project: Option<String>,
}

static STATE: OnceLock<Mutex<Vec<Follow>>> = OnceLock::new();

fn state() -> &'static Mutex<Vec<Follow>> {
    STATE.get_or_init(|| Mutex::new(Vec::new()))
}

/// How many transcripts are followed at once.
///
/// Several sessions run side by side often enough that following only the
/// newest shows the wrong one; past a handful the tabs stop being readable.
const FOLLOWED: usize = 6;

/// The most recently written transcripts under the scan roots, newest first.
///
/// The depth covers both layouts: Claude Code files one directory per project,
/// Codex one per day, which is three levels of its own.
pub fn newest_transcripts(limit: usize) -> Vec<PathBuf> {
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();

    for root in crate::paths::default_scan_roots() {
        for entry in walkdir::WalkDir::new(&root)
            .max_depth(4)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
        {
            let Ok(modified) = entry
                .metadata()
                .map_err(std::io::Error::from)
                .and_then(|meta| meta.modified())
            else {
                continue;
            };
            found.push((modified, entry.path().to_path_buf()));
        }
    }

    found.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    found
        .into_iter()
        .take(limit)
        .map(|(_, path)| path)
        .collect()
}

/// Reads one transcript line into what the window shows.
///
/// Anything that is not a turn — summaries, meta records — is skipped rather
/// than shown as an empty row.
fn parse(line: &str, path: &Path) -> Option<LiveTurn> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let message = value.get("message")?;
    let role = message.get("role")?.as_str()?.to_owned();

    let mut text = String::new();
    let mut tools = Vec::new();
    let mut thinking = false;

    match message.get("content") {
        Some(serde_json::Value::String(plain)) => text.push_str(plain),
        Some(serde_json::Value::Array(parts)) => {
            for part in parts {
                match part.get("type").and_then(serde_json::Value::as_str) {
                    Some("text") => {
                        if let Some(value) = part.get("text").and_then(serde_json::Value::as_str) {
                            text.push_str(value);
                        }
                    }
                    Some("tool_use") => {
                        if let Some(name) = part.get("name").and_then(serde_json::Value::as_str) {
                            tools.push(name.to_owned());
                        }
                    }
                    Some("thinking") => thinking = true,
                    _ => {}
                }
            }
        }
        _ => {}
    }

    if text.trim().is_empty() && tools.is_empty() && !thinking {
        return None;
    }

    let usage = |field: &str| {
        message
            .get("usage")
            .and_then(|usage| usage.get(field))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0)
    };

    Some(LiveTurn {
        session_id: value
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("")
            })
            .to_owned(),
        project: value
            .get("cwd")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        role,
        timestamp: value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        text: (!text.trim().is_empty()).then(|| text.trim().to_owned()),
        kind: if tools.is_empty() { "chat" } else { "code" }.to_owned(),
        tools,
        git_branch: value
            .get("gitBranch")
            .and_then(serde_json::Value::as_str)
            .filter(|branch| !branch.is_empty())
            .map(str::to_owned),
        model: message
            .get("model")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        input_tokens: usage("input_tokens"),
        output_tokens: usage("output_tokens"),
        cache_read_tokens: usage("cache_read_input_tokens"),
        cache_write_tokens: usage("cache_creation_input_tokens"),
        thinking,
        agent: value
            .get("isSidechain")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

/// Reads one Codex rollout line into what the window shows.
///
/// Codex spreads a turn over several lines: what it said, what it asked for,
/// and only then what it cost. The tally is therefore folded back into the row
/// it belongs to instead of becoming a row of its own with nothing in it.
fn apply_codex_line(line: &str, follow: &mut Follow, turns: &mut Vec<LiveTurn>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return;
    };
    let payload = value.get("payload").unwrap_or(&serde_json::Value::Null);
    let kind = payload
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    let string = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(|text| text.trim().to_owned())
    };

    match value.get("type").and_then(serde_json::Value::as_str) {
        Some("turn_context") | Some("session_meta") => {
            if let Some(name) = string(payload, "model") {
                follow.model = Some(name);
            }
            if let Some(cwd) = string(payload, "cwd") {
                follow.project = Some(cwd);
            }
            return;
        }
        _ => {}
    }

    if kind == "token_count" {
        // The tally reports the call that has just been written, and Codex
        // counts the cached part inside the input.
        let Some(last) = payload
            .get("info")
            .and_then(|info| info.get("last_token_usage"))
        else {
            return;
        };
        let count = |key: &str| {
            last.get(key)
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0)
        };
        if let Some(turn) = turns.last_mut() {
            turn.cache_read_tokens = count("cached_input_tokens");
            turn.input_tokens = (count("input_tokens") - turn.cache_read_tokens).max(0);
            turn.output_tokens = count("output_tokens");
        }
        return;
    }

    let (role, text, thinking, tools) = match kind {
        "user_message" => ("user", string(payload, "message"), false, Vec::new()),
        "agent_message" => ("assistant", string(payload, "message"), false, Vec::new()),
        "agent_reasoning" => ("assistant", string(payload, "text"), true, Vec::new()),
        "function_call" => (
            "assistant",
            None,
            false,
            string(payload, "name").into_iter().collect(),
        ),
        _ => return,
    };

    if text.is_none() && tools.is_empty() && !thinking {
        return;
    }

    turns.push(LiveTurn {
        session_id: crate::ingest::codex::session_id_from_path(&follow.path),
        project: follow.project.clone(),
        git_branch: None,
        role: role.to_owned(),
        timestamp: value
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        text,
        kind: if tools.is_empty() { "chat" } else { "code" }.to_owned(),
        tools,
        model: follow.model.clone(),
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        thinking,
        agent: false,
    });
}

/// Reads whatever has been appended since the last look.
fn read_new(follow: &mut Follow) -> Vec<LiveTurn> {
    let Ok(file) = File::open(&follow.path) else {
        return Vec::new();
    };
    let Ok(length) = file.metadata().map(|meta| meta.len()) else {
        return Vec::new();
    };

    // A shorter file is a different file: the session was replaced, so the
    // reader starts over rather than reading from a nonsense offset.
    if length < follow.offset {
        follow.offset = 0;
    }

    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::Start(follow.offset)).is_err() {
        return Vec::new();
    }

    let mut turns = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(bytes) => {
                // A line without its newline is still being written; leave the
                // offset before it and pick it up on the next event.
                if !line.ends_with('\n') {
                    break;
                }
                follow.offset += bytes as u64;
                match crate::ingest::source::Source::of_path(&follow.path) {
                    crate::ingest::source::Source::Codex => {
                        apply_codex_line(&line, follow, &mut turns);
                    }
                    crate::ingest::source::Source::Claude => {
                        if let Some(turn) = parse(&line, &follow.path) {
                            turns.push(turn);
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }

    turns
}

/// Starts following the newest transcript, and keeps following it.
///
/// The last few turns are sent straight away so the page is not empty while
/// nothing happens; everything after that arrives as it is written.
pub fn start(app: &AppHandle, tail: usize) -> Result<Vec<LiveTurn>> {
    let paths = newest_transcripts(FOLLOWED);
    if paths.is_empty() {
        return Err(Error::BadRequest(
            "no transcript has been written yet".to_owned(),
        ));
    }

    let mut follows = Vec::new();
    let mut recent = Vec::new();
    for path in &paths {
        let mut follow = Follow {
            path: path.clone(),
            offset: 0,
            model: None,
            project: None,
        };
        let all = read_new(&mut follow);
        // The tail is per session: one busy transcript must not push every
        // other session's opening lines out of the window.
        recent.extend(all.iter().rev().take(tail).rev().cloned());
        follows.push(follow);
    }
    recent.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    let already_running = state()
        .lock()
        .map(|mut slot| {
            let running = !slot.is_empty();
            *slot = follows;
            running
        })
        .unwrap_or(false);

    if already_running {
        return Ok(recent);
    }

    let handle = app.clone();
    let mut directories: Vec<PathBuf> = paths
        .iter()
        .map(|path| path.parent().unwrap_or(Path::new(".")).to_path_buf())
        .collect();
    directories.sort();
    directories.dedup();

    std::thread::spawn(move || {
        let (sender, receiver) = mpsc::channel();
        let Ok(mut watcher) = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        }) else {
            return;
        };
        for directory in &directories {
            if watcher
                .watch(directory, RecursiveMode::NonRecursive)
                .is_err()
            {
                return;
            }
        }

        // The watcher only says "something changed"; what changed is read from
        // the files themselves, which is the only thing that can be trusted.
        while receiver.recv().is_ok() {
            let Ok(mut slot) = state().lock() else { break };
            if slot.is_empty() {
                break;
            }
            let mut turns = Vec::new();
            for follow in slot.iter_mut() {
                turns.extend(read_new(follow));
            }
            drop(slot);
            for turn in turns {
                let _ = handle.emit("live:turn", turn);
            }
        }
    });

    Ok(recent)
}

pub fn stop() {
    if let Ok(mut slot) = state().lock() {
        slot.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_without_a_message_is_not_a_turn() {
        assert!(parse(r#"{"type":"summary"}"#, Path::new("x.jsonl")).is_none());
    }

    #[test]
    fn text_and_tool_calls_both_reach_the_window() {
        let line = r#"{"sessionId":"abc","cwd":"C:/p","timestamp":"2026-08-03T09:00:00Z",
            "message":{"role":"assistant","content":[{"type":"text","text":"Working"},
            {"type":"tool_use","name":"Edit"}]}}"#;
        let turn = parse(line, Path::new("x.jsonl")).unwrap();
        assert_eq!(turn.session_id, "abc");
        assert_eq!(turn.role, "assistant");
        assert_eq!(turn.text.as_deref(), Some("Working"));
        assert_eq!(turn.tools, vec!["Edit"]);
        assert_eq!(turn.kind, "code", "a line that used a tool is work");
    }

    #[test]
    fn a_turn_carries_what_the_table_shows() {
        let line = r#"{"sessionId":"abc","cwd":"C:/p","gitBranch":"main","isSidechain":true,
            "timestamp":"2026-08-03T09:00:00Z",
            "message":{"role":"assistant","model":"claude-opus-5",
            "usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":900,
            "cache_creation_input_tokens":7},
            "content":[{"type":"thinking","thinking":"…"},{"type":"text","text":"Done"}]}}"#;
        let turn = parse(line, Path::new("x.jsonl")).unwrap();
        assert_eq!(turn.model.as_deref(), Some("claude-opus-5"));
        assert_eq!(
            (
                turn.input_tokens,
                turn.output_tokens,
                turn.cache_read_tokens,
                turn.cache_write_tokens
            ),
            (10, 5, 900, 7)
        );
        assert_eq!(turn.git_branch.as_deref(), Some("main"));
        assert!(turn.thinking);
        assert!(turn.agent);
        assert_eq!(turn.kind, "chat", "no tool call, so it is conversation");
    }

    #[test]
    fn a_line_that_only_thinks_is_still_a_turn() {
        let line = r#"{"sessionId":"abc","message":{"role":"assistant",
            "content":[{"type":"thinking","thinking":"…"}]}}"#;
        let turn = parse(line, Path::new("x.jsonl")).expect("thinking alone is worth a row");
        assert!(turn.thinking);
        assert!(turn.text.is_none());
    }

    #[test]
    fn an_empty_turn_is_dropped() {
        let line = r#"{"message":{"role":"user","content":[]}}"#;
        assert!(parse(line, Path::new("x.jsonl")).is_none());
    }
}
