//! Coding sessions this app runs and steers.
//!
//! A session is one long-lived SDK client with the `claude` CLI behind it — the
//! same binary the user is signed in to, so a run costs the subscription and no
//! API key is involved. The window sends prompts in, messages come back out as
//! events, and every tool the session wants to use passes through here first.
//!
//! Permission is the point of the whole module. The default is to ask: the run
//! stops at each tool call and waits for an answer from the window. Nothing in
//! here may decide on its own that a command is harmless.

use crate::error::{Error, Result};
use claude_agent_sdk_rs::{
    ClaudeAgentOptions, ClaudeClient, ContentBlock, Message, PermissionResult,
    PermissionResultAllow, PermissionResultDeny,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

/// How far a session may go without being asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Rule {
    /// Every tool call waits for the window. The only default worth having.
    Ask,
    /// Reading is free; anything that writes or runs still waits.
    ReadsFree,
}

/// What the window asks for when it starts a run.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub project: String,
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    /// The binary to drive, when it is not the one on the path.
    #[serde(default)]
    pub cli_path: Option<String>,
    pub rule: Rule,
    /// Ends the run when the spend passes this, in US dollars.
    #[serde(default)]
    pub budget_usd: Option<f64>,
    /// A task this run belongs to, carried back on every event.
    #[serde(default)]
    pub task_id: Option<i64>,
}

/// One running session, as the window lists them.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunInfo {
    pub id: String,
    pub project: String,
    pub task_id: Option<i64>,
    pub rule: Rule,
}

/// A tool waiting for an answer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ask {
    pub id: String,
    pub run: String,
    pub tool: String,
    /// What the tool was called with, as it came, for the window to show.
    pub input: serde_json::Value,
}

/// One line of a run, as the window shows it.
///
/// The SDK's message tree is flattened here rather than in the window: the
/// shapes belong to a dependency that will change, and a console needs four
/// things — who said it, what it was, the text, and whether it went wrong.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub run: String,
    /// `text`, `thinking`, `tool`, `result` or `system`.
    pub kind: String,
    pub text: String,
    /// The tool's name, for the lines that are a tool.
    pub tool: Option<String>,
    /// What the CLI says the turn cost; under a subscription this is what it
    /// would have cost at API rates, not a charge.
    pub cost_usd: Option<f64>,
}

/// Flattens one message into the lines a console shows.
fn lines_of(run: &str, message: &Message) -> Vec<Line> {
    let line = |kind: &str, text: String, tool: Option<String>, cost: Option<f64>| Line {
        run: run.to_owned(),
        kind: kind.to_owned(),
        text,
        tool,
        cost_usd: cost,
    };

    match message {
        Message::Assistant(assistant) => assistant
            .message
            .content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(text) => Some(line("text", text.text.clone(), None, None)),
                ContentBlock::Thinking(thinking) => {
                    Some(line("thinking", thinking.thinking.clone(), None, None))
                }
                ContentBlock::ToolUse(call) => Some(line(
                    "tool",
                    call.input.to_string(),
                    Some(call.name.clone()),
                    None,
                )),
                _ => None,
            })
            .collect(),
        Message::Result(result) => vec![line(
            "result",
            result.result.clone().unwrap_or_default(),
            None,
            result.total_cost_usd,
        )],
        Message::System(system) => vec![line("system", system.subtype.clone(), None, None)],
        // Stream events and the control protocol are noise in a console.
        _ => Vec::new(),
    }
}

enum Step {
    Prompt(String),
    Interrupt,
    Stop,
}

struct Run {
    tx: mpsc::UnboundedSender<Step>,
    info: RunInfo,
}

fn runs() -> &'static Mutex<HashMap<String, Run>> {
    static RUNS: OnceLock<Mutex<HashMap<String, Run>>> = OnceLock::new();
    RUNS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn asks() -> &'static Mutex<HashMap<String, oneshot::Sender<bool>>> {
    static ASKS: OnceLock<Mutex<HashMap<String, oneshot::Sender<bool>>>> = OnceLock::new();
    ASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Tools that only look. Everything else waits for a person, whatever the rule.
