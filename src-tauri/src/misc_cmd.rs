use crate::state::AppState;
use crate::types::AboutInfo;
use crate::ui_error::Error;
use tauri::State;

/// App-Informationen für den About-Dialog / Statusleiste.
#[tauri::command]
pub fn about(state: State<'_, AppState>) -> Result<AboutInfo, Error> {
    let info = state.app.package_info();
    Ok(AboutInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
        identifier: state.app.config().identifier.clone(),
    })
}