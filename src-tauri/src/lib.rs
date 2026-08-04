pub mod analysis;
pub mod asr;
pub mod assistant;
pub mod commands;
pub mod error;
pub mod hub;
pub mod ingest;
pub mod live;
pub mod orchestration;
pub mod paths;
pub mod projects;
pub mod session_tools;
pub mod sessions;
pub mod speech;
pub mod state;
pub mod storage;
pub mod tts;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
use tauri_plugin_log::{Target, TargetKind};

pub fn run() {
    tauri::Builder::default()
        .manage(state::ScanGuard::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan::get_scan_state,
            commands::app::get_app_info,
            commands::app::open_data_folder,
            commands::scan::start_scan,
            commands::overview::get_overview,
            commands::sessions::list_sessions,
            commands::sessions::get_session_facets,
            commands::sessions::get_transcript,
            commands::projects::list_projects,
            commands::projects::open_claude_config,
            commands::projects::preview_project_transcripts,
            commands::projects::delete_project_transcripts,
            commands::projects::get_project_settings,
            commands::projects::update_project_settings,
            commands::projects::remove_project_registration,
            commands::usage::list_tools,
            commands::usage::list_models,
            commands::runs::start_session,
            commands::runs::send_to_session,
            commands::runs::interrupt_session,
            commands::runs::stop_session,
            commands::runs::answer_permission,
            commands::runs::list_sessions_running,
            commands::orchestration::list_tasks,
            commands::orchestration::add_task,
            commands::orchestration::update_task,
            commands::orchestration::remove_task,
            commands::orchestration::list_definitions,
            commands::usage::list_activities,
            commands::usage::list_agents,
            commands::usage::get_rhythm,
            commands::usage::get_series,
            commands::usage::get_series_facets,
            commands::speech::speech_available,
            commands::speech::recognize_speech,
            commands::speech::open_speech_settings,
            commands::speech::list_speech_languages,
            commands::speech::list_speech_models,
            commands::speech::speech_models_folder,
            commands::speech::install_speech_model,
            commands::speech::remove_speech_model,
            commands::speech::dictate,
            commands::speech::stop_dictating,
            commands::speech::list_microphones,
            commands::speech::open_sound_settings,
            commands::live::start_live,
            commands::live::stop_live,
            commands::tts::list_voice_packs,
            commands::tts::voice_packs_folder,
            commands::tts::install_voice_pack,
            commands::tts::search_voice_hub,
            commands::tts::install_hub_voice,
            commands::tts::cancel_voice_pack,
            commands::tts::remove_voice_pack,
            commands::tts::speak_text,
            commands::tts::stop_speaking,
            commands::assistant::get_cli_status,
            commands::assistant::ask_claude,
            commands::assistant::list_providers,
            commands::assistant::local_options,
            commands::assistant::has_api_key,
            commands::assistant::set_api_key,
            commands::assistant::open_free_key_url,
        ])
        .setup(|app| {
            // Always, not only in a debug build: the log view is the one place
            // a user can look when a download or a scan goes wrong, and that
            // happens in the build they actually run. The webview target is
            // what carries the host's lines into that view.
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .target(Target::new(TargetKind::Stdout))
                    .target(Target::new(TargetKind::Webview))
                    .target(Target::new(TargetKind::LogDir { file_name: None }))
                    .build(),
            )?;

            commands::scan::scan_on_first_launch(app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
