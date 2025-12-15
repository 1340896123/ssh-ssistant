use super::client::{AppState, SshClient};
use super::manager::SshCommand;
use crate::ssh::ShellMsg;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn write_to_pty(
    state: State<'_, AppState>,
    id: String,
    data: String, // xterm sends string usually
) -> Result<(), String> {
    let clients = state.clients.lock().map_err(|e| e.to_string())?;
    let client = clients.get(&id).ok_or("Session not found")?;
    if let Some(tx) = &client.shell_tx {
        let _ = tx.send(ShellMsg::Data(data.into_bytes()));
    }
    Ok(())
}

#[tauri::command]
pub async fn write_binary_to_pty(
    state: State<'_, AppState>,
    id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let clients = state.clients.lock().map_err(|e| e.to_string())?;
    let client = clients.get(&id).ok_or("Session not found")?;
    if let Some(tx) = &client.shell_tx {
        let _ = tx.send(ShellMsg::Data(data));
    }
    Ok(())
}

#[tauri::command]
pub async fn resize_pty(
    state: State<'_, AppState>,
    id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let clients = state.clients.lock().map_err(|e| e.to_string())?;
    let client = clients.get(&id).ok_or("Session not found")?;
    if let Some(tx) = &client.shell_tx {
        let _ = tx.send(ShellMsg::Resize { rows, cols });
    }
    Ok(())
}

pub fn start_shell_thread(
    app: AppHandle,
    client: &mut SshClient,
    id: String,
) -> Result<Sender<ShellMsg>, String> {
    // Determine connection type
    match &client.client_type {
        crate::ssh::client::ClientType::Ssh(ssh_sender) => {
            let ssh_sender = ssh_sender.clone();
            let shell_id = id.clone();

            // 1. Create callback channel for data FROM SSH to UI
            let (callback_tx, callback_rx): (Sender<ShellMsg>, Receiver<ShellMsg>) = channel();

            // 2. Spawn thread to pump data from callback to UI
            let app_clone = app.clone();
            let shell_id_clone = shell_id.clone();
            thread::spawn(move || {
                while let Ok(msg) = callback_rx.recv() {
                    match msg {
                        ShellMsg::Data(d) => {
                            let _ = app_clone.emit(&format!("term-data:{}", shell_id_clone), d);
                        }
                        ShellMsg::Resize { .. } => {} // Incoming resize? Usually not relevant
                        ShellMsg::Exit => {
                            let _ = app_clone.emit(&format!("term-exit:{}", shell_id_clone), ());
                            break;
                        }
                    }
                }
            });

            // 3. Send ShellOpen command
            // xterm default size
            let _ = ssh_sender.send(SshCommand::ShellOpen {
                cols: 80,
                rows: 24,
                sender: callback_tx,
            });

            // 4. Create Adapter Channel for UI -> SSH
            let (ui_tx, ui_rx): (Sender<ShellMsg>, Receiver<ShellMsg>) = channel();

            // 5. Spawn adapter thread
            thread::spawn(move || {
                while let Ok(msg) = ui_rx.recv() {
                    match msg {
                        ShellMsg::Data(d) => {
                            let _ = ssh_sender.send(SshCommand::ShellWrite(d));
                        }
                        ShellMsg::Resize { rows, cols } => {
                            let _ = ssh_sender.send(SshCommand::ShellResize { rows, cols });
                        }
                        ShellMsg::Exit => {
                            let _ = ssh_sender.send(SshCommand::ShellClose);
                            break;
                        }
                    }
                }
            });

            Ok(ui_tx)
        }
        crate::ssh::client::ClientType::Wsl(distro) => {
            use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

            let (tx, rx): (Sender<ShellMsg>, Receiver<ShellMsg>) = channel();
            let shell_id = id.clone();

            // Setup PtySystem
            let pty_system = NativePtySystem::default();
            let pair = pty_system
                .openpty(PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| format!("Failed to open PTY: {}", e))?;

            // Spawn WSL
            let mut cmd = CommandBuilder::new("wsl");
            cmd.arg("-d");
            cmd.arg(distro);

            // Need to drop slave to close it in this process effectively?
            // portable-pty documentation suggests spawn_command takes generic command.
            let _child = pair
                .slave
                .spawn_command(cmd)
                .map_err(|e| format!("Failed to spawn WSL: {}", e))?;

            // Reader thread
            let mut reader = pair
                .master
                .try_clone_reader()
                .map_err(|e| format!("Failed to clone reader: {}", e))?;
            let app_clone = app.clone();
            let shell_id_read = shell_id.clone();

            thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            let _ = app_clone
                                .emit(&format!("term-data:{}", shell_id_read), buf[0..n].to_vec());
                        }
                        Ok(_) => break,
                        Err(_) => break,
                    }
                }
                let _ = app_clone.emit(&format!("term-exit:{}", shell_id_read), ());
            });

            // Writer thread (handle rx)
            let mut writer = pair
                .master
                .take_writer()
                .map_err(|e| format!("Failed to take writer: {}", e))?;
            let master = pair.master; // Move master here to keep it alive and for resize

            thread::spawn(move || {
                while let Ok(msg) = rx.recv() {
                    match msg {
                        ShellMsg::Data(d) => {
                            if let Err(e) = writer.write_all(&d) {
                                eprintln!("WSL Write Error: {}", e);
                                break;
                            }
                        }
                        ShellMsg::Resize { rows, cols } => {
                            if let Err(e) = master.resize(PtySize {
                                rows,
                                cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            }) {
                                eprintln!("WSL Resize Error: {}", e);
                            }
                        }
                        ShellMsg::Exit => {
                            break;
                        }
                    }
                }
            });

            Ok(tx)
        }
    }
}
