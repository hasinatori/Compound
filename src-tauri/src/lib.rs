//! Сборка Tauri-приложения: тут регистрируются все команды.
//! Tauri app assembly: every command is registered here.
//! Новую #[tauri::command] — не забудь добавить в invoke_handler ниже.
//! New #[tauri::command] -> do not forget to add it to invoke_handler below.
//! Порядок в invoke_handler = порядок чтения, держим по модулям.
//! Order in invoke_handler = reading order, keep it grouped by module.

mod archive_cmd;
mod editor_cmd;
mod fs_cmd;
mod fsutil;
mod misc_cmd;
mod ops_cmd;
mod search_cmd;
mod state;
mod terminal_cmd;
mod tools_cmd;
mod types;
mod ui_error;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = AppState::new(app.handle().clone())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Navigation / Basis
            fs_cmd::list_dir,
            fs_cmd::get_places,
            fs_cmd::path_parts,
            fs_cmd::disk_usage,
            fs_cmd::thumb,
            fs_cmd::open_default,
            // Datei-Operationen
            ops_cmd::create_item,
            ops_cmd::rename_item,
            ops_cmd::copy_items,
            ops_cmd::move_items,
            ops_cmd::trash_items,
            ops_cmd::delete_permanent,
            ops_cmd::duplicate_item,
            ops_cmd::cancel_op,
            ops_cmd::list_ops,
            ops_cmd::resolve_conflict,
            ops_cmd::list_trash,
            ops_cmd::restore_trash,
            ops_cmd::empty_trash,
            ops_cmd::open_external_terminal,
            // Suche / Index
            search_cmd::index_folder,
            search_cmd::indexed_roots,
            search_cmd::search_index,
            search_cmd::search_now,
            search_cmd::clear_index,
            // Werkzeuge
            tools_cmd::find_duplicates,
            tools_cmd::compare_folders,
            tools_cmd::multi_rename,
            // Archive
            archive_cmd::archive_create,
            archive_cmd::archive_extract,
            archive_cmd::archive_list,
            // Editor
            editor_cmd::read_text,
            editor_cmd::save_text,
            // Terminal
            terminal_cmd::terminal_open,
            terminal_cmd::terminal_write,
            terminal_cmd::terminal_resize,
            terminal_cmd::terminal_close,
            // Sonstiges
            misc_cmd::about,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}