const READ_ONLY: [&str; 5] = ["Read", "Grep", "Glob", "NotebookRead", "WebFetch"];

pub fn list() -> Vec<RunInfo> {
    runs()
        .lock()
        .map(|runs| runs.values().map(|run| run.info.clone()).collect())
        .unwrap_or_default()
}

/// Answers a tool that was waiting. Unknown ids are ignored: a run that ended
/// while the dialog was open must not make the window's reply an error.
pub fn answer(id: &str, allow: bool) {
    let sender = asks().lock().ok().and_then(|mut asks| asks.remove(id));
    if let Some(sender) = sender {
        let _ = sender.send(allow);
    }
}

fn step(id: &str, step: Step) -> Result<()> {
    let runs = runs()
        .lock()
        .map_err(|_| Error::BadRequest("the session registry is wedged".to_owned()))?;
    let run = runs
        .get(id)
        .ok_or_else(|| Error::BadRequest(format!("no session '{id}' is running")))?;
    run.tx
        .send(step)
        .map_err(|_| Error::BadRequest("the session has already ended".to_owned()))
}

pub fn send(id: &str, prompt: &str) -> Result<()> {
    step(id, Step::Prompt(prompt.to_owned()))
}

pub fn interrupt(id: &str) -> Result<()> {
    step(id, Step::Interrupt)
}

pub fn stop(id: &str) -> Result<()> {
    step(id, Step::Stop)
}

/// Starts a session and returns the id the window steers it by.
///
/// The id is the app's own, not the CLI's: a run has to be addressable before
/// the CLI has said anything about which session it opened.
pub fn start<R: Runtime>(app: &AppHandle<R>, request: RunRequest) -> Result<RunInfo> {
    let id = format!("run-{}", chrono::Utc::now().timestamp_millis());
    let info = RunInfo {
        id: id.clone(),
        project: request.project.clone(),
        task_id: request.task_id,
        rule: request.rule,
    };

    let (tx, rx) = mpsc::unbounded_channel();
    runs()
        .lock()
        .map_err(|_| Error::BadRequest("the session registry is wedged".to_owned()))?
        .insert(
            id.clone(),
            Run {
                tx: tx.clone(),
                info: info.clone(),
            },
        );

    // The first prompt is queued before anything runs, so the task has work the
    // moment it connects.
    let _ = tx.send(Step::Prompt(request.prompt.clone()));

    let handle = app.clone();
    let run_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let outcome = drive(handle.clone(), run_id.clone(), request, rx).await;
        if let Err(error) = &outcome {
            let _ = handle.emit(
                "session:state",
                serde_json::json!({ "id": run_id, "state": "failed", "message": error.to_string() }),
            );
        }
        if let Ok(mut runs) = runs().lock() {
            runs.remove(&run_id);
        }
        let _ = handle.emit(
            "session:state",
            serde_json::json!({ "id": run_id, "state": "ended" }),
        );
    });

    Ok(info)
}

