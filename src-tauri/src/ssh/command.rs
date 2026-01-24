use super::client::{AppState, ClientType};
use crate::ssh::{execute_ssh_operation, SshCommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
    let is_ai = tool_call_id_clone.is_some();

    let result = match &client.client_type {
        ClientType::Ssh(sender) => {
            let sender = sender.clone();
            let command = command.clone();
            let cancel_flag = cancel_flag.clone();

            execute_ssh_operation(move || {
                let (tx, rx) = std::sync::mpsc::channel();
                sender
                    .send(SshCommand::Exec {
                        command,
                        listener: tx,
                        cancel_flag,
                        is_ai,
                    })
                    .map_err(|e| format!("Failed to send command: {}", e))?;

                rx.recv()
                    .map_err(|_| "Failed to receive response from SSH Manager".to_string())?
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
        ClientType::Ssh(sender) => {
            let sender = sender.clone();
            execute_ssh_operation(move || {
                let (tx, rx) = std::sync::mpsc::channel();
                sender
                    .send(SshCommand::Exec {
                        command: "pwd".to_string(),
                        listener: tx,
                        cancel_flag: None,
                        is_ai: false,
                    })
                    .map_err(|e| format!("Failed to send command: {}", e))?;

                let working_dir = rx
                    .recv()
                    .map_err(|_| "Failed to receive response from SSH Manager".to_string())??;

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
