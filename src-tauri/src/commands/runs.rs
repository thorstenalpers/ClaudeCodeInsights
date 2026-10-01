use crate::error::Result;
use crate::sessions::{self, RunInfo, RunRequest};

/// Starts a coding session in a project and returns the id to steer it by.
#[tauri::command]
pub fn start_session(app: tauri::AppHandle, request: RunRequest) -> Result<RunInfo> {
    sessions::start(&app, request)
}

#[tauri::command]
pub fn send_to_session(id: String, prompt: String) -> Result<()> {
    sessions::send(&id, &prompt)
}

#[tauri::command]
pub fn interrupt_session(id: String) -> Result<()> {
    sessions::interrupt(&id)
}

#[tauri::command]
pub fn stop_session(id: String) -> Result<()> {
    sessions::stop(&id)
}

/// Answers a tool that is waiting for permission.
#[tauri::command]
pub fn answer_permission(id: String, allow: bool) {
    sessions::answer(&id, allow);
}

#[tauri::command]
pub fn list_sessions_running() -> Vec<RunInfo> {
    sessions::list()
}
