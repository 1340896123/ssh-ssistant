use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use std::thread;
use std::io::{Read, Write, ErrorKind};
use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{channel, Sender, Receiver};
use ssh2::Session;
use tauri::{State, AppHandle, Emitter};
use uuid::Uuid;
use crate::models::{Connection as SshConnConfig, FileEntry};

enum ShellMsg {
    Data(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Exit,
}

pub struct SshClient {
    session: Arc<Mutex<Session>>,
    shell_tx: Option<Sender<ShellMsg>>,
}

pub struct AppState {
    pub clients: Mutex<HashMap<String, SshClient>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }
}

// Helper to retry ssh2 operations that might return EAGAIN/WouldBlock
fn block_on<F, T>(mut f: F) -> Result<T, ssh2::Error>
where
    F: FnMut() -> Result<T, ssh2::Error>,
{
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                if e.code() == ssh2::ErrorCode::Session(-37) {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                return Err(e);
            }
        }
    }
}

#[tauri::command]
pub async fn connect(
    app: AppHandle,
    state: State<'_, AppState>,
    config: SshConnConfig,
) -> Result<String, String> {
    let addr_str = format!("{}:{}", config.host, config.port);
    let addr = addr_str
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("Invalid address")?;

    let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| format!("Connection failed: {}", e))?;
    
    let mut sess = Session::new().map_err(|e| e.to_string())?;
    sess.set_tcp_stream(tcp);
    sess.handshake().map_err(|e| e.to_string())?;
    
    sess.userauth_password(&config.username, config.password.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;

    // Set non-blocking mode for concurrency
    sess.set_blocking(false);
        
    let id = Uuid::new_v4().to_string();
    let sess = Arc::new(Mutex::new(sess));
    
    // Spawn shell thread
    let (tx, rx): (Sender<ShellMsg>, Receiver<ShellMsg>) = channel();
    let shell_sess = sess.clone();
    let shell_id = id.clone();
    
    thread::spawn(move || {
        // Macro to retry operations on EAGAIN
        macro_rules! retry {
            ($e:expr) => {
                loop {
                    match $e {
                        Ok(res) => break res,
                        Err(e) => {
                            if e.code() == ssh2::ErrorCode::Session(-37) {
                                thread::sleep(Duration::from_millis(10));
                                continue;
                            }
                            eprintln!("SSH Op error: {}", e);
                            return;
                        }
                    }
                }
            }
        }

        let mut channel = retry!({
            let sess_lock = shell_sess.lock().unwrap();
            sess_lock.channel_session()
        });

        retry!(channel.request_pty("xterm", None, Some((80, 24, 0, 0))));
        retry!(channel.shell());

        let mut buf = [0u8; 4096];
        loop {
            // 1. Read from SSH
            let read_result = channel.read(&mut buf);
            match read_result {
                Ok(n) if n > 0 => {
                    let data = buf[0..n].to_vec();
                    let _ = app.emit(&format!("term-data://{}", shell_id), data);
                },
                Ok(_) => {
                    // EOF
                    break;
                },
                Err(e) => {
                    if e.kind() != ErrorKind::WouldBlock {
                        break;
                    }
                }
            }

            // 2. Process incoming messages
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ShellMsg::Data(d) => {
                        let _ = channel.write_all(&d);
                    },
                    ShellMsg::Resize { rows, cols } => {
                        let _ = channel.request_pty_size(cols.into(), rows.into(), None, None);
                    },
                    ShellMsg::Exit => return,
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
        let _ = app.emit(&format!("term-exit://{}", shell_id), ());
    });
    
    let client = SshClient {
        session: sess,
        shell_tx: Some(tx),
    };

    state.clients.lock().map_err(|e| e.to_string())?.insert(id.clone(), client);
    
    Ok(id)
}

#[tauri::command]
pub async fn disconnect(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    if let Some(client) = state.clients.lock().map_err(|e| e.to_string())?.remove(&id) {
        if let Some(tx) = client.shell_tx {
            let _ = tx.send(ShellMsg::Exit);
        }
    }
    Ok(())
}

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

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    let path_path = std::path::Path::new(&path);
    let files = block_on(|| sftp.readdir(path_path)).map_err(|e| e.to_string())?;
    
    let mut entries = Vec::new();
    for (path_buf, stat) in files {
        if let Some(name) = path_buf.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str == "." || name_str == ".." {
                    continue;
                }
                entries.push(FileEntry {
                    name: name_str.to_string(),
                    is_dir: stat.is_dir(),
                    size: stat.size.unwrap_or(0),
                    mtime: stat.mtime.unwrap_or(0) as i64,
                    permissions: stat.perm.unwrap_or(0),
                });
            }
        }
    }
    
    entries.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.cmp(&b.name)
        } else {
            b.is_dir.cmp(&a.is_dir)
        }
    });
    Ok(entries)
}

