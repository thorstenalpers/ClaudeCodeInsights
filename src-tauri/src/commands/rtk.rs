use crate::error::Result;
use crate::rtk;

#[tauri::command]
pub fn get_rtk_status(path: Option<String>) -> rtk::Status {
    rtk::status(path.as_deref())
}

#[tauri::command]
pub fn get_rtk_report() -> Result<rtk::Report> {
    rtk::report()
}
