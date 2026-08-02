use super::db;
use crate::analysis::rhythm::{self, Rhythm};
use crate::analysis::series::{self, Series, SeriesFacets, SeriesQuery};
use crate::analysis::usage::{self, AgentRow, ModelRow, ToolRow};
use crate::error::Result;

#[tauri::command]
pub fn list_tools() -> Result<Vec<ToolRow>> {
    Ok(usage::tools(&db()?)?)
}

#[tauri::command]
pub fn list_models() -> Result<Vec<ModelRow>> {
    Ok(usage::models(&db()?)?)
}

#[tauri::command]
pub fn list_agents() -> Result<Vec<AgentRow>> {
    Ok(usage::agents(&db()?)?)
}

#[tauri::command]
pub fn get_rhythm() -> Result<Rhythm> {
    Ok(rhythm::load(&db()?)?)
}

#[tauri::command]
pub fn get_series(query: SeriesQuery) -> Result<Series> {
    Ok(series::load(&db()?, &query)?)
}

#[tauri::command]
pub fn get_series_facets() -> Result<SeriesFacets> {
    Ok(series::facets(&db()?)?)
}
