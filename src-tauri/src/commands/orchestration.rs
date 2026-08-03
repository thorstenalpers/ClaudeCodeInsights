use super::db;
use crate::error::Result;
use crate::orchestration::{self, Definition, Task};

#[tauri::command]
pub fn list_tasks(project: Option<String>) -> Result<Vec<Task>> {
    orchestration::list_tasks(&db()?, project.as_deref())
}

#[tauri::command]
pub fn add_task(project: String, title: String) -> Result<Task> {
    orchestration::add_task(&db()?, &project, &title)
}

#[tauri::command]
pub fn update_task(task: Task) -> Result<()> {
    orchestration::update_task(&db()?, &task)
}

#[tauri::command]
pub fn remove_task(id: i64) -> Result<()> {
    orchestration::remove_task(&db()?, id)
}

/// The skills and agent definitions Claude Code would find for a project.
#[tauri::command]
pub fn list_definitions(project: Option<String>) -> (Vec<Definition>, Vec<Definition>) {
    (
        orchestration::skills(project.as_deref()),
        orchestration::agents(project.as_deref()),
    )
}
