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
    pub role: String,
    pub timestamp: Option<String>,
    pub text: Option<String>,
    /// The tools this line called, by name, in order.
    pub tools: Vec<String>,
}

/// The file being followed and how far it has been read.
struct Follow {
    path: PathBuf,
    offset: u64,
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

/// The most recently written transcript under the scan roots.
/// The most recently written transcripts under the scan roots, newest first.
pub fn newest_transcripts(limit: usize) -> Vec<PathBuf> {
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();

    for root in crate::paths::default_scan_roots() {
        for entry in walkdir::WalkDir::new(&root)
            .max_depth(3)
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
                    _ => {}
                }
            }
        }
        _ => {}
    }

    if text.trim().is_empty() && tools.is_empty() {
        return None;
    }

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
        tools,
    })
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
                if let Some(turn) = parse(&line, &follow.path) {
                    turns.push(turn);
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
    }

    #[test]
    fn an_empty_turn_is_dropped() {
        let line = r#"{"message":{"role":"user","content":[]}}"#;
        assert!(parse(line, Path::new("x.jsonl")).is_none());
    }
}
