//! Asking the local Claude Code CLI, which is the one source that needs no key.
//!
//! The prompt goes through the Agent SDK to a binary on this machine, and the
//! answer comes back as typed messages rather than as JSON to be guessed at.
//! Whether that binary reaches the network is Claude Code's business, under the
//! user's own account.

use crate::error::{Error, Result};
use claude_agent_sdk_rs::{ClaudeAgentOptions, ClaudeClient, ContentBlock, Message};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tokio_stream::StreamExt;

/// How long a single question may take before it is abandoned.
///
/// A CLI that is waiting for a login prompt would otherwise hang the button
/// forever, and the window has no way to answer it.
const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub found: bool,
    pub path: Option<String>,
    /// What `claude --version` answers, so the window can name what it found.
    pub version: Option<String>,
}

/// The effort levels the CLI accepts, in the order it lists them.
pub const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// The model aliases worth offering. An empty choice leaves the CLI on
/// whatever the user configured for it, which is the honest default.
pub const MODELS: &[&str] = &["opus", "sonnet", "haiku", "fable"];

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub text: String,
    /// What the CLI reports it actually used, not what was asked for.
    pub model: Option<String>,
    pub effort: Option<String>,
    /// What this one question cost, as the CLI accounts for it.
    pub cost_usd: Option<f64>,
}

/// Where Claude Code installs itself, in the order worth trying.
fn candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local/bin/claude.exe"));
        paths.push(home.join(".local/bin/claude"));
        paths.push(home.join("AppData/Roaming/npm/claude.cmd"));
    }
    paths
}

/// The configured binary, or the first one that exists in a known place.
pub fn locate(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured.filter(|value| !value.trim().is_empty()) {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    candidates().into_iter().find(|path| path.is_file())
}

pub fn status(configured: Option<&str>) -> CliStatus {
    let path = locate(configured);
    CliStatus {
        found: path.is_some(),
        version: path.as_deref().and_then(version_of),
        path: path.map(|value| value.to_string_lossy().into_owned()),
    }
}

/// The version string the binary reports, or none when it will not say.
///
/// Asked of the binary the user configured rather than of whatever `claude`
/// happens to be on PATH — those are not always the same install.
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

/// Runs one prompt through the CLI and returns what it answered.
///
/// `max_turns: 1` is the shape that fits a button: it answers once and stops.
/// The SDK spawns the same binary the user configured — `cli_path` rather than
/// whatever `claude` is on PATH — and `--effort` rides along as an extra
/// argument, since the options struct has no field of its own for it.
pub fn ask(
    configured: Option<&str>,
    prompt: &str,
    model: Option<&str>,
    effort: Option<&str>,
) -> Result<Answer> {
    if prompt.trim().is_empty() {
        return Err(Error::BadRequest("the prompt is empty".to_owned()));
    }

    let binary = locate(configured)
        .ok_or_else(|| Error::BadRequest("Claude Code was not found on this machine".to_owned()))?;

    let mut options = ClaudeAgentOptions {
        max_turns: Some(1),
        cli_path: Some(binary),
        model: model
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned),
        ..ClaudeAgentOptions::default()
    };
    if let Some(effort) = effort.filter(|value| EFFORTS.contains(value)) {
        options
            .extra_args
            .insert("effort".to_owned(), Some(effort.to_owned()));
    }

    let prompt = prompt.to_owned();
    let messages = tauri::async_runtime::block_on(async move {
        // A CLI waiting for a login prompt would otherwise hang the button
        // forever, and the window has no way to answer it.
        tokio::time::timeout(TIMEOUT, drain(prompt, options))
            .await
            .unwrap_or_else(|_| {
                Err(Error::BadRequest(format!(
                    "Claude Code did not answer within {} seconds",
                    TIMEOUT.as_secs()
                )))
            })
    })?;

    collect(&messages, effort)
}

/// Reads the run to its end, keeping only the messages this build understands.
///
/// Two SDK entry points are ruled out for this. The one-shot `query` fails the
/// whole run on the first message it cannot parse, and the CLI emits kinds this
/// version has not caught up with — a `rate_limit_event` would cost the user
/// their answer. `query_stream` drops its transport with the answer still in
/// flight, so it usually returns nothing but the opening system message. A
/// client owns its transport for as long as it lives, which is what makes the
/// run survive to its result.
async fn drain(prompt: String, options: ClaudeAgentOptions) -> Result<Vec<Message>> {
    let mut client = ClaudeClient::new(options);
    client
        .connect()
        .await
        .map_err(|cause| Error::BadRequest(cause.to_string()))?;
    client
        .query(prompt)
        .await
        .map_err(|cause| Error::BadRequest(cause.to_string()))?;

    let mut messages = Vec::new();
    {
        let mut stream = client.receive_response();
        while let Some(message) = stream.next().await {
            match message {
                Ok(message) => messages.push(message),
                Err(cause) => log::debug!("skipping a message the SDK cannot read: {cause}"),
            }
        }
    }
    let _ = client.disconnect().await;
    Ok(messages)
}

