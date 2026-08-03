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