#[tauri::command]
pub async fn create_directory(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    block_on(|| sftp.mkdir(std::path::Path::new(&path), 0o755)).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn create_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    block_on(|| sftp.create(std::path::Path::new(&path))).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_item(
    state: State<'_, AppState>,
    id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    
    if is_dir {
        block_on(|| sftp.rmdir(std::path::Path::new(&path))).map_err(|e| e.to_string())?;
    } else {
        block_on(|| sftp.unlink(std::path::Path::new(&path))).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_item(
    state: State<'_, AppState>,
    id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    block_on(|| sftp.rename(std::path::Path::new(&old_path), std::path::Path::new(&new_path), None))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn download_file(
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    let mut remote_file = block_on(|| sftp.open(std::path::Path::new(&remote_path))).map_err(|e| e.to_string())?;
    let mut local_file = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;
    
    let mut buf = [0u8; 32768];
    loop {
        match remote_file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                local_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            },
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            },
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn upload_file(
    state: State<'_, AppState>,
    id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    let mut local_file = std::fs::File::open(&local_path).map_err(|e| e.to_string())?;
    let mut remote_file = block_on(|| sftp.create(std::path::Path::new(&remote_path))).map_err(|e| e.to_string())?;
    
    let mut buf = [0u8; 32768];
    loop {
        match local_file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let mut pos = 0;
                while pos < n {
                    match remote_file.write(&buf[pos..n]) {
                        Ok(written) => pos += written,
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        },
                        Err(e) => return Err(e.to_string()),
                    }
                }
            },
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn download_temp_and_open(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    remote_name: String,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let temp_dir = std::env::temp_dir();
    let local_path = temp_dir.join(&remote_name);
    let local_path_str = local_path.to_str().ok_or("Invalid path")?.to_string();

    {
        let sess = client_sess.lock().map_err(|e| e.to_string())?;
        let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
        let mut remote_file = block_on(|| sftp.open(std::path::Path::new(&remote_path))).map_err(|e| e.to_string())?;
        let mut local_file = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;
        
        let mut buf = [0u8; 32768];
        loop {
            match remote_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    local_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                },
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    use tauri_plugin_opener::OpenerExt;
    app.opener().open_path(local_path_str, None::<String>).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn exec_command(
    state: State<'_, AppState>,
    id: String,
    command: String,
) -> Result<String, String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let mut channel = block_on(|| sess.channel_session()).map_err(|e| e.to_string())?;
    block_on(|| channel.exec(&command)).map_err(|e| e.to_string())?;
    
    let mut s = String::new();
    let mut buf = [0u8; 1024];
    loop {
        match channel.read(&mut buf) {
             Ok(0) => break,
             Ok(n) => s.push_str(&String::from_utf8_lossy(&buf[..n])),
             Err(e) if e.kind() == ErrorKind::WouldBlock => {
                 thread::sleep(Duration::from_millis(10));
             },
             Err(e) => return Err(e.to_string()),
        }
    }
    block_on(|| channel.wait_close()).ok();
    Ok(s)
}
