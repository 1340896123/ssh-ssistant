use super::client::{AppState, ClientType};
use crate::ssh::{
    emit_command_output, execute_ssh_operation, ExecStreamContext, ExecTarget, SshCommand,
};
use std::io::Read;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, State};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

enum WslStreamEvent {
    Chunk { data: String, stream: &'static str },
    Done,
}

fn spawn_pipe_reader<R>(mut reader: R, stream: &'static str, sender: Sender<WslStreamEvent>)
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                    if sender
                        .send(WslStreamEvent::Chunk {
                            data: chunk,
                            stream,
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                Err(_) => break,
            }
        }

        let _ = sender.send(WslStreamEvent::Done);
    });
}

#[tauri::command]
pub async fn exec_command(
    app_handle: AppHandle,
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
    let target = if tool_call_id_clone.is_some() {
        ExecTarget::Ai
    } else {
        ExecTarget::FileBrowser
    };
    let stream = tool_call_id_clone.as_ref().map(|cmd_id| ExecStreamContext {
        event_name: format!("command-output-{}-{}", id, cmd_id),
        app_handle: app_handle.clone(),
    });

    let result = match &client.client_type {
        ClientType::Ssh(senders) => {
            let sender = senders.ops.clone();
            let command = command.clone();
            let cancel_flag = cancel_flag.clone();
            let stream = stream.clone();

            execute_ssh_operation(move || {
                let (tx, rx) = std::sync::mpsc::channel();
                sender
                    .send(SshCommand::Exec {
                        command,
                        listener: tx,
                        cancel_flag,
                        target,
                        stream,
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
            let stream = stream.clone();

            tokio::task::spawn_blocking(move || {
                // Check cancellation before start
                if let Some(ref flag) = cancel_flag {
                    if flag.load(Ordering::Relaxed) {
                        return Err("Command cancelled by user".to_string());
                    }
                }

                let mut cmd = std::process::Command::new("wsl");
                #[cfg(target_os = "windows")]
                cmd.creation_flags(CREATE_NO_WINDOW);

                let mut child = cmd
                    .arg("-d")
                    .arg(&distro)
                    .arg("bash")
                    .arg("-c")
                    .arg(&command)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|e| e.to_string())?;

                let stdout = child
                    .stdout
                    .take()
                    .ok_or("Failed to capture WSL stdout".to_string())?;
                let stderr = child
                    .stderr
                    .take()
                    .ok_or("Failed to capture WSL stderr".to_string())?;

                let (tx, rx) = mpsc::channel();
                spawn_pipe_reader(stdout, "stdout", tx.clone());
                spawn_pipe_reader(stderr, "stderr", tx);

                let mut output = String::new();
                let mut completed_readers = 0;

                while completed_readers < 2 {
                    if let Some(ref flag) = cancel_flag {
                        if flag.load(Ordering::Relaxed) {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err("Command cancelled by user".to_string());
                        }
                    }

                    match rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(WslStreamEvent::Chunk {
                            data,
                            stream: stream_name,
                        }) => {
                            output.push_str(&data);
                            emit_command_output(stream.as_ref(), data, stream_name, false);
                        }
                        Ok(WslStreamEvent::Done) => {
                            completed_readers += 1;
                        }
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }

                let _status = child.wait().map_err(|e| e.to_string())?;
                emit_command_output(stream.as_ref(), String::new(), "stdout", true);
                Ok(output)
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
        ClientType::Ssh(senders) => {
            let sender = senders.ops.clone();
            execute_ssh_operation(move || {
                let (tx, rx) = std::sync::mpsc::channel();
                sender
                    .send(SshCommand::Exec {
                        command: "pwd".to_string(),
                        listener: tx,
                        cancel_flag: None,
                        target: ExecTarget::FileBrowser,
                        stream: None,
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
                let mut cmd = std::process::Command::new("wsl");
                #[cfg(target_os = "windows")]
                cmd.creation_flags(CREATE_NO_WINDOW);

                let output = cmd
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
