//! Reads a Codex rollout into the same shape a Claude transcript produces.
//!
//! Codex records events rather than messages: a model call is reported by the
//! `token_count` that follows it, the tools it asked for arrive as their own
//! `function_call` lines before it, and their outcomes as `function_call_output`
//! lines after. A turn is therefore assembled from a run of lines instead of
//! read off one.

use super::reader::{FileParse, ParsedTurn, SessionMeta};
use super::record::ToolUse;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A session with no title of its own is labelled with what was asked for, cut
/// to something a table column can hold.
const TOPIC_CHARS: usize = 120;

pub fn parse_file(path: &Path, skip_lines: u64) -> Result<FileParse> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::with_capacity(64 * 1024, file);

    // The head of the file names the session, and an incremental scan reads only
    // the tail. The name is in the file name too, which every pass can see.
    let session_id = session_id_from_path(path);

    let mut result = FileParse::default();
    let mut meta = SessionMeta {
        session_id: session_id.clone(),
        ..SessionMeta::default()
    };

    let mut model: Option<String> = None;
    // Collected as they come and handed to the turn that reports them, which is
    // the next tally: a Codex tool call is its own line, ahead of the usage.
    let mut pending_tools: Vec<ToolUse> = Vec::new();
    // Codex reports a running total, and repeats the last one at the start of
    // every turn. The cumulative figure is what tells a fresh call from a repeat.
    let mut seen_totals: HashSet<i64> = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        result.line_count += 1;

        if result.line_count <= skip_lines || line.trim().is_empty() {
            continue;
        }

        let record: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                result.malformed_lines += 1;
                continue;
            }
        };

        let payload = record.get("payload").unwrap_or(&Value::Null);
        let timestamp = record.get("timestamp").and_then(Value::as_str);

        match record
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "session_meta" => {
                if let Some(cwd) = text(payload.get("cwd")) {
                    meta.cwd = Some(cwd);
                }
                if let Some(branch) = payload.get("git").and_then(|git| text(git.get("branch"))) {
                    meta.git_branch = Some(branch);
                }
            }
            "turn_context" => {
                if let Some(cwd) = text(payload.get("cwd")) {
                    meta.cwd = Some(cwd);
                }
                if let Some(name) = text(payload.get("model")) {
                    model = Some(name);
                }
            }
            "event_msg" => match payload
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default()
            {
                "user_message" => {
                    if meta.topic.is_none()
                        && let Some(message) = text(payload.get("message"))
                    {
                        meta.topic = Some(shorten(&message));
                        meta.topic_source = Some("first-message".to_owned());
                    }
                }
                "token_count" => {
                    if let Some(turn) = build_turn(
                        payload,
                        &session_id,
                        timestamp,
                        model.as_deref(),
                        meta.cwd.as_deref(),
                        &mut seen_totals,
                        &mut pending_tools,
                    ) {
                        result.turns.push(turn);
                    }
                }
                _ => {}
            },
            "response_item" => {
                match payload
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                {
                    "function_call" => {
                        if let Some(name) = text(payload.get("name")) {
                            let arguments = arguments_of(payload);
                            pending_tools.push(ToolUse {
                                id: text(payload.get("call_id")),
                                file_path: target_path(&name, arguments.as_ref()),
                                name,
                            });
                        }
                    }
                    "function_call_output" => {
                        if let Some(id) = text(payload.get("call_id")) {
                            let output = payload.get("output").unwrap_or(&Value::Null);
                            result.tool_results.push((id, failed(output)));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Tools that were asked for in the tail of an unfinished turn have no tally
    // to belong to yet. They are left for the scan that reads the rest.
    result.sessions.insert(session_id, meta);
    Ok(result)
}

/// A rollout is named `rollout-<started>-<session id>.jsonl`.
///
/// The id is the last five dash-separated groups, which is what a UUID is. A
/// file that does not carry one falls back to its own name, so it still groups
/// its turns together rather than losing them.
pub fn session_id_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let parts: Vec<&str> = stem.split('-').collect();
    if parts.len() >= 5 {
        let candidate = parts[parts.len() - 5..].join("-");
        if is_uuid(&candidate) {
            return candidate;
        }
    }
    stem
}

fn is_uuid(value: &str) -> bool {
    let groups: Vec<&str> = value.split('-').collect();
    groups.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&groups)
            .all(|(len, group)| group.len() == *len)
        && value.chars().all(|c| c == '-' || c.is_ascii_hexdigit())
}

/// One model call, from the tally that reports it.
///
/// The tally is repeated at the start of the next turn, so the cumulative total
/// is what says whether this is a call or an echo of the last one. That total
/// also names the turn: it is the one value a repeat carries unchanged, which
/// makes a rescan correct the row instead of adding a second one.
fn build_turn(
    payload: &Value,
    session_id: &str,
    timestamp: Option<&str>,
    model: Option<&str>,
    cwd: Option<&str>,
    seen_totals: &mut HashSet<i64>,
    pending_tools: &mut Vec<ToolUse>,
) -> Option<ParsedTurn> {
    let info = payload.get("info")?;
    let total = number(info.get("total_token_usage")?.get("total_tokens"));
    if total == 0 || !seen_totals.insert(total) {
        return None;
    }

    let last = info.get("last_token_usage")?;
    // Codex counts the cached part inside the input, where this app keeps the
    // two apart: added back together they would charge the cached tokens twice.
    let cached = number(last.get("cached_input_tokens"));
    let input = (number(last.get("input_tokens")) - cached).max(0);
    let output = number(last.get("output_tokens"));

    if input + cached + output == 0 {
        return None;
    }

    Some(ParsedTurn {
        session_id: session_id.to_owned(),
        message_id: Some(format!("codex:{session_id}:{total}")),
        uuid: None,
        timestamp: timestamp?.to_owned(),
        model: model.map(str::to_owned),
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: cached,
        // Codex reports nothing it was charged for writing a cache entry.
        cache_write_tokens: 0,
        cwd: cwd.map(str::to_owned),
        git_branch: None,
        is_subagent: false,
        agent_id: None,
        tools: std::mem::take(pending_tools),
    })
}

/// A call's arguments, which Codex records as JSON inside a string.
fn arguments_of(payload: &Value) -> Option<Value> {
    match payload.get("arguments") {
        Some(Value::String(raw)) => serde_json::from_str(raw).ok(),
        Some(other) => Some(other.clone()),
        None => None,
    }
}

/// The file a call named, under whichever key that tool uses.
///
/// `apply_patch` names its files inside the patch instead of in a field, and it
/// is the one call that changes code — leaving it out would put every edit
/// outside the count of files a session touched.
fn target_path(name: &str, arguments: Option<&Value>) -> Option<String> {
    let arguments = arguments?;

    if name == "apply_patch"
        && let Some(patch) = arguments.get("input").and_then(Value::as_str)
    {
        return patch_target(patch);
    }

    ["path", "file_path", "filepath"]
        .iter()
        .find_map(|key| arguments.get(key).and_then(Value::as_str))
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
}

fn patch_target(patch: &str) -> Option<String> {
    patch.lines().find_map(|line| {
        ["*** Update File: ", "*** Add File: ", "*** Delete File: "]
            .iter()
            .find_map(|marker| line.strip_prefix(marker))
            .map(|path| path.trim().to_owned())
    })
}

/// Whether a call came back as a failure.
///
/// Codex writes the outcome as text for a shell call and as an object for the
/// rest, so both are read. A call the user turned down failed as surely as one
/// that exited non-zero — it did not do what it was asked to.
pub fn failed(output: &Value) -> bool {
    if let Some(success) = output.get("success").and_then(Value::as_bool) {
        return !success;
    }

    let Some(text) = output.as_str() else {
        return false;
    };

    if let Some(rest) = text.strip_prefix("Exit code: ") {
        return rest
            .split_whitespace()
            .next()
            .and_then(|code| code.parse::<i64>().ok())
            .is_some_and(|code| code != 0);
    }

    text.contains("rejected by user")
}

fn number(value: Option<&Value>) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(0)
}

