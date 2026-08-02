use crate::assistant::{self, cli::CliStatus, providers::ProviderInfo};
use crate::error::Result;

#[tauri::command]
pub fn get_cli_status(path: Option<String>) -> CliStatus {
    assistant::cli::status(path.as_deref())
}

/// The hosted sources on offer, and where a free key can be had.
#[tauri::command]
pub fn list_providers() -> Vec<ProviderInfo> {
    assistant::providers::catalogue()
}

/// Whether a key is stored — never the key.
#[tauri::command]
pub fn has_api_key(provider: String) -> bool {
    assistant::secrets::has(&provider)
}

/// Stores a key, or forgets it when the value is empty.
#[tauri::command]
pub fn set_api_key(provider: String, key: String) -> Result<()> {
    assistant::secrets::set(&provider, &key)
}

/// Runs on a worker thread: a hosted call takes seconds, and an invoke that
/// blocks freezes the window for all of them.
#[tauri::command]
pub async fn ask_claude(source: String, path: Option<String>, prompt: String) -> Result<String> {
    tauri::async_runtime::spawn_blocking(move || assistant::ask(&source, path.as_deref(), &prompt))
        .await
        .map_err(|error| crate::error::Error::BadRequest(error.to_string()))?
}

/// Opens the chosen provider's free-key page in the system browser.
#[tauri::command]
pub fn open_free_key_url(provider: String) -> Result<()> {
    assistant::providers::open_free_key_url(&provider)
}
