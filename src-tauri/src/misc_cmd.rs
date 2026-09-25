// Мелочи: about-инфо для окна «О программе».
// Odds and ends: about info for the app dialog.

use crate::state::AppState;
use crate::types::AboutInfo;
use crate::ui_error::Error;
use tauri::State;

/// Инфо о приложении для окна «О программе» и статусбара.
/// App info for the about dialog / status bar.
#[tauri::command]
pub fn about(state: State<'_, AppState>) -> Result<AboutInfo, Error> {
    let info = state.app.package_info();
    Ok(AboutInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
        identifier: state.app.config().identifier.clone(),
    })
}