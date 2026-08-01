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

    /// The ordered names of the tools this message called.
    pub fn tool_uses(&self) -> Vec<String> {
        let Some(Value::Array(blocks)) = self.message.as_ref().and_then(|m| m.content.as_ref())
        else {
            return Vec::new();
        };

        blocks
            .iter()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
            .filter_map(|block| block.get("name").and_then(Value::as_str))
            .map(str::to_owned)
            .collect()
    }
}

/// Metadata a subagent dispatch reports back on the parent's `user` record.
#[derive(Debug, Clone)]
pub struct AgentDispatch {
    pub agent_id: String,
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
