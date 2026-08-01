use super::record::{AgentDispatch, RawRecord};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// One assistant response, after deduplication.
#[derive(Debug, Clone)]
pub struct ParsedTurn {
    pub session_id: String,
    pub message_id: Option<String>,
    pub uuid: Option<String>,
    pub timestamp: String,
    pub model: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub is_subagent: bool,
    pub agent_id: Option<String>,
    pub tools: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct SessionMeta {
    pub session_id: String,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub topic: Option<String>,
    /// `custom-title` outranks `ai-title`; recorded so a later scan does not
    /// let an inferred title overwrite one the user wrote.
    pub topic_source: Option<String>,
    pub has_subagents: bool,
}

#[derive(Debug, Default)]
pub struct FileParse {
    pub sessions: HashMap<String, SessionMeta>,
    pub turns: Vec<ParsedTurn>,
    pub agents: Vec<AgentDispatch>,
    pub line_count: u64,
    /// Lines that were not valid JSON, or whose shape did not fit. Surfaced
    /// rather than swallowed: silently dropping transcript lines would show up
    /// as quietly wrong totals.
    pub malformed_lines: u64,
}

/// Reads one transcript file, starting after `skip_lines`.
///
/// Skipping is how an append-only transcript stays cheap to rescan: the caller
/// remembers how many lines it consumed last time and only the tail is parsed.
pub fn parse_file(path: &Path, skip_lines: u64) -> Result<FileParse> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let reader = BufReader::with_capacity(64 * 1024, file);

    // A transcript under a `subagents/` directory is a subagent's own log; every
    // line in it belongs to that agent even when the flag is missing.
    let path_marks_subagent = path
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("subagents"));

    let mut result = FileParse::default();

    // Claude Code writes several streaming records per API response and only the
    // last carries the final usage tally, so a later record for the same
    // message.id replaces the earlier one.
    let mut by_message_id: HashMap<String, ParsedTurn> = HashMap::new();
    // Turns with no message.id cannot be attributed to a tool call; they are the
    // reasoning bucket and are kept as they come.
    let mut anonymous_turns: Vec<ParsedTurn> = Vec::new();

    // A session resumed headlessly and then reopened in the CLI rewrites the
    // same line range with identical uuids. Without this every turn in that
    // range would be counted twice.
    let mut seen_uuids: HashSet<String> = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        result.line_count += 1;

        if result.line_count <= skip_lines || line.trim().is_empty() {
            continue;
        }

        let record: RawRecord = match serde_json::from_str(&line) {
            Ok(record) => record,
            Err(_) => {
                result.malformed_lines += 1;
                continue;
            }
        };

        let Some(session_id) = record.session_id.clone().filter(|s| !s.is_empty()) else {
            continue;
        };

        if let Some(uuid) = record.uuid.as_ref()
            && !seen_uuids.insert(uuid.clone())
        {
            continue;
        }

        let kind = record.kind.as_deref().unwrap_or_default();
        let is_subagent =
            path_marks_subagent || record.is_sidechain == Some(true) || record.agent_id.is_some();

        let meta = result
            .sessions
            .entry(session_id.clone())
            .or_insert_with(|| SessionMeta {
                session_id: session_id.clone(),
                ..SessionMeta::default()
            });
        meta.has_subagents |= is_subagent;
        if record.cwd.is_some() {
            meta.cwd = record.cwd.clone();
        }
        if record.git_branch.is_some() {
            meta.git_branch = record.git_branch.clone();
        }

        match kind {
            "custom-title" | "ai-title" => {
                apply_title(meta, kind, &record);
            }
            "user" => {
                if let Some(value) = record.tool_use_result.as_ref()
                    && let Some(dispatch) = AgentDispatch::from_value(value)
                {
                    result.agents.push(dispatch);
                }
            }
            "assistant" => {
                if let Some(turn) = build_turn(&record, session_id, is_subagent) {
                    match turn.message_id.clone() {
                        Some(id) => {
                            by_message_id.insert(id, turn);
                        }
                        None => anonymous_turns.push(turn),
                    }
                }
            }
            _ => {}
        }
    }

    result.turns = by_message_id.into_values().chain(anonymous_turns).collect();
    result.turns.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    Ok(result)
}

/// A user-written title always beats an inferred one, and a title that is
/// already set is never replaced by another of the same rank.
fn apply_title(meta: &mut SessionMeta, kind: &str, record: &RawRecord) {
    let (title, rank) = match kind {
        "custom-title" => (record.custom_title.as_ref(), 2),
        _ => (record.ai_title.as_ref(), 1),
    };

    let Some(title) = title.filter(|t| !t.trim().is_empty()) else {
        return;
    };

    let current_rank = match meta.topic_source.as_deref() {
        Some("custom-title") => 2,
        Some("ai-title") => 1,
        _ => 0,
    };

    if rank >= current_rank {
        meta.topic = Some(title.clone());
        meta.topic_source = Some(kind.to_owned());
    }
}

fn build_turn(record: &RawRecord, session_id: String, is_subagent: bool) -> Option<ParsedTurn> {
    let message = record.message.as_ref()?;
    let usage = message.usage.unwrap_or_default();

    // Streaming artefacts carry no usage at all. Keeping them would inflate the
    // turn count with rows that contribute nothing.
    if usage.total() == 0 {
        return None;
    }

    let timestamp = record.timestamp.clone()?;
    let tools = record.tool_uses();

    Some(ParsedTurn {
        session_id,
        message_id: message.id.clone().filter(|s| !s.is_empty()),
        uuid: record.uuid.clone(),
        timestamp,
        model: message.model.clone(),
        input_tokens: usage.input_tokens.unwrap_or(0),
        output_tokens: usage.output_tokens.unwrap_or(0),
        cache_read_tokens: usage.cache_read_input_tokens.unwrap_or(0),
        cache_write_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
        cwd: record.cwd.clone(),
        git_branch: record.git_branch.clone(),
        is_subagent,
        agent_id: record.resolved_agent_id(),
        tools,
    })
}
