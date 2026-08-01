use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Long tool payloads are truncated before they cross to the UI. A single Read
/// result can be hundreds of kilobytes, and a session has thousands of them.
const MAX_PAYLOAD: usize = 4_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub name: String,
    pub input: String,
    pub input_truncated: bool,
    pub result: Option<String>,
    pub result_truncated: bool,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptTurn {
    pub index: usize,
    pub role: String,
    pub timestamp: Option<String>,
    pub text: Option<String>,
    /// Kept separate from `text` so the UI can collapse it by default.
    pub thinking: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPage {
    pub turns: Vec<TranscriptTurn>,
    pub total: usize,
    pub offset: usize,
    pub path: Option<String>,
}

pub fn load(
    conn: &Connection,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<TranscriptPage> {
    let path: Option<String> = conn
        .query_row(
            "SELECT transcript_path FROM sessions WHERE session_id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    let Some(path) = path else {
        return Ok(TranscriptPage {
            turns: Vec::new(),
            total: 0,
            offset,
            path: None,
        });
    };

    let turns = read_turns(Path::new(&path), session_id)?;
    let total = turns.len();
    let window = turns.into_iter().skip(offset).take(limit).collect();

    Ok(TranscriptPage {
        turns: window,
        total,
        offset,
        path: Some(path),
    })
}

/// Rebuilds the conversation from a transcript file.
fn read_turns(path: &Path, session_id: &str) -> Result<Vec<TranscriptTurn>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::with_capacity(64 * 1024, file);

    let mut records: Vec<Value> = Vec::new();
    let mut seen_uuids: HashSet<String> = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        if value.get("sessionId").and_then(Value::as_str) != Some(session_id) {
            continue;
        }

        // A session resumed headlessly and reopened in the CLI rewrites the same
        // line range verbatim; without this every turn appears twice.
        if let Some(uuid) = value.get("uuid").and_then(Value::as_str)
            && !seen_uuids.insert(uuid.to_owned())
        {
            continue;
        }

        records.push(value);
    }

    // With parallel tools and subagents, Claude Code writes each tool_use on its
    // own assistant line and each tool_result on its own user line, often out of
    // order. Pairing against the immediately following message misses most of
    // them, so results are indexed across the whole session first.
    let mut results_by_id: HashMap<String, (String, bool)> = HashMap::new();
    for record in &records {
        for block in content_blocks(record) {
            if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                continue;
            }
            let Some(id) = block
                .get("tool_use_id")
                .or_else(|| block.get("toolUseId"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            results_by_id.entry(id.to_owned()).or_insert_with(|| {
                (
                    stringify(block.get("content")),
                    block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )
            });
        }
    }

    let mut turns: Vec<TranscriptTurn> = Vec::new();

    for record in &records {
        let kind = record.get("type").and_then(Value::as_str).unwrap_or("");
        if kind != "user" && kind != "assistant" {
            continue;
        }

        let blocks = content_blocks(record);

        // A user message carrying nothing but tool results is the harness
        // reporting back, not the person saying something. Showing it as a user
        // turn would put words in their mouth.
        let only_tool_results = !blocks.is_empty()
            && blocks
                .iter()
                .all(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"));
        if kind == "user" && only_tool_results {
            continue;
        }

        let text = join_blocks(&blocks, "text");
        let thinking = join_blocks(&blocks, "thinking");
        let tool_calls = collect_tool_calls(&blocks, &results_by_id);

        if text.is_none() && thinking.is_none() && tool_calls.is_empty() {
            continue;
        }

        turns.push(TranscriptTurn {
            index: turns.len(),
            role: kind.to_owned(),
            timestamp: record
                .get("timestamp")
                .and_then(Value::as_str)
                .map(str::to_owned),
            text,
            thinking,
            tool_calls,
        });
    }

    Ok(turns)
}

/// `message.content` is an array of blocks, or a bare string for simple turns.
fn content_blocks(record: &Value) -> Vec<&Value> {
    match record.get("message").and_then(|m| m.get("content")) {
        Some(Value::Array(blocks)) => blocks.iter().collect(),
        _ => Vec::new(),
    }
}

fn join_blocks(blocks: &[&Value], kind: &str) -> Option<String> {
    let field = if kind == "thinking" { "thinking" } else { "text" };
    let joined: Vec<&str> = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some(kind))
        .filter_map(|b| b.get(field).and_then(Value::as_str))
        .collect();

    if joined.is_empty() {
        return None;
    }
    Some(joined.join("\n\n"))
}

fn collect_tool_calls(
    blocks: &[&Value],
    results: &HashMap<String, (String, bool)>,
) -> Vec<ToolCall> {
    blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"))
        .map(|block| {
            let id = block.get("id").and_then(Value::as_str).unwrap_or_default();
            let (input, input_truncated) = truncate(stringify(block.get("input")));

            let (result, result_truncated, is_error) = match results.get(id) {
                Some((raw, is_error)) => {
                    let (text, truncated) = truncate(raw.clone());
                    (Some(text), truncated, *is_error)
                }
                None => (None, false, false),
            };

            ToolCall {
                name: block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
                input,
                input_truncated,
                result,
                result_truncated,
                is_error,
            }
        })
        .collect()
}

/// Tool payloads are sometimes a string, sometimes structured. Both are shown as
/// text; pretty-printing keeps structured input readable.
fn stringify(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item.get("text").and_then(Value::as_str) {
                Some(text) => text.to_owned(),
                None => item.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}

fn truncate(text: String) -> (String, bool) {
    if text.len() <= MAX_PAYLOAD {
        return (text, false);
    }
    // Cut on a char boundary; the payload can be arbitrary UTF-8.
    let mut end = MAX_PAYLOAD;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn transcript(name: &str, lines: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cua-transcript-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.jsonl"));
        let mut file = std::fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    #[test]
    fn a_tool_result_finds_its_call_across_the_whole_session() {
        // The result arrives three lines later, after an unrelated turn. Pairing
        // against the next message would miss it.
        let path = transcript(
            "parallel",
            &[
                r#"{"type":"assistant","sessionId":"s1","uuid":"a","message":{"content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file":"a.rs"}}]}}"#,
                r#"{"type":"assistant","sessionId":"s1","uuid":"b","message":{"content":[{"type":"text","text":"meanwhile"}]}}"#,
                r#"{"type":"user","sessionId":"s1","uuid":"c","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"file contents"}]}}"#,
            ],
        );

        let turns = read_turns(&path, "s1").unwrap();
        assert_eq!(turns.len(), 2, "the tool-result-only user turn is absorbed");
        assert_eq!(turns[0].tool_calls.len(), 1);
        assert_eq!(turns[0].tool_calls[0].result.as_deref(), Some("file contents"));
    }

    #[test]
    fn a_user_turn_that_is_only_tool_results_is_not_shown() {
        let path = transcript(
            "absorbed",
            &[
                r#"{"type":"user","sessionId":"s1","uuid":"a","message":{"content":[{"type":"text","text":"do the thing"}]}}"#,
                r#"{"type":"assistant","sessionId":"s1","uuid":"b","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"cmd":"ls"}}]}}"#,
                r#"{"type":"user","sessionId":"s1","uuid":"c","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"out"}]}}"#,
            ],
        );

        let turns = read_turns(&path, "s1").unwrap();
        let user_turns: Vec<_> = turns.iter().filter(|t| t.role == "user").collect();
        assert_eq!(user_turns.len(), 1);
        assert_eq!(user_turns[0].text.as_deref(), Some("do the thing"));
    }

    #[test]
    fn thinking_is_kept_apart_from_the_answer() {
        let path = transcript(
            "thinking",
            &[
                r#"{"type":"assistant","sessionId":"s1","uuid":"a","message":{"content":[{"type":"thinking","thinking":"weighing it up"},{"type":"text","text":"the answer"}]}}"#,
            ],
        );

        let turns = read_turns(&path, "s1").unwrap();
        assert_eq!(turns[0].thinking.as_deref(), Some("weighing it up"));
        assert_eq!(turns[0].text.as_deref(), Some("the answer"));
    }

    #[test]
    fn a_repeated_uuid_is_shown_once() {
        let line = r#"{"type":"assistant","sessionId":"s1","uuid":"same","message":{"content":[{"type":"text","text":"once"}]}}"#;
        let path = transcript("dup", &[line, line]);
        assert_eq!(read_turns(&path, "s1").unwrap().len(), 1);
    }

    #[test]
    fn turns_from_another_session_in_the_same_file_are_left_out() {
        let path = transcript(
            "mixed",
            &[
                r#"{"type":"assistant","sessionId":"s1","uuid":"a","message":{"content":[{"type":"text","text":"mine"}]}}"#,
                r#"{"type":"assistant","sessionId":"s2","uuid":"b","message":{"content":[{"type":"text","text":"theirs"}]}}"#,
            ],
        );

        let turns = read_turns(&path, "s1").unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].text.as_deref(), Some("mine"));
    }

    #[test]
    fn an_oversized_payload_is_cut_on_a_char_boundary() {
        let (text, truncated) = truncate("ä".repeat(MAX_PAYLOAD));
        assert!(truncated);
        assert!(text.len() <= MAX_PAYLOAD);
        // The point of the boundary walk: this would panic on a byte slice.
        assert!(text.chars().all(|c| c == 'ä'));
    }
}
