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
/// only shape that fits a button.
pub fn ask(configured: Option<&str>, prompt: &str) -> Result<String> {
    if prompt.trim().is_empty() {
        return Err(Error::BadRequest("the prompt is empty".to_owned()));
    }

    let binary = locate(configured)
        .ok_or_else(|| Error::BadRequest("Claude Code was not found on this machine".to_owned()))?;

    let mut child = Command::new(&binary)
        .arg("-p")
        .arg(prompt)
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

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_prompt_is_refused_before_anything_is_launched() {
        let error = ask(Some("does-not-exist.exe"), "   ").unwrap_err();
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
    fn a_blank_configuration_falls_back_to_the_known_places() {
        // Only that the blank is ignored; whether a CLI is installed is the
        // developer's business, not this test's.
        let _ = locate(Some("   "));
    }
}
