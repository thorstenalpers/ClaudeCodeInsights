use crate::error::Result;
use crate::live::{self, LiveTurn};
use tauri::AppHandle;

/// Starts following the newest transcript and returns its last few turns.
#[tauri::command(async)]
pub fn start_live(app: AppHandle, tail: usize) -> Result<Vec<LiveTurn>> {
    live::start(&app, tail)
}

#[tauri::command]
pub fn stop_live() {
    live::stop();
}
