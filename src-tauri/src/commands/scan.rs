use crate::analysis::activity;
use crate::analysis::categories::CategoryMap;
use crate::error::Result;
use crate::ingest::scanner::{self, ScanProgress, ScanStats};
use crate::state::ScanGuard;
use crate::{paths, storage};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanState {
    pub running: bool,
    pub roots: Vec<String>,
}

#[tauri::command]
pub fn get_scan_state(guard: State<'_, ScanGuard>) -> ScanState {
    ScanState {
        running: guard.is_running(),
        roots: paths::default_scan_roots()
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
    }
}

/// Starts a scan on a worker thread and reports progress as events.
///
/// It returns as soon as the scan is under way; a scan over a large history
/// takes seconds, and blocking the invoke would freeze the window for all of it.
#[tauri::command]
pub fn start_scan(app: AppHandle, guard: State<'_, ScanGuard>) -> bool {
    if !guard.try_claim() {
        return false;
    }
    spawn_scan(app.clone());
    true
}

/// Starts a scan if the database holds nothing yet.
///
/// An empty app cannot answer a single question, so the first launch should not
/// require the user to find a button first. Once there is data, scanning stays
/// explicit — re-reading a large history on every start would be rude.
pub fn scan_on_first_launch(app: &AppHandle) {
    let is_empty = storage::open(&paths::database_path())
        .ok()
        .and_then(|conn| {
            conn.query_row("SELECT COUNT(*) FROM turns", [], |r| r.get::<_, i64>(0))
                .ok()
        })
        .is_none_or(|turns| turns == 0);

    if !is_empty {
        return;
    }
    if !app.state::<ScanGuard>().try_claim() {
        return;
    }
    spawn_scan(app.clone());
}

fn spawn_scan(app: AppHandle) {
    std::thread::spawn(move || {
        let outcome = run_scan(&app);
        app.state::<ScanGuard>().release();

        match outcome {
            Ok(stats) => {
                let _ = app.emit("scan:completed", &stats);
            }
            Err(error) => {
                let _ = app.emit("scan:failed", error.to_string());
            }
        }
    });
}

fn run_scan(app: &AppHandle) -> Result<ScanStats> {
    paths::ensure_data_dir()?;
    let mut conn = storage::open(&paths::database_path())?;

    let roots = paths::default_scan_roots();
    let emitter = app.clone();
    let mut last_emit = std::time::Instant::now() - std::time::Duration::from_secs(1);

    let stats = scanner::scan(&mut conn, &roots, move |progress: ScanProgress| {
        // A file can be parsed in milliseconds; forwarding every one of them
        // would flood the UI with more events than it can render.
        let finished = progress.files_done == progress.files_total;
        if finished || last_emit.elapsed() >= std::time::Duration::from_millis(100) {
            last_emit = std::time::Instant::now();
            let _ = emitter.emit("scan:progress", &progress);
        }
    })?;

    activity::recompute(&conn, &CategoryMap::default())?;
    Ok(stats)
}
