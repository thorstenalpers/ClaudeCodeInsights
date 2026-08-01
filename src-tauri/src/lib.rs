pub mod analysis;
pub mod commands;
pub mod ingest;
pub mod paths;
pub mod storage;

use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

/// How long the splash waits before showing the window regardless.
///
/// A UI that is merely slow is still better than a splash the user cannot get
/// past, so this is a safety net, not a deadline.
const READY_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SetupTask {
    Frontend,
    Backend,
}

impl SetupTask {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "frontend" => Some(Self::Frontend),
            "backend" => Some(Self::Backend),
            _ => None,
        }
    }
}

#[derive(Default)]
struct SetupState {
    frontend_ready: bool,
    backend_ready: bool,
    revealed: bool,
}

/// Records that one half of the startup finished, and reveals the window once
/// both have.
///
/// Both halves must go through here. An earlier version let the backend set its
/// flag directly on the state: whichever half finished second never re-checked
/// the condition, so a completed startup still sat on the splash until the
/// timeout fired.
fn mark_ready(app: &AppHandle, task: SetupTask) {
    {
        let state = app.state::<Mutex<SetupState>>();
        let Ok(mut setup) = state.lock() else {
            return;
        };

        match task {
            SetupTask::Frontend => setup.frontend_ready = true,
            SetupTask::Backend => setup.backend_ready = true,
        }

        if !(setup.frontend_ready && setup.backend_ready) {
            return;
        }
    }

    reveal_main_window(app);
}

/// Closes the splash and shows the main window. Safe to call more than once —
/// the handshake and the timeout race, and whichever arrives first wins.
fn reveal_main_window(app: &AppHandle) {
    {
        let state = app.state::<Mutex<SetupState>>();
        let Ok(mut setup) = state.lock() else {
            return;
        };
        if setup.revealed {
            return;
        }
        setup.revealed = true;
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
async fn set_complete(
    app: AppHandle,
    _state: State<'_, Mutex<SetupState>>,
    task: String,
) -> Result<(), String> {
    let task = SetupTask::parse(&task).ok_or_else(|| format!("unknown setup task '{task}'"))?;
    mark_ready(&app, task);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(SetupState::default()))
        .manage(commands::ScanGuard::default())
        .invoke_handler(tauri::generate_handler![
            set_complete,
            commands::get_scan_state,
            commands::start_scan,
            commands::get_overview,
            commands::list_sessions,
            commands::get_session_facets,
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

            commands::scan_on_first_launch(app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
