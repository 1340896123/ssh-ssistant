mod db;
mod models;
mod ops;
mod ssh;
mod system;

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
            ops::init_ops_schema(app.handle())?;
            ssh::wsl::import_wsl_to_db(app.handle()).ok(); // Best effort import
            app.manage(ssh::AppState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            ssh::client::test_connection,
            ssh::client::connect,
            ssh::client::disconnect,
            ssh::client::cleanup_and_reconnect,
            ssh::client::cancel_transfer,
            ssh::client::cancel_command_execution,
            ssh::client::get_os_info,
            ssh::file_ops::list_files,
            ssh::file_ops::list_files_page,
            ssh::file_ops::read_remote_file,
            ssh::file_ops::write_remote_file,
            ssh::file_ops::search_remote_files,
            ssh::file_ops::create_directory,
            ssh::file_ops::create_file,
            ssh::file_ops::delete_item,
            ssh::file_ops::rename_item,
            ssh::file_ops::change_file_permission,
            ssh::file_ops::download_file,
            ssh::file_ops::upload_file,
            ssh::file_ops::upload_file_with_progress,
            ssh::file_ops::download_file_with_progress,
            ssh::file_ops::get_transfers,
            ssh::file_ops::remove_transfer,
            ssh::file_ops::start_transfer_with_manager,
            ssh::file_ops::pause_transfer,
            ssh::file_ops::resume_transfer,
            ssh::file_ops::get_transfer_records,
            ssh::file_ops::cleanup_old_transfers,
            ssh::terminal::write_to_pty,
            ssh::terminal::write_binary_to_pty,
            ssh::terminal::resize_pty,
            ssh::command::exec_command,
            ssh::command::get_working_directory,
            db::get_connections,
            db::create_connection,
            db::update_connection,
            db::delete_connection,
            ops::asset_get_host_assets,
            ops::asset_search_host_assets,
            ops::asset_create_host_asset,
            ops::asset_update_host_asset,
            ops::asset_delete_host_asset,
            ops::asset_touch_host_asset,
            ops::asset_toggle_favorite,
            ops::asset_get_asset_folders,
            ops::asset_create_asset_folder,
            ops::asset_update_asset_folder,
            ops::asset_delete_asset_folder,
            ops::asset_get_environments,
            ops::asset_create_environment,
            ops::asset_update_environment,
            ops::asset_delete_environment,
            ops::asset_get_asset_tags,
            ops::asset_create_asset_tag,
            ops::asset_delete_asset_tag,
            ops::asset_get_saved_views,
            ops::asset_create_saved_view,
            ops::asset_delete_saved_view,
            ops::access_get_access_endpoints,
            ops::access_get_credential_refs,
            ops::ops_list_job_templates,
            ops::ops_create_job_template,
            ops::ops_delete_job_template,
            ops::ops_list_job_runs,
            ops::ops_execute_job,
            ops::audit_list_events,
            ops::audit_create_event,
            ops::sync_get_state,
            ops::sync_save_state,
            ops::ai_plan_action,
            ops::ai_explain_state,
            ops::ai_generate_runbook,
            db::get_tunnels,
            db::create_tunnel,
            db::update_tunnel,
            db::delete_tunnel,
            db::get_settings,
            db::save_settings,
            db::get_groups,
            db::create_group,
            db::update_group,
            db::delete_group,
            db::get_ssh_keys,
            db::create_ssh_key,
            db::delete_ssh_key,
            db::generate_ssh_key,
            ssh::connection::install_ssh_key,
            ssh::tunnel::get_active_tunnels,
            ssh::tunnel::start_tunnel,
            ssh::tunnel::stop_tunnel,
            system::get_file_icon,
            ssh::system::get_remote_system_status,
            ssh::system::get_server_status,
            ssh::system::get_disk_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
