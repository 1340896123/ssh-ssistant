mod db;
mod models;
mod ssh;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            db::init_db(app.handle())?;
            app.manage(ssh::AppState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            ssh::connect,
            ssh::disconnect,
            ssh::list_files,
            ssh::read_remote_file,
            ssh::write_remote_file,
            ssh::search_remote_files,
            ssh::create_directory,
            ssh::create_file,
            ssh::delete_item,
            ssh::rename_item,
            ssh::change_file_permission,
            ssh::download_file,
            ssh::upload_file,
            ssh::upload_file_with_progress,
            ssh::download_file_with_progress,
            ssh::cancel_transfer,
            ssh::download_temp_and_open,
            ssh::edit_remote_file,
            ssh::exec_command,
            ssh::write_to_pty,
            ssh::resize_pty,
            db::get_connections,
            db::create_connection,
            db::update_connection,
            db::delete_connection,
            db::get_settings,
            db::save_settings,
            db::get_groups,
            db::create_group,
            db::update_group,
            db::delete_group
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
