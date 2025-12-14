use super::connection::SessionSshPool;
use super::terminal::start_shell_thread;
use crate::models::Connection as SshConnConfig;
use crate::ssh::{execute_ssh_operation, ShellMsg};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, State};
use uuid::Uuid;

#[derive(Clone)]
pub enum ClientType {
    Ssh(Arc<SessionSshPool>),
    Wsl(String), // Distro name
}

#[derive(Clone)]
pub struct SshClient {
    pub client_type: ClientType,                       // SSH连接池 或 WSL
    pub shell_tx: Option<Sender<ShellMsg>>,            // 终端消息通道
    pub owner_cache: Arc<Mutex<HashMap<u32, String>>>, // UID缓存
    pub shutdown_signal: Arc<AtomicBool>,              // 用于通知后台监控任务停止
    pub os_info: Option<String>,                       // Remote OS information
}

use crate::models::Transfer;

pub struct TransferState {
    pub data: Mutex<Transfer>,
    pub cancel_flag: Arc<AtomicBool>,
}

pub struct AppState {
    pub clients: Mutex<HashMap<String, SshClient>>,
    pub transfers: Mutex<HashMap<String, Arc<TransferState>>>, // ID -> TransferState
    pub command_cancellations: Mutex<HashMap<String, Arc<AtomicBool>>>, // Command ID -> CancelFlag
}

