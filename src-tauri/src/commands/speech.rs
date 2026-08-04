use crate::error::Result;
use crate::speech;

#[tauri::command]
pub fn speech_available() -> bool {
    speech::available()
}

/// Blocks until the recogniser has heard a phrase or given up, so it runs on a
/// worker rather than holding the window's thread.
#[tauri::command(async)]
pub fn recognize_speech(locale: String) -> Result<String> {
    speech::recognize(&locale)
}

/// The languages Windows can dictate in on this machine.
#[tauri::command]
pub fn list_speech_languages() -> Vec<String> {
    speech::languages()
}

#[tauri::command]
pub fn open_speech_settings() -> Result<()> {
    speech::open_settings()
}

/// The capture devices, with the one the recogniser will actually hear marked.
#[tauri::command]
pub fn list_microphones() -> Vec<speech::Microphone> {
    speech::microphones()
}

#[tauri::command]
pub fn open_sound_settings() -> Result<()> {
    speech::open_sound_settings()
}

/// The speech models this app can fetch, and whether each is here.
#[tauri::command]
pub fn list_speech_models() -> Vec<crate::asr::ModelInfo> {
    crate::asr::list()
}

#[tauri::command]
pub fn speech_models_folder() -> Result<String> {
    crate::asr::folder()
}

/// Hundreds of megabytes, so it runs on a worker and reports as it goes.
#[tauri::command(async)]
pub fn install_speech_model(app: tauri::AppHandle, id: String) -> Result<crate::asr::ModelInfo> {
    crate::asr::install(&app, &id)
}

#[tauri::command]
pub fn remove_speech_model(id: String) -> Result<()> {
    crate::asr::remove(&id)
}

/// Records from a chosen microphone and reads back what was said, here.
#[tauri::command(async)]
pub fn dictate(device: Option<String>, model: String, locale: String) -> Result<String> {
    crate::asr::listen(device.as_deref(), &model, &locale)
}

/// Ends the current dictation; what was heard up to now is still read.
#[tauri::command]
pub fn stop_dictating() {
    crate::asr::stop();
}