fn text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn shorten(text: &str) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match single_line.char_indices().nth(TOPIC_CHARS) {
        Some((end, _)) => format!("{}…", &single_line[..end]),
        None => single_line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// The file name is the same in every case because it is what names the
    /// session; a directory per test is what keeps them from overwriting each
    /// other while the suite runs in parallel.
    fn rollout(test: &str, lines: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("cua-codex-{}", std::process::id()))
            .join(test);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("rollout-2026-02-11T15-55-01-{SESSION}.jsonl"));
        let mut file = std::fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        path
    }

    const SESSION: &str = "019c4d32-d047-7360-ae88-6d13033e3276";

    fn tally(ts: &str, total: i64, input: i64, cached: i64, output: i64) -> String {
        format!(
            r#"{{"timestamp":"{ts}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"total_tokens":{total}}},"last_token_usage":{{"input_tokens":{input},"cached_input_tokens":{cached},"output_tokens":{output}}}}}}}}}"#
        )
    }

    #[test]
    fn the_session_id_comes_from_the_file_name() {
        // The head of the file names it too, but an incremental scan reads only
        // the tail and would otherwise have nothing to attribute a turn to.
        let path = rollout("name", &[]);
        assert_eq!(session_id_from_path(&path), SESSION);
    }

    #[test]
    fn cached_tokens_are_taken_out_of_the_input() {
        // Codex counts them inside it; this app keeps them apart, and adding
        // them back together would charge the cached half twice.
        let path = rollout(
            "cached",
            &[&tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175)],
        );
        let parsed = parse_file(&path, 0).unwrap();

        assert_eq!(parsed.turns.len(), 1);
        assert_eq!(parsed.turns[0].input_tokens, 948);
        assert_eq!(parsed.turns[0].cache_read_tokens, 7552);
        assert_eq!(parsed.turns[0].output_tokens, 175);
    }

    #[test]
    fn the_tally_repeated_at_the_start_of_the_next_turn_is_not_a_second_call() {
        let repeat = tally("2026-02-11T15:55:39Z", 8675, 8500, 7552, 175);
        let path = rollout(
            "repeat",
            &[
                &tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175),
                &repeat,
                &tally("2026-02-11T15:55:42Z", 17587, 8703, 7552, 209),
            ],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.turns.len(), 2, "three tallies, two calls");
    }

    #[test]
    fn a_turn_is_named_so_a_rescan_corrects_it_instead_of_adding_one() {
        // The repeat carries the same running total and nothing else in common,
        // so the total is what has to name the turn.
        let path = rollout(
            "naming",
            &[&tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175)],
        );
        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(
            parsed.turns[0].message_id.as_deref(),
            Some("codex:019c4d32-d047-7360-ae88-6d13033e3276:8675")
        );
    }

    #[test]
    fn a_call_belongs_to_the_tally_that_follows_it() {
        let call = r#"{"timestamp":"2026-02-11T15:55:07Z","type":"response_item","payload":{"type":"function_call","name":"shell_command","call_id":"call_1","arguments":"{\"command\":\"Get-ChildItem\"}"}}"#;
        let path = rollout(
            "call",
            &[call, &tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175)],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.turns[0].tools.len(), 1);
        assert_eq!(parsed.turns[0].tools[0].name, "shell_command");
        assert_eq!(parsed.turns[0].tools[0].id.as_deref(), Some("call_1"));
    }

    #[test]
    fn a_patch_names_the_file_it_changes() {
        let patch = r#"{"timestamp":"2026-02-11T15:55:07Z","type":"response_item","payload":{"type":"function_call","name":"apply_patch","call_id":"call_2","arguments":"{\"input\":\"*** Begin Patch\\n*** Update File: src/main.rs\\n@@\\n-old\\n+new\\n*** End Patch\"}"}}"#;
        let path = rollout(
            "patch",
            &[patch, &tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175)],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(
            parsed.turns[0].tools[0].file_path.as_deref(),
            Some("src/main.rs"),
            "without this every edit falls outside the files a session touched"
        );
    }

    #[test]
    fn an_outcome_is_read_from_the_exit_code_and_from_a_refusal() {
        let ok = r#"{"timestamp":"2026-02-11T15:55:38Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"Exit code: 0\nWall time: 1.9 seconds"}}"#;
        let failed = r#"{"timestamp":"2026-02-11T15:56:01Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c2","output":"Exit code: 124\nWall time: 10.2 seconds"}}"#;
        let refused = r#"{"timestamp":"2026-02-11T15:56:52Z","type":"response_item","payload":{"type":"function_call_output","call_id":"c3","output":"exec command rejected by user"}}"#;

        let parsed = parse_file(&rollout("outcome", &[ok, failed, refused]), 0).unwrap();
        assert_eq!(
            parsed.tool_results,
            vec![
                ("c1".to_owned(), false),
                ("c2".to_owned(), true),
                ("c3".to_owned(), true),
            ]
        );
    }

    #[test]
    fn the_first_thing_asked_for_becomes_the_title() {
        // Codex writes no title of its own, and a list of untitled sessions is
        // one no one can find anything in.
        let first = r#"{"timestamp":"2026-02-11T15:55:03Z","type":"event_msg","payload":{"type":"user_message","message":"Create a simple c# console app\n"}}"#;
        let later = r#"{"timestamp":"2026-02-11T15:57:03Z","type":"event_msg","payload":{"type":"user_message","message":"now add tests"}}"#;

        let parsed = parse_file(&rollout("title", &[first, later]), 0).unwrap();
        let meta = &parsed.sessions[SESSION];
        assert_eq!(
            meta.topic.as_deref(),
            Some("Create a simple c# console app")
        );
        assert_eq!(meta.topic_source.as_deref(), Some("first-message"));
    }

    #[test]
    fn the_model_is_taken_from_the_context_the_turn_ran_under() {
        let context = r#"{"timestamp":"2026-02-11T15:55:03Z","type":"turn_context","payload":{"cwd":"c:\\Sources\\test","model":"gpt-5.3-codex"}}"#;
        let path = rollout(
            "model",
            &[
                context,
                &tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175),
            ],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.turns[0].model.as_deref(), Some("gpt-5.3-codex"));
        assert_eq!(parsed.turns[0].cwd.as_deref(), Some(r"c:\Sources\test"));
    }

    #[test]
    fn skipping_lines_reads_only_the_tail() {
        let path = rollout(
            "tail",
            &[
                &tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175),
                &tally("2026-02-11T15:55:42Z", 17587, 8703, 7552, 209),
                &tally("2026-02-11T15:56:03Z", 26637, 8949, 8576, 101),
            ],
        );

        let parsed = parse_file(&path, 2).unwrap();
        assert_eq!(
            parsed.line_count, 3,
            "the count still covers the whole file"
        );
        assert_eq!(parsed.turns.len(), 1);
        assert_eq!(parsed.turns[0].cache_read_tokens, 8576);
    }

    #[test]
    fn malformed_lines_are_counted_not_fatal() {
        let path = rollout(
            "malformed",
            &[
                "{not json at all",
                &tally("2026-02-11T15:55:07Z", 8675, 8500, 7552, 175),
            ],
        );

        let parsed = parse_file(&path, 0).unwrap();
        assert_eq!(parsed.malformed_lines, 1);
        assert_eq!(parsed.turns.len(), 1);
    }

    /// Runs against the machine's own rollouts. Ignored for the same reason the
    /// Claude one is: it depends on data no other machine has.
    #[test]
    #[ignore = "reads the developer's own ~/.codex/sessions"]
    fn reads_the_real_rollout_folder() {
        let roots = crate::paths::codex_scan_roots();
        if roots.is_empty() {
            eprintln!("no rollout folder on this machine; nothing to check");
            return;
        }

        for path in super::super::scanner::discover_transcripts(&roots) {
            let parsed = parse_file(&path, 0).unwrap();
            let tokens: i64 = parsed
                .turns
                .iter()
                .map(|t| t.input_tokens + t.output_tokens + t.cache_read_tokens)
                .sum();
            eprintln!(
                "{}: {} turns, {} tools, {} outcomes, {tokens} tokens, model {:?}",
                path.file_name().unwrap().to_string_lossy(),
                parsed.turns.len(),
                parsed.turns.iter().map(|t| t.tools.len()).sum::<usize>(),
                parsed.tool_results.len(),
                parsed.turns.first().and_then(|t| t.model.clone()),
            );
            assert_eq!(parsed.malformed_lines, 0);
        }
    }
}
