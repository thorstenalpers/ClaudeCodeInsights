use super::db;
use crate::error::Result;
use crate::projects::{self, DeleteOutcome, ProjectsReport, TranscriptFile, WriteOutcome};

#[tauri::command]
pub fn list_projects() -> Result<ProjectsReport> {
    Ok(projects::list(&db()?)?)
}

/// The exact files a deletion would remove, for the confirmation dialog.
#[tauri::command]
pub fn preview_project_transcripts(path: String) -> Result<Vec<TranscriptFile>> {
    Ok(projects::transcript_files(&path)?)
}

#[tauri::command]
pub fn delete_project_transcripts(path: String) -> Result<DeleteOutcome> {
    Ok(projects::delete_transcripts(&mut db()?, &path)?)
}

#[tauri::command]
pub fn open_claude_config() -> Result<()> {
    Ok(projects::open_config()?)
}

#[tauri::command]
pub fn get_project_settings(path: String) -> Result<String> {
    Ok(projects::settings_json(&path)?)
}

#[tauri::command]
pub fn update_project_settings(path: String, settings: String) -> Result<WriteOutcome> {
    Ok(projects::update_settings(&path, &settings)?)
}

#[tauri::command]
pub fn remove_project_registration(path: String) -> Result<WriteOutcome> {
    Ok(projects::remove_registration(&path)?)
}