impl AppState {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            transfers: Mutex::new(HashMap::new()),
            command_cancellations: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub async fn test_connection(app: AppHandle, config: SshConnConfig) -> Result<String, String> {
    let mut populated_config = config.clone();

    if populated_config.auth_type.as_deref() == Some("key") {
        if let Some(key_id) = populated_config.ssh_key_id {
            match crate::db::get_ssh_key_by_id(&app, key_id) {
                Ok(Some(key)) => {
                    populated_config.key_content = Some(key.content);
                    populated_config.key_passphrase = key.passphrase;
                }
                Ok(None) => {
                    return Err(format!("SSH Key with ID {} not found", key_id));
                }
                Err(e) => {
                    return Err(format!("Failed to fetch SSH Key: {}", e));
                }
            }
        } else {
            // If key auth is selected but no ID provided, fail early
            return Err("SSH Key ID is missing needed for key authentication".to_string());
        }
    }

    execute_ssh_operation(move || {
        let session = super::connection::establish_connection_with_retry(&populated_config)?;
        // Disconnect immediately as we only wanted to test credentials/reachability
        let _ = session.session.disconnect(None, "Connection Test", None);
        Ok("Connection successful".to_string())
    })
    .await
}

#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    config: SshConnConfig,
    id: Option<String>,
) -> Result<String, String> {
    // Use OS type from connection config with fallback to Linux for backward compatibility
    let os_info = config
        .os_type
        .clone()
        .unwrap_or_else(|| "Linux".to_string());
    println!("Using OS type from config: {}", os_info);
    let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

    // Define shutdown_signal early
    let shutdown_signal = Arc::new(AtomicBool::new(false));

    let client_type = if config.host.starts_with("wsl://") {
        let distro = config.host.trim_start_matches("wsl://").to_string();
        ClientType::Wsl(distro)
    } else {
        // Create SSH connection pool in a blocking task to avoid blocking the async runtime
        let max_bg_sessions = {
            let settings = crate::db::get_settings(app.clone()).unwrap_or_else(|_| {
                crate::models::AppSettings {
                    theme: "dark".to_string(),
                    language: "zh".to_string(),
                    ai: crate::models::AIConfig {
                        api_url: "".to_string(),
                        api_key: "".to_string(),
                        model_name: "".to_string(),
                    },
                    terminal_appearance: crate::models::TerminalAppearanceSettings {
                        font_size: 14,
                        font_family: "Menlo".to_string(),
                        cursor_style: "block".to_string(),
                        line_height: 1.0,
                    },
                    file_manager: crate::models::FileManagerSettings {
                        view_mode: "flat".to_string(),
                        sftp_buffer_size: 512,
                    },
                    ssh_pool: crate::models::SshPoolSettings {
                        max_background_sessions: 10,
                        enable_auto_cleanup: true,
                        cleanup_interval_minutes: 10,
                    },
                }
            });
            settings.ssh_pool.max_background_sessions as usize
        };

        // Populate key content if needed
        let mut populated_config = config.clone();
        if populated_config.auth_type.as_deref() == Some("key") {
            if let Some(key_id) = populated_config.ssh_key_id {
                match crate::db::get_ssh_key_by_id(&app, key_id) {
                    Ok(Some(key)) => {
                        populated_config.key_content = Some(key.content);
                        populated_config.key_passphrase = key.passphrase;
                    }
                    Ok(None) => {
                        println!("Warning: SSH Key with ID {} not found", key_id);
                    }
                    Err(e) => {
                        println!("Error fetching SSH Key: {}", e);
                        return Err(format!("Failed to fetch SSH Key: {}", e));
                    }
                }
            }
        }

        let config_clone = populated_config.clone();
        let ssh_pool =
            tokio::task::spawn_blocking(move || SessionSshPool::new(config_clone, max_bg_sessions))
                .await
                .map_err(|e| format!("Task join error: {}", e))??;

        // Start cleanup task for SSH only
        let cleanup_pool = ssh_pool.clone();
        let monitor_signal = shutdown_signal.clone();

        // ... (Cleanup task setup same as before)
        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(Duration::from_secs(30));
            let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                tokio::select! {
                     _ = cleanup_interval.tick() => {
                        if monitor_signal.load(Ordering::Relaxed) { break; }
                        cleanup_pool.cleanup_disconnected();
                    }
                    _ = heartbeat_interval.tick() => {
                        if monitor_signal.load(Ordering::Relaxed) { break; }
                        if let Err(e) = cleanup_pool.heartbeat_check() {
                             eprintln!("Heartbeat check failed: {}", e);
                        }
                    }
                }
            }
        });

        ClientType::Ssh(Arc::new(ssh_pool))
    };

    // Create mutable client reference for terminal initialization
    let mut client = SshClient {
        client_type,
        shell_tx: None, // Will be set by start_shell_thread
        owner_cache: Arc::new(Mutex::new(HashMap::new())),
        shutdown_signal,
        os_info: Some(os_info),
    };

    // Start shell thread for terminal functionality
    let shell_tx = start_shell_thread(app.clone(), &mut client, id.clone())
        .map_err(|e| format!("Failed to start shell thread: {}", e))?;

    // Update client with the shell transmitter
    client.shell_tx = Some(shell_tx);

    state
        .clients
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id.clone(), client);

    Ok(id)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Get client to disconnect
    let client = {
        let mut clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.remove(&id)
    };

    if let Some(client) = client {
        // 1. 发送停止信号，终止后台监控任务
        client.shutdown_signal.store(true, Ordering::Relaxed);

        // 2. 关闭 Shell 线程
        if let Some(tx) = client.shell_tx {
            let _ = tx.send(ShellMsg::Exit);
        }

        // 3. 关闭连接
        match &client.client_type {
            ClientType::Ssh(pool) => pool.close_all(),
            ClientType::Wsl(_) => {}
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn cleanup_and_reconnect(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    // 重新建立连接
    match &client.client_type {
        ClientType::Ssh(pool) => {
            pool.close_all();
            pool.rebuild_all()?;
        }
        ClientType::Wsl(_) => {
            // Nothing to rebuild really, maybe just restart shell?
            // For now, no-op as WSL connection is just local process
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn cancel_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), String> {
    if let Some(transfer_state) = state
        .transfers
        .lock()
        .map_err(|e| e.to_string())?
        .get(&transfer_id)
    {
        transfer_state.cancel_flag.store(true, Ordering::Relaxed);

        // Update status immediately if possible
        let mut data = transfer_state.data.lock().map_err(|e| e.to_string())?;
        if data.status == "running" || data.status == "pending" {
            data.status = "cancelled".to_string();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn cancel_command_execution(
    state: State<'_, AppState>,
    command_id: String,
) -> Result<(), String> {
    let cancellations = state
        .command_cancellations
        .lock()
        .map_err(|e| e.to_string())?;
    if let Some(cancel_flag) = cancellations.get(&command_id) {
        cancel_flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_os_info(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let clients = state.clients.lock().map_err(|e| e.to_string())?;
    let client = clients.get(&id).ok_or("Session not found")?;
    Ok(client.os_info.clone().unwrap_or("Unknown".to_string()))
}
