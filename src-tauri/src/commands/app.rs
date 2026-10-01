use crate::error::Result;
use crate::paths;
use serde::Serialize;

/// What the info page says about the running build.
///
/// The paths travel with it because they are the answer to "where does this
/// thing keep my data" — a question the app should not make anybody guess at.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub database: String,
    pub voices: String,
    /// Where the log plugin writes the file it keeps beside the window's view.
    pub logs: String,
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> Result<AppInfo> {
    use tauri::Manager;

    let data = paths::data_dir();
    let logs = app
        .path()
        .app_log_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();

    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: data.to_string_lossy().into_owned(),
        database: paths::database_path().to_string_lossy().into_owned(),
        voices: data.join("voices").to_string_lossy().into_owned(),
        logs,
    })
}

/// Opens a folder in the system's file browser.
///
/// It takes what the info page was told by `get_app_info` rather than an
/// arbitrary string, so the window cannot ask for a path the host never named.
#[tauri::command]
pub fn open_data_folder(which: String) -> Result<()> {
    let data = paths::data_dir();
    let path = match which.as_str() {
        "voices" => data.join("voices"),
        _ => data,
    };

    std::fs::create_dir_all(&path)?;

    #[cfg(windows)]
    std::process::Command::new("explorer").arg(&path).spawn()?;

    #[cfg(not(windows))]
    std::process::Command::new("xdg-open").arg(&path).spawn()?;

    Ok(())
}
