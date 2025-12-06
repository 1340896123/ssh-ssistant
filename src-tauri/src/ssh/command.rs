use crate::ssh::{ssh2_retry, execute_ssh_operation};
use super::client::{AppState};
use std::io::{ErrorKind, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn exec_command(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
    command: String,
    tool_call_id: Option<String>,
) -> Result<String, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    // Setup cancellation if tool_call_id is provided
    let cancel_flag = if let Some(ref cmd_id) = tool_call_id {
        let flag = Arc::new(AtomicBool::new(false));
        let mut cancellations = state.command_cancellations.lock().map_err(|e| e.to_string())?;
        cancellations.insert(cmd_id.clone(), flag.clone());
        Some(flag)
    } else {
        None
    };

    let tool_call_id_clone = tool_call_id.clone();

    let result = execute_ssh_operation(move || {
        // 使用后台会话执行命令，避免阻塞SFTP操作
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;

        // 创建channel
        let sess = bg_session.lock().unwrap();
        let mut channel = ssh2_retry(|| sess.channel_session())
            .map_err(|e| e.to_string())?;

        ssh2_retry(|| channel.exec(&command))
            .map_err(|e| e.to_string())?;

        let mut s = String::new();
        let mut buf = [0u8; 4096]; // Increased buffer size

        loop {
            // Check for cancellation
            if let Some(ref flag) = cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    // Try to close channel gracefully first
                    let _ = channel.close();
                    return Err("Command cancelled by user".to_string());
                }
            }

            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    s.push_str(&chunk);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }

        // Wait for close, but check cancellation during wait if possible
        // Since we can't easily inject into ssh2_retry, we rely on the fact
        // that we only reach here if we got EOF (Ok(0)).
        ssh2_retry(|| channel.wait_close())
            .map_err(|e| format!("Failed to wait for channel close: {}", e))?;

        Ok(s)
    }).await;

    // Cleanup cancellation flag
    if let Some(cmd_id) = tool_call_id_clone {
         if let Ok(mut cancellations) = state.command_cancellations.lock() {
             cancellations.remove(&cmd_id);
         }
    }

    result
}

#[tauri::command]
pub async fn get_working_directory(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {

        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let mut channel = ssh2_retry(|| sess.channel_session())
            .map_err(|e| e.to_string())?;
        ssh2_retry(|| channel.exec("pwd"))
            .map_err(|e| e.to_string())?;

        let mut working_dir = String::new();
        let mut buf = [0u8; 1024];
        loop {
            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => working_dir.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        ssh2_retry(|| channel.wait_close()).ok();

        // 清理换行符和空白字符
        Ok(working_dir.trim().to_string())
    }).await
}