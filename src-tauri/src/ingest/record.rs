use serde::Deserialize;
use serde_json::Value;

/// One line of a `.jsonl` transcript.
///
/// Everything is optional and nothing is denied: transcripts carry many fields
/// this app does not care about, and their shapes vary between Claude Code
/// versions. A record that fails to deserialise is counted and skipped rather
/// than aborting the file.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawRecord {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Only `system` records carry one; `compact_boundary` is the one that matters.
    pub subtype: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub uuid: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Option<bool>,
    #[serde(rename = "agentId")]
    pub agent_id: Option<String>,
    /// Some builds nest the agent id one level down.
    pub data: Option<Value>,
    pub message: Option<RawMessage>,
    /// Shape varies — sometimes an object, sometimes a bare string — so it is
    /// read defensively instead of being typed.
    #[serde(rename = "toolUseResult")]
    pub tool_use_result: Option<Value>,
    #[serde(rename = "customTitle")]
    pub custom_title: Option<String>,
    #[serde(rename = "aiTitle")]
    pub ai_title: Option<String>,
    #[serde(rename = "compactMetadata")]
    pub compact_metadata: Option<CompactMetadata>,
}

/// What Claude Code records when it compacts a conversation.
///
/// Read from the `compact_boundary` record rather than guessed from the prefix
/// of the summary message: the record says outright whether the user asked for
/// it or the context window overflowed, and how large the context had grown.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
pub struct CompactMetadata {
    /// `auto` when the context window overflowed, `manual` for `/compact`.
    pub trigger: Option<String>,
    #[serde(rename = "preTokens")]
    pub pre_tokens: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawMessage {
    pub id: Option<String>,
    pub model: Option<String>,
    pub usage: Option<Usage>,
    /// A string for simple turns, an array of blocks otherwise.
    pub content: Option<Value>,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Usage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
}

impl Usage {
    pub fn total(&self) -> i64 {
        self.input_tokens.unwrap_or(0)
            + self.output_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0)
            + self.cache_creation_input_tokens.unwrap_or(0)
    }
}

impl RawRecord {
    /// The agent id, whether it sits at the top level or under `data`.
    pub fn resolved_agent_id(&self) -> Option<String> {
        if let Some(id) = self.agent_id.as_ref().filter(|s| !s.is_empty()) {
            return Some(id.clone());
        }
        self.data
            .as_ref()?
            .get("agentId")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    }

    /// The tools this message called, in order.
    pub fn tool_uses(&self) -> Vec<ToolUse> {
        let Some(Value::Array(blocks)) = self.message.as_ref().and_then(|m| m.content.as_ref())
        else {
            return Vec::new();
        };

        blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
            .filter_map(|block| {
                let name = block.get("name").and_then(Value::as_str)?;
                Some(ToolUse {
                    id: block
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                    name: name.to_owned(),
                    file_path: block.get("input").and_then(target_path),
                })
            })
            .collect()
    }

    /// The outcomes this record reports back, as `(tool_use_id, is_error)`.
    ///
    /// They arrive on a later `user` record than the call, and with parallel
    /// tools not even in the same order, so they are collected on their own and
    /// paired by id rather than positionally.
    pub fn tool_results(&self) -> Vec<(String, bool)> {
        let Some(Value::Array(blocks)) = self.message.as_ref().and_then(|m| m.content.as_ref())
        else {
            return Vec::new();
        };

        blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
            .filter_map(|block| {
                let id = block.get("tool_use_id").and_then(Value::as_str)?;
                if id.is_empty() {
                    return None;
                }
                let failed = block
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                Some((id.to_owned(), failed))
            })
            .collect()
    }

    /// The slash command this record invoked, without its arguments.
    ///
    /// Claude Code wraps it in a `<command-name>` element inside an otherwise
    /// ordinary user message, which is the only place the invocation is named.
    pub fn slash_command(&self) -> Option<String> {
        let text = self.text_content()?;
        let start = text.find("<command-name>")? + "<command-name>".len();
        let end = text[start..].find("</command-name>")? + start;
        let name = text[start..end].split_whitespace().next()?;
        name.starts_with('/').then(|| name.to_owned())
    }

    /// The message's plain text, whether it is a bare string or text blocks.
    fn text_content(&self) -> Option<String> {
        match self.message.as_ref()?.content.as_ref()? {
            Value::String(text) => Some(text.clone()),
            Value::Array(blocks) => {
                let joined: String = blocks
                    .iter()
                    .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|block| block.get("text").and_then(Value::as_str))
                    .collect();
                (!joined.is_empty()).then_some(joined)
            }
            _ => None,
        }
    }
}

/// One tool call, with what it was pointed at.
#[derive(Debug, Clone)]
pub struct ToolUse {
    /// Absent on older transcripts, which is why the result pairing is optional.
    pub id: Option<String>,
    pub name: String,
    pub file_path: Option<String>,
}

/// The file a tool call names, under whichever key that tool happens to use.
fn target_path(input: &Value) -> Option<String> {
    ["file_path", "notebook_path", "path"]
        .iter()
        .find_map(|key| input.get(key).and_then(Value::as_str))
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
}

/// Metadata a subagent dispatch reports back on the parent's `user` record.
#[derive(Debug, Clone)]
pub struct AgentDispatch {
    pub agent_id: String,
    /// The session the dispatch was read from; the record itself does not say.
    pub parent_session_id: Option<String>,
    pub agent_type: Option<String>,
    pub status: Option<String>,
    pub total_tokens: Option<i64>,
    pub total_duration_ms: Option<i64>,
    pub tool_use_count: Option<i64>,
}

impl AgentDispatch {
    pub fn from_value(value: &Value) -> Option<Self> {
        let agent_id = value.get("agentId")?.as_str()?.to_owned();
        if agent_id.is_empty() {
            return None;
        }

        Some(Self {
            agent_id,
            parent_session_id: None,
            agent_type: value
                .get("agentType")
                .and_then(Value::as_str)
                .map(str::to_owned),
            status: value
                .get("status")
                .and_then(Value::as_str)
                .map(str::to_owned),
            total_tokens: value.get("totalTokens").and_then(Value::as_i64),
            total_duration_ms: value.get("totalDurationMs").and_then(Value::as_i64),
            tool_use_count: value.get("totalToolUseCount").and_then(Value::as_i64),
        })
    }
}
