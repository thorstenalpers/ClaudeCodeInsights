use crate::assistant::{self, CliStatus};
use crate::error::Result;

#[tauri::command]
pub fn get_cli_status(path: Option<String>) -> CliStatus {
    assistant::status(path.as_deref())
}

/// Runs on a worker thread: the CLI takes seconds, and an invoke that blocks
/// freezes the window for all of them.
#[tauri::command]
pub async fn ask_claude(path: Option<String>, prompt: String) -> Result<String> {
    tauri::async_runtime::spawn_blocking(move || assistant::ask(path.as_deref(), &prompt))
        .await
        .map_err(|error| crate::error::Error::BadRequest(error.to_string()))?
}
