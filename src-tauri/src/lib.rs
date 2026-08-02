pub mod analysis;
pub mod assistant;
pub mod commands;
pub mod error;
pub mod ingest;
pub mod paths;
pub mod projects;
pub mod speech;
pub mod state;
pub mod storage;


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::ScanGuard::default())
        .invoke_handler(tauri::generate_handler![
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
            commands::speech::speech_available,
            commands::speech::recognize_speech,
            commands::assistant::get_cli_status,
            commands::assistant::ask_claude,
            commands::assistant::list_providers,
            commands::assistant::local_options,
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

            commands::scan::scan_on_first_launch(app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