/// Folds the message stream into the one answer the window shows.
///
/// The assistant messages carry the text and the model that actually served
/// it; the closing result message carries the cost. Both are read from typed
/// fields rather than from a JSON envelope that a future version may reshape.
fn collect(messages: &[Message], effort: Option<&str>) -> Result<Answer> {
    let mut answer = Answer {
        effort: effort.map(str::to_owned),
        ..Answer::default()
    };
    let mut text = String::new();

    for message in messages {
        match message {
            Message::Assistant(assistant) => {
                if assistant.message.model.is_some() {
                    answer.model = assistant.message.model.clone();
                }
                for block in &assistant.message.content {
                    if let ContentBlock::Text(part) = block {
                        text.push_str(&part.text);
                    }
                }
            }
            Message::Result(result) => {
                if result.is_error {
                    let message = result
                        .result
                        .clone()
                        .unwrap_or_else(|| "Claude Code reported an error".to_owned());
                    return Err(Error::BadRequest(message));
                }
                answer.cost_usd = result.total_cost_usd;
                // The result text is the whole answer; the assistant blocks are
                // only used when the run ends without one.
                if let Some(final_text) = result.result.as_deref().filter(|value| !value.is_empty())
                {
                    text = final_text.to_owned();
                }
            }
            _ => {}
        }
    }

    answer.text = text.trim().to_owned();
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_prompt_is_refused_before_anything_is_launched() {
        let error = ask(Some("does-not-exist.exe"), "   ", None, None).unwrap_err();
        assert!(matches!(error, Error::BadRequest(_)));
    }

    #[test]
    fn a_configured_path_that_is_not_a_file_counts_as_missing() {
        assert!(locate(Some("C:/nowhere/claude.exe")).is_none());
        let status = status(Some("C:/nowhere/claude.exe"));
        assert!(!status.found);
        assert!(status.path.is_none());
    }

    fn assistant(text: &str, model: Option<&str>) -> Message {
        Message::Assistant(claude_agent_sdk_rs::AssistantMessage {
            message: claude_agent_sdk_rs::AssistantMessageInner {
                content: vec![ContentBlock::Text(claude_agent_sdk_rs::TextBlock {
                    text: text.to_owned(),
                })],
                model: model.map(str::to_owned),
                id: None,
                stop_reason: None,
                usage: None,
                error: None,
            },
            parent_tool_use_id: None,
            session_id: None,
            uuid: None,
        })
    }

    fn result(text: Option<&str>, failed: bool, cost: Option<f64>) -> Message {
        Message::Result(claude_agent_sdk_rs::ResultMessage {
            subtype: "success".to_owned(),
            duration_ms: 1,
            duration_api_ms: 1,
            is_error: failed,
            num_turns: 1,
            session_id: "s".to_owned(),
            total_cost_usd: cost,
            usage: None,
            result: text.map(str::to_owned),
            structured_output: None,
        })
    }

    #[test]
    fn the_messages_give_up_the_model_and_the_cost() {
        let answer = collect(
            &[
                assistant("Ok.", Some("claude-haiku-4-5-20251001")),
                result(Some("Ok."), false, Some(0.021)),
            ],
            Some("low"),
        )
        .unwrap();
        assert_eq!(answer.text, "Ok.");
        assert_eq!(answer.model.as_deref(), Some("claude-haiku-4-5-20251001"));
        assert_eq!(answer.effort.as_deref(), Some("low"));
        assert_eq!(answer.cost_usd, Some(0.021));
    }

    #[test]
    fn a_run_without_a_result_still_reaches_the_window() {
        let answer = collect(&[assistant("just words", None)], None).unwrap();
        assert_eq!(answer.text, "just words");
        assert!(answer.model.is_none());
    }

    #[test]
    fn a_reported_error_is_an_error() {
        let failed = collect(&[result(Some("no session"), true, None)], None);
        assert!(failed.is_err());
    }

    #[test]
    #[ignore = "spends a turn of the developer's own Claude Code account"]
    fn answers_from_the_real_cli() {
        let answer = ask(None, "Reply with the single word: pong", None, Some("low")).unwrap();
        assert!(!answer.text.is_empty());
        assert!(answer.model.is_some());
    }

    #[test]
    fn a_blank_configuration_falls_back_to_the_known_places() {
        // Only that the blank is ignored; whether a CLI is installed is the
        // developer's business, not this test's.
        let _ = locate(Some("   "));
    }
}
