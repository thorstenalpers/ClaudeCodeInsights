use crate::error::Result;
use crate::speech;

#[tauri::command]
pub fn open_speech_settings() -> Result<()> {
    speech::open_settings()
}
