use super::client::{AppState, ClientType};
use crate::ssh::{execute_ssh_operation, ssh2_retry};
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
        let mut cancellations = state
            .command_cancellations
            .lock()
            .map_err(|e| e.to_string())?;
        cancellations.insert(cmd_id.clone(), flag.clone());
        Some(flag)
    } else {
        None
    };

    let tool_call_id_clone = tool_call_id.clone();

    let result = match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            let command = command.clone();
            let cancel_flag = cancel_flag.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;

                let sess = bg_session.lock().unwrap();
                let mut channel =
                    ssh2_retry(|| sess.channel_session()).map_err(|e| e.to_string())?;

                ssh2_retry(|| channel.exec(&command)).map_err(|e| e.to_string())?;

                let mut s = String::new();
                let mut buf = [0u8; 4096];

                loop {
                    // Check for cancellation
                    if let Some(ref flag) = cancel_flag {
                        if flag.load(Ordering::Relaxed) {
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
                        Err(e) => return Err(e.to_string()),
                    }
                }

                ssh2_retry(|| channel.wait_close())
                    .map_err(|e| format!("Failed to wait for channel close: {}", e))?;

                Ok(s)
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            let command = command.clone();
            let cancel_flag = cancel_flag.clone();

            tokio::task::spawn_blocking(move || {
                // Check cancellation before start
                if let Some(ref flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        return Err("Command cancelled by user".to_string());
                    }
                }

                let output = std::process::Command::new("wsl")
                    .arg("-d")
                    .arg(&distro)
                    .arg("bash")
                    .arg("-c")
                    .arg(&command)
                    .output()
                    .map_err(|e| e.to_string())?;

                // Check cancellation after (best effort)
                if let Some(ref flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        return Err("Command cancelled by user".to_string());
                    }
                }

                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    // Combine stdout and stderr for errors, or just stderr
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    let out = String::from_utf8_lossy(&output.stdout).to_string();
                    if err.is_empty() {
                        Ok(out)
                    } else {
                        Ok(out + &err)
                    }
                }
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    };

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

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;
                let sess = bg_session.lock().unwrap();
                let mut channel =
                    ssh2_retry(|| sess.channel_session()).map_err(|e| e.to_string())?;
                ssh2_retry(|| channel.exec("pwd")).map_err(|e| e.to_string())?;

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

                Ok(working_dir.trim().to_string())
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let output = std::process::Command::new("wsl")
                    .arg("-d")
                    .arg(&distro)
                    .arg("exec")
                    .arg("pwd")
                    .output()
                    .map_err(|e| e.to_string())?;

                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
}
