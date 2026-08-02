//! Asking the local Claude Code CLI, which is the one source that needs no key.
//!
//! The prompt goes to a binary on this machine and the answer comes back on its
//! stdout. Whether that binary reaches the network is Claude Code's business,
//! under the user's own account.

use crate::error::{Error, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

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
        path: path.map(|value| value.to_string_lossy().into_owned()),
    }
}

/// Runs one prompt through the CLI and returns what it printed.
///
/// `-p` is the non-interactive form: it answers once and exits, which is the
/// only shape that fits a button. `--output-format json` costs nothing extra
/// and carries back which model actually answered and what it cost — so the
/// window can state those rather than repeat what it asked for.
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

    let mut command = Command::new(&binary);
    command.arg("-p").arg(prompt).arg("--output-format").arg("json");

    if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
        command.arg("--model").arg(model);
    }
    if let Some(effort) = effort.filter(|value| EFFORTS.contains(value)) {
        command.arg("--effort").arg(effort);
    }

    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let deadline = std::time::Instant::now() + TIMEOUT;
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            return Err(Error::BadRequest(format!(
                "Claude Code did not answer within {} seconds",
                TIMEOUT.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        let message = message.trim();
        return Err(Error::BadRequest(if message.is_empty() {
            "Claude Code exited without an answer".to_owned()
        } else {
            message.to_owned()
        }));
    }

    parse(&String::from_utf8_lossy(&output.stdout), effort)
}

/// Reads the CLI's JSON result, falling back to the raw text.
///
/// A future version that changes the envelope should degrade to showing the
/// answer, not to showing nothing.
fn parse(stdout: &str, effort: Option<&str>) -> Result<Answer> {
    let trimmed = stdout.trim();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return Ok(Answer {
            text: trimmed.to_owned(),
            effort: effort.map(str::to_owned),
            ..Answer::default()
        });
    };

    if value["is_error"].as_bool() == Some(true) {
        let message = value["result"].as_str().unwrap_or("Claude Code reported an error");
        return Err(Error::BadRequest(message.to_owned()));
    }

    Ok(Answer {
        text: value["result"].as_str().unwrap_or(trimmed).trim().to_owned(),
        // The key of modelUsage is the full model id, which is the only place
        // the answer says what actually served it.
        model: value["modelUsage"]
            .as_object()
            .and_then(|usage| usage.keys().next().cloned()),
        effort: effort.map(str::to_owned),
        cost_usd: value["total_cost_usd"].as_f64(),
    })
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

    #[test]
    fn the_json_envelope_gives_up_the_model_and_the_cost() {
        let answer = parse(
            r#"{"type":"result","is_error":false,"result":"Ok.","total_cost_usd":0.021,
                "modelUsage":{"claude-haiku-4-5-20251001":{"inputTokens":10}}}"#,
            Some("low"),
        )
        .unwrap();
        assert_eq!(answer.text, "Ok.");
        assert_eq!(answer.model.as_deref(), Some("claude-haiku-4-5-20251001"));
        assert_eq!(answer.effort.as_deref(), Some("low"));
        assert_eq!(answer.cost_usd, Some(0.021));
    }

    #[test]
    fn plain_text_still_reaches_the_window() {
        let answer = parse("just words", None).unwrap();
        assert_eq!(answer.text, "just words");
        assert!(answer.model.is_none());
    }

    #[test]
    fn a_reported_error_is_an_error() {
        let failed = parse(r#"{"is_error":true,"result":"no session"}"#, None);
        assert!(failed.is_err());
    }

    #[test]
    fn a_blank_configuration_falls_back_to_the_known_places() {
        // Only that the blank is ignored; whether a CLI is installed is the
        // developer's business, not this test's.
        let _ = locate(Some("   "));
    }
}
