use crate::models::Connection as SshConnConfig;
use super::connection::SessionSshPool;
use crate::ssh::{ShellMsg, execute_ssh_operation};
use super::terminal::start_shell_thread;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, State};
use uuid::Uuid;

#[derive(Clone)]
pub struct SshClient {
    pub ssh_pool: Arc<SessionSshPool>,                 // SSH连接池
    pub shell_tx: Option<Sender<ShellMsg>>,            // 终端消息通道
    pub owner_cache: Arc<Mutex<HashMap<u32, String>>>, // UID缓存
    pub shutdown_signal: Arc<AtomicBool>,              // 用于通知后台监控任务停止
    pub os_info: Option<String>,                       // Remote OS information
}

pub struct AppState {
    pub clients: Mutex<HashMap<String, SshClient>>,
    pub transfers: Mutex<HashMap<String, Arc<AtomicBool>>>, // ID -> CancelFlag
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
pub async fn test_connection(config: SshConnConfig) -> Result<String, String> {
    execute_ssh_operation(move || {
        let session = super::connection::establish_connection_with_retry(&config)?;
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
    // 获取SSH池设置
    let max_bg_sessions = {
        let settings =
            crate::db::get_settings(app.clone()).unwrap_or_else(|_| crate::models::AppSettings {
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
                    max_background_sessions: 3,
                    enable_auto_cleanup: true,
                    cleanup_interval_minutes: 5,
                },
            });
        settings.ssh_pool.max_background_sessions as usize
    };

    // Create SSH connection pool in a blocking task to avoid blocking the async runtime
    let config_clone = config.clone();
    let ssh_pool = tokio::task::spawn_blocking(move || {
        SessionSshPool::new(config_clone, max_bg_sessions)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    ?;

    let main_session = ssh_pool.get_main_session();

    // 启动定时清理任务
    let cleanup_pool = ssh_pool.clone();
    let shutdown_signal = Arc::new(AtomicBool::new(false));
    let monitor_signal = shutdown_signal.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5分钟
        loop {
            interval.tick().await;
            // 关键修复：检查停止信号
            if monitor_signal.load(Ordering::Relaxed) {
                break;
            }
            cleanup_pool.cleanup_disconnected();
        }
    });

    let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

    // If reconnecting (ID provided), cleanup old session first
    {
        let mut clients = state.clients.lock().map_err(|e| e.to_string())?;
        if let Some(client) = clients.remove(&id) {
            if let Some(tx) = client.shell_tx {
                let _ = tx.send(ShellMsg::Exit);
            }
            client.ssh_pool.close_all();
        }
    }

    // Use OS type from connection config with fallback to Linux for backward compatibility
    let os_info = config
        .os_type
        .clone()
        .unwrap_or_else(|| "Linux".to_string());
    println!("Using OS type from config: {}", os_info);

    // Create mutable client reference for terminal initialization
    let mut client = SshClient {
        ssh_pool: Arc::new(ssh_pool),
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

        // 3. 关闭所有 SSH 连接
        client.ssh_pool.close_all();
    }

    Ok(())
}

#[tauri::command]
pub async fn cleanup_and_reconnect(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    // 关闭所有现有连接
    client.ssh_pool.close_all();

    // 重建所有连接
    client.ssh_pool.rebuild_all()?;

    Ok(())
}

#[tauri::command]
pub async fn cancel_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), String> {
    if let Some(flag) = state
        .transfers
        .lock()
        .map_err(|e| e.to_string())?
        .get(&transfer_id)
    {
        flag.store(true, Ordering::Relaxed);
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