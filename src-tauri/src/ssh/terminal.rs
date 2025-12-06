use crate::ssh::{ShellMsg, ssh2_retry};
use std::io::{ErrorKind, Read, Write};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use super::client::{AppState, SshClient};

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
    // 在主会话上启动shell线程
    let (tx, rx): (Sender<ShellMsg>, Receiver<ShellMsg>) = channel();

    // Get main session
    let main_session = client.ssh_pool.get_main_session();
    let shell_sess = main_session.clone();
    let shell_id = id.clone();

    thread::spawn(move || {
        // Wait for frontend to be ready
        thread::sleep(Duration::from_millis(500));

        // Macro to retry operations on EAGAIN
        macro_rules! retry {
            ($e:expr) => {
                ssh2_retry(|| $e)
            };
        }

        let mut channel = match retry!({
            let sess_lock = shell_sess.lock().unwrap();
            sess_lock.channel_session()
        }) {
            Ok(channel) => channel,
            Err(e) => {
                eprintln!("Failed to create channel: {}", e);
                let _ = app.emit(&format!("term-exit:{}", shell_id), ());
                return;
            }
        };

        // Apply retry! to request_pty as it might return WouldBlock
        if let Err(e) = retry!(channel.request_pty("xterm", None, Some((80, 24, 0, 0)))) {
            eprintln!("Failed to request PTY: {}", e);
            let _ = app.emit(&format!("term-exit:{}", shell_id), ());
            return;
        }

        // Apply retry! to shell request as well
        if let Err(e) = retry!(channel.shell()) {
            eprintln!("Failed to start shell: {}", e);
            let _ = app.emit(&format!("term-exit:{}", shell_id), ());
            return;
        }

        let mut buf = [0u8; 4096];
        loop {
            // 1. Read from SSH
            let read_result = channel.read(&mut buf);
            match read_result {
                Ok(n) if n > 0 => {
                    let data = buf[0..n].to_vec();
                    let _ = app.emit(&format!("term-data:{}", shell_id), data);
                }
                Ok(_) => {
                    // EOF
                    break;
                }
                Err(e) => {
                    if e.kind() != ErrorKind::WouldBlock {
                        eprintln!("SSH Read Error: {}", e);
                        break;
                    }
                }
            }

            // 2. Process incoming messages
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ShellMsg::Data(d) => {
                        let _ = channel.write_all(&d);
                    }
                    ShellMsg::Resize { rows, cols } => {
                        let _ = retry!(channel.request_pty_size(cols.into(), rows.into(), None, None));
                    }
                    ShellMsg::Exit => return,
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
        let _ = app.emit(&format!("term-exit:{}", shell_id), ());
    });

    Ok(tx)
}