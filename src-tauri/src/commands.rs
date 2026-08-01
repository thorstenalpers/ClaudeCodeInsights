use crate::ingest::scanner::{self, ScanProgress, ScanStats};
use crate::{paths, storage};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct ScanGuard {
    running: AtomicBool,
}

/// What the Overview page needs for its headline figures.
///
/// Cost is deliberately absent: it is derived from the token counts and the
/// price table at display time, so changing a rate never means rescanning.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub sessions: i64,
    pub turns: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub active_days: i64,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanState {
    pub running: bool,
    pub roots: Vec<String>,
}

#[tauri::command]
pub fn get_scan_state(guard: State<'_, ScanGuard>) -> ScanState {
    ScanState {
        running: guard.running.load(Ordering::SeqCst),
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
pub fn start_scan(app: AppHandle, guard: State<'_, ScanGuard>) -> Result<bool, String> {
    if guard.running.swap(true, Ordering::SeqCst) {
        return Ok(false);
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        let outcome = run_scan(&handle);

        let guard = handle.state::<ScanGuard>();
        guard.running.store(false, Ordering::SeqCst);

        match outcome {
            Ok(stats) => {
                let _ = handle.emit("scan:completed", &stats);
            }
            Err(message) => {
                let _ = handle.emit("scan:failed", message);
            }
        }
    });

    Ok(true)
}

fn run_scan(app: &AppHandle) -> Result<ScanStats, String> {
    paths::ensure_data_dir().map_err(|e| e.to_string())?;
    let mut conn = storage::open(&paths::database_path()).map_err(|e| format!("{e:#}"))?;

    let roots = paths::default_scan_roots();
    let emitter = app.clone();
    let mut last_emit = std::time::Instant::now() - std::time::Duration::from_secs(1);

    scanner::scan(&mut conn, &roots, move |progress: ScanProgress| {
        // A file can be parsed in milliseconds; forwarding every one of them
        // would flood the UI with more events than it can render.
        let finished = progress.files_done == progress.files_total;
        if finished || last_emit.elapsed() >= std::time::Duration::from_millis(100) {
            last_emit = std::time::Instant::now();
            let _ = emitter.emit("scan:progress", &progress);
        }
    })
    .map_err(|e| format!("{e:#}"))
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

    let guard = app.state::<ScanGuard>();
    if guard.running.swap(true, Ordering::SeqCst) {
        return;
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        let outcome = run_scan(&handle);
        handle
            .state::<ScanGuard>()
            .running
            .store(false, Ordering::SeqCst);

        match outcome {
            Ok(stats) => {
                let _ = handle.emit("scan:completed", &stats);
            }
            Err(message) => {
                let _ = handle.emit("scan:failed", message);
            }
        }
    });
}

#[tauri::command]
pub fn get_overview() -> Result<Overview, String> {
    let conn = storage::open(&paths::database_path()).map_err(|e| format!("{e:#}"))?;

    let overview = conn
        .query_row(
            r#"
            SELECT
                (SELECT COUNT(*) FROM sessions),
                COUNT(*),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(cache_read_tokens), 0),
                COALESCE(SUM(cache_write_tokens), 0),
                COUNT(DISTINCT substr(ts_utc, 1, 10)),
                MIN(ts_utc),
                MAX(ts_utc)
            FROM turns
            "#,
            [],
            |row| {
                Ok(Overview {
                    sessions: row.get(0)?,
                    turns: row.get(1)?,
                    input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                    cache_read_tokens: row.get(4)?,
                    cache_write_tokens: row.get(5)?,
                    active_days: row.get(6)?,
                    first_ts: row.get(7)?,
                    last_ts: row.get(8)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(overview)
}
