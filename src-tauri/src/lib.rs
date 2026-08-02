pub mod analysis;
pub mod assistant;
pub mod commands;
pub mod error;
pub mod ingest;
pub mod paths;
pub mod projects;
pub mod state;
pub mod storage;

use crate::error::{Error, Result};
use crate::state::{SetupState, SetupTask};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// How long the splash waits before showing the window regardless.
///
/// A UI that is merely slow is still better than a splash the user cannot get
/// past, so this is a safety net, not a deadline.
const READY_TIMEOUT: Duration = Duration::from_secs(15);

fn mark_ready(app: &AppHandle, task: SetupTask) {
    let both_ready = {
        let state = app.state::<Mutex<SetupState>>();
        let Ok(mut setup) = state.lock() else {
            return;
        };
        setup.complete(task)
    };

    if both_ready {
        reveal_main_window(app);
    }
}

/// Closes the splash and shows the main window. Safe to call more than once —
/// the handshake and the timeout race, and whichever arrives first wins.
fn reveal_main_window(app: &AppHandle) {
    {
        let state = app.state::<Mutex<SetupState>>();
        let Ok(mut setup) = state.lock() else {
            return;
        };
        if !setup.claim_reveal() {
            return;
        }
    }

    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

/// Reported by each side once it has finished starting up.
///
/// The frontend calls this only after its first real paint — a window that is
/// visible but still blank looks broken, which is why the splash exists at all.
#[tauri::command]
async fn set_complete(app: AppHandle, task: String) -> Result<()> {
    let task = SetupTask::parse(&task)
        .ok_or_else(|| Error::BadRequest(format!("unknown setup task '{task}'")))?;
    mark_ready(&app, task);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(SetupState::default()))
        .manage(state::ScanGuard::default())
        .invoke_handler(tauri::generate_handler![
            set_complete,
            commands::scan::get_scan_state,
            commands::scan::start_scan,
            commands::overview::get_overview,
            commands::sessions::list_sessions,
            commands::sessions::get_session_facets,
            commands::sessions::get_transcript,
            commands::projects::list_projects,
            commands::projects::preview_project_transcripts,
            commands::projects::delete_project_transcripts,
            commands::projects::get_project_settings,
            commands::projects::update_project_settings,
            commands::projects::remove_project_registration,
            commands::usage::list_tools,
            commands::usage::list_models,
            commands::usage::list_agents,
            commands::usage::get_rhythm,
            commands::usage::get_series,
            commands::usage::get_series_facets,
            commands::assistant::get_cli_status,
            commands::assistant::ask_claude,
            commands::assistant::list_providers,
            commands::assistant::has_api_key,
            commands::assistant::set_api_key,
            commands::assistant::open_free_key_url,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Nothing slow to do yet; the database and the transcript scan will
            // hang off here, which is why the handshake has a backend half.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                mark_ready(&handle, SetupTask::Backend);
            });

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(READY_TIMEOUT);
                reveal_main_window(&handle);
            });

            commands::scan::scan_on_first_launch(app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
