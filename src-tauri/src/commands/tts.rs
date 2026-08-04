use crate::error::Result;
use crate::tts::{self, PackInfo};

#[tauri::command]
pub fn voice_packs_folder() -> Result<String> {
    tts::folder()
}

#[tauri::command]
pub fn list_voice_packs() -> Result<Vec<PackInfo>> {
    tts::list()
}

/// Downloads and unpacks a voice pack. How far it has come arrives as
/// `voice:progress` events while this call is still running.
///
/// `spawn_blocking`, not `command(async)`: the latter runs the body *inside* the
/// async runtime, and the blocking HTTP client tears down its own runtime there,
/// which panics before the first byte is read.
#[tauri::command]
pub async fn install_voice_pack(app: tauri::AppHandle, id: String) -> Result<PackInfo> {
    tauri::async_runtime::spawn_blocking(move || tts::install(&app, &id))
        .await
        .map_err(|error| crate::error::Error::BadRequest(error.to_string()))?
}

/// Stops one running download at its next chunk.
#[tauri::command]
pub fn cancel_voice_pack(id: String) {
    tts::cancel(&id);
}

#[tauri::command(async)]
pub fn remove_voice_pack(id: String) -> Result<()> {
    tts::remove(&id)
}

/// Cuts the sentence being read short.
#[tauri::command]
pub fn stop_speaking() {
    tts::silence();
}

/// Speaks and returns when the sentence has been said.
#[tauri::command(async)]
pub fn speak_text(id: String, speaker: i32, text: String) -> Result<()> {
    tts::speak(&id, speaker, &text)
}
