use super::db;
use crate::analysis::sessions::{self, SessionFacets, SessionPage, SessionQuery};
use crate::analysis::transcript::{self, TranscriptPage};
use crate::error::Result;

#[tauri::command]
pub fn list_sessions(query: SessionQuery) -> Result<SessionPage> {
    Ok(sessions::query(&db()?, &query)?)
}

#[tauri::command]
pub fn get_session_facets() -> Result<SessionFacets> {
    Ok(sessions::facets(&db()?)?)
}

/// The conversation itself, read from the transcript rather than the database.
///
/// The database holds figures; the transcript stays the source for what was
/// said. That keeps the database small and the replay always current, at the
/// cost of re-reading the file — which is why it is paged.
#[tauri::command]
pub fn get_transcript(session_id: String, offset: usize, limit: usize) -> Result<TranscriptPage> {
    Ok(transcript::load(
        &db()?,
        &session_id,
        offset,
        limit.clamp(1, 500),
    )?)
}