/// Builds the options for one run.
///
/// `permission_mode` stays at its default: the decision is made by the callback
/// below, and a mode that pre-approves anything would go around it.
fn options<R: Runtime>(app: &AppHandle<R>, id: &str, request: &RunRequest) -> ClaudeAgentOptions {
    let mut options = ClaudeAgentOptions {
        cwd: Some(std::path::PathBuf::from(&request.project)),
        model: request.model.clone(),
        max_budget_usd: request.budget_usd,
        ..Default::default()
    };
    if let Some(path) = &request.cli_path {
        if !path.trim().is_empty() {
            options.cli_path = Some(std::path::PathBuf::from(path));
        }
    }

    let handle = app.clone();
    let run = id.to_owned();
    let rule = request.rule;
    options.can_use_tool = Some(std::sync::Arc::new(move |tool, input, _context| {
        let handle = handle.clone();
        let run = run.clone();
        Box::pin(async move {
            if rule == Rule::ReadsFree && READ_ONLY.contains(&tool.as_str()) {
                return PermissionResult::Allow(PermissionResultAllow::default());
            }

            let ask = Ask {
                id: format!(
                    "ask-{}",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
                ),
                run,
                tool: tool.clone(),
                input,
            };

            let (tx, rx) = oneshot::channel();
            if let Ok(mut waiting) = asks().lock() {
                waiting.insert(ask.id.clone(), tx);
            }
            let _ = handle.emit("session:permission", ask);

            // A dropped sender means the run ended or the window went away.
            // Either way the answer is no: silence is not consent here.
            match rx.await {
                Ok(true) => PermissionResult::Allow(PermissionResultAllow::default()),
                _ => PermissionResult::Deny(PermissionResultDeny {
                    message: "The window did not allow this tool".to_owned(),
                    interrupt: false,
                }),
            }
        })
    }));

    options
}

/// The run itself: connect, then take one prompt at a time.
///
/// A new prompt can only go in between turns — `query` needs the client
/// exclusively, while the message stream borrows it. Interrupting does not:
/// that goes out on the control channel and can happen mid-turn.
async fn drive<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    request: RunRequest,
    mut rx: mpsc::UnboundedReceiver<Step>,
) -> Result<()> {
    let mut client = ClaudeClient::new(options(&app, &id, &request));
    client
        .connect()
        .await
        .map_err(|error| Error::BadRequest(error.to_string()))?;

    let _ = app.emit(
        "session:state",
        serde_json::json!({ "id": id, "state": "running" }),
    );

    while let Some(step) = rx.recv().await {
        let prompt = match step {
            Step::Prompt(prompt) => prompt,
            Step::Interrupt => continue,
            Step::Stop => break,
        };

        client
            .query(prompt)
            .await
            .map_err(|error| Error::BadRequest(error.to_string()))?;

        let mut stop = false;
        {
            let mut stream = client.receive_response();
            while let Some(message) = stream.next().await {
                match message {
                    Ok(message) => {
                        for line in lines_of(&id, &message) {
                            let _ = app.emit("session:message", line);
                        }
                    }
                    Err(error) => {
                        let _ = app.emit(
                            "session:state",
                            serde_json::json!({ "id": id, "state": "error", "message": error.to_string() }),
                        );
                    }
                }

                // Between messages is where an interrupt or a stop can land.
                while let Ok(pending) = rx.try_recv() {
                    match pending {
                        Step::Interrupt => {
                            let _ = client.interrupt().await;
                        }
                        Step::Stop => stop = true,
                        // A prompt sent mid-turn waits its turn rather than
                        // being dropped; the loop picks it up next time round.
                        Step::Prompt(_) => {}
                    }
                }
                if stop {
                    break;
                }
            }
        }

        let _ = app.emit(
            "session:state",
            serde_json::json!({ "id": id, "state": "idle" }),
        );
        if stop {
            break;
        }
    }

    let _ = client.disconnect().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_looking_tools_are_free_and_only_under_that_rule() {
        // The list is the whole reason a rule can be relaxed at all, so it is
        // worth stating what is on it — and what is not.
        assert!(READ_ONLY.contains(&"Read"));
        assert!(!READ_ONLY.contains(&"Edit"));
        assert!(!READ_ONLY.contains(&"Write"));
        assert!(!READ_ONLY.contains(&"Bash"));
    }

    #[test]
    fn answering_an_unknown_ask_is_not_an_error() {
        answer("ask-nobody-waits-for-this", true);
    }

    #[test]
    fn steering_a_session_that_is_not_running_says_so() {
        let error = send("run-nothing", "hello").unwrap_err().to_string();
        assert!(error.contains("no session"), "{error}");
        assert!(interrupt("run-nothing").is_err());
        assert!(stop("run-nothing").is_err());
        assert!(list().iter().all(|run| run.id != "run-nothing"));
    }
}
