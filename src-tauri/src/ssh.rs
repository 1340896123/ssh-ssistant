use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::time::Duration;
use std::thread;
use std::io::{Read, Write, ErrorKind, Seek, SeekFrom};
use std::sync::mpsc::{channel, Sender, Receiver};
use ssh2::Session;
use tauri::{State, AppHandle, Emitter};
use uuid::Uuid;
use crate::models::{Connection as SshConnConfig, FileEntry};
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, EventKind};
use std::path::Path;
use sha2::{Sha256, Digest};
use hex;

#[derive(Clone)]
enum ShellMsg {
    Data(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Exit,
}

#[derive(Clone)]
pub struct SshClient {
    session: Arc<Mutex<Session>>,
    shell_tx: Option<Sender<ShellMsg>>,
    owner_cache: Arc<Mutex<HashMap<u32, String>>>,
}

pub struct AppState {
    pub clients: Mutex<HashMap<String, SshClient>>,
    pub transfers: Mutex<HashMap<String, Arc<AtomicBool>>>, // ID -> CancelFlag
    pub watchers: Mutex<HashMap<String, RecommendedWatcher>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            transfers: Mutex::new(HashMap::new()),
            watchers: Mutex::new(HashMap::new()),
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
    // Determine the underlying TCP stream (either direct or via jump host)
    let tcp_stream = if let Some(jump_host) = &config.jump_host {
        if jump_host.trim().is_empty() {
             // Fallback to direct if empty
             let addr_str = format!("{}:{}", config.host, config.port);
             let addr = addr_str.to_socket_addrs().map_err(|e| e.to_string())?
                .next().ok_or("Invalid address")?;
             TcpStream::connect_timeout(&addr, Duration::from_secs(5))
                .map_err(|e| format!("Connection failed: {}", e))?
        } else {
            // Jump Host Logic
            let jump_port = config.jump_port.unwrap_or(22);
            let jump_addr = format!("{}:{}", jump_host, jump_port);
            let jump_tcp = TcpStream::connect(&jump_addr)
                .map_err(|e| format!("Jump host connection failed: {}", e))?;

            let mut jump_sess = Session::new().map_err(|e| e.to_string())?;
            jump_sess.set_tcp_stream(jump_tcp);
            jump_sess.handshake().map_err(|e| format!("Jump handshake failed: {}", e))?;
            jump_sess.set_keepalive(true, 60);
            
            jump_sess.userauth_password(
                config.jump_username.as_deref().unwrap_or(""), 
                config.jump_password.as_deref().unwrap_or("")
            ).map_err(|e| format!("Jump auth failed: {}", e))?;

            // Setup local listener for forwarding
            let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
            let local_port = listener.local_addr().map_err(|e| e.to_string())?.port();
            
            let target_host = config.host.clone();
            let target_port = config.port;

            // Spawn proxy thread
            thread::spawn(move || {
                if let Ok((mut local_stream, _)) = listener.accept() {
                    let _ = local_stream.set_nonblocking(true);
                    
                    // Keep blocking for channel creation to ensure it doesn't fail with WouldBlock
                    match jump_sess.channel_direct_tcpip(&target_host, target_port, None) {
                        Ok(mut channel) => {
                            let _ = jump_sess.set_blocking(false);
                            let mut buf = [0u8; 8192];
                            loop {
                                let mut did_work = false;
                                
                                // Local -> Remote
                                match local_stream.read(&mut buf) {
                                    Ok(0) => break, // EOF
                                    Ok(n) => {
                                        let mut total_written = 0;
                                        let mut failed = false;
                                        while total_written < n {
                                            match channel.write(&buf[total_written..n]) {
                                                Ok(w) => total_written += w,
                                                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                                    thread::sleep(Duration::from_millis(1));
                                                },
                                                Err(_) => { failed = true; break; }
                                            }
                                        }
                                        if failed { break; }
                                        did_work = true;
                                    },
                                    Err(e) if e.kind() == ErrorKind::WouldBlock => {},
                                    Err(_) => break,
                                }

                                // Remote -> Local
                                match channel.read(&mut buf) {
                                    Ok(0) => break, // EOF from remote
                                    Ok(n) => {
                                        let mut total_written = 0;
                                        let mut failed = false;
                                        while total_written < n {
                                            match local_stream.write(&buf[total_written..n]) {
                                                Ok(w) => total_written += w,
                                                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                                    thread::sleep(Duration::from_millis(1));
                                                },
                                                Err(_) => { failed = true; break; }
                                            }
                                        }
                                        if failed { break; }
                                        did_work = true;
                                    }, 
                                    Err(e) if e.kind() == ErrorKind::WouldBlock => {},
                                    Err(_) => break,
                                }

                                if !did_work {
                                    thread::sleep(Duration::from_millis(1));
                                }
                            }
                        },
                        Err(e) => eprintln!("Failed to open direct-tcpip channel: {}", e),
                    }
                }
            });

            TcpStream::connect(format!("127.0.0.1:{}", local_port))
                .map_err(|e| format!("Failed to connect to local proxy: {}", e))?
        }
    } else {
        let addr_str = format!("{}:{}", config.host, config.port);
        let addr = addr_str
            .to_socket_addrs()
            .map_err(|e| e.to_string())?
            .next()
            .ok_or("Invalid address")?;
        TcpStream::connect_timeout(&addr, Duration::from_secs(5))
            .map_err(|e| format!("Connection failed: {}", e))?
    };
    
    let mut sess = Session::new().map_err(|e| e.to_string())?;
    sess.set_tcp_stream(tcp_stream);
    sess.handshake().map_err(|e| e.to_string())?;
    
    sess.userauth_password(&config.username, config.password.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;

    // Enable keepalive to avoid idle disconnects
    sess.set_keepalive(true, 30);

    // Set non-blocking mode for concurrency
    sess.set_blocking(false);
        
    let id = Uuid::new_v4().to_string();
    let sess = Arc::new(Mutex::new(sess));
    
    // Spawn shell thread
    let (tx, rx): (Sender<ShellMsg>, Receiver<ShellMsg>) = channel();
    let shell_sess = sess.clone();
    let shell_id = id.clone();
    
    thread::spawn(move || {
        // Wait for frontend to be ready
        thread::sleep(Duration::from_millis(500));

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
                    let _ = app.emit(&format!("term-data:{}", shell_id), data);
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
        let _ = app.emit(&format!("term-exit:{}", shell_id), ());
    });
    
    let client = SshClient {
        session: sess,
        shell_tx: Some(tx),
        owner_cache: Arc::new(Mutex::new(HashMap::new())),
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

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let (client_sess, owner_cache) = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        (client.session.clone(), client.owner_cache.clone())
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
                let uid = stat.uid.unwrap_or(0);
                // Resolve UID to username with per-session cache
                let owner = {
                    if let Ok(mut cache) = owner_cache.lock() {
                        if let Some(cached) = cache.get(&uid) {
                            cached.clone()
                        } else {
                            // Try to resolve via remote command: id -nu <uid>
                            let username = {
                                let mut name = uid.to_string();
                                if let Ok(mut channel) = sess.channel_session() {
                                    let cmd = format!("id -nu {}", uid);
                                    if channel.exec(&cmd).is_ok() {
                                        let mut buf = Vec::new();
                                        if channel.read_to_end(&mut buf).is_ok() {
                                            if let Ok(s) = String::from_utf8(buf) {
                                                let trimmed = s.trim();
                                                if !trimmed.is_empty() {
                                                    name = trimmed.to_string();
                                                }
                                            }
                                        }
                                        let _ = channel.wait_close();
                                    }
                                }
                                name
                            };
                            cache.insert(uid, username.clone());
                            username
                        }
                    } else {
                        uid.to_string()
                    }
                };
                entries.push(FileEntry {
                    name: name_str.to_string(),
                    is_dir: stat.is_dir(),
                    size: stat.size.unwrap_or(0),
                    mtime: stat.mtime.unwrap_or(0) as i64,
                    permissions: stat.perm.unwrap_or(0),
                    uid,
                    owner,
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

fn rm_recursive(sftp: &ssh2::Sftp, path: &std::path::Path) -> Result<(), String> {
    let files = block_on(|| sftp.readdir(path)).map_err(|e| e.to_string())?;
    
    for (path_buf, stat) in files {
        if let Some(name) = path_buf.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str == "." || name_str == ".." {
                    continue;
                }
                
                let full_path = path.join(name);

                if stat.is_dir() {
                    rm_recursive(sftp, &full_path)?;
                } else {
                    block_on(|| sftp.unlink(&full_path)).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    
    block_on(|| sftp.rmdir(path)).map_err(|e| e.to_string())?;
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
        rm_recursive(&sftp, std::path::Path::new(&path))?;
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
pub async fn change_file_permission(
    state: State<'_, AppState>,
    id: String,
    path: String,
    perms: String,
) -> Result<(), String> {
    let command = format!("chmod {} \"{}\"", perms, path);
    exec_command(state, id, command).await.map(|_| ())
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

fn upload_recursive(
    sftp: &ssh2::Sftp,
    local_path: &std::path::Path,
    remote_path: &str,
) -> Result<(), String> {
    if local_path.is_dir() {
        // Create remote directory
        // Ignore error if it already exists
        let _ = block_on(|| sftp.mkdir(std::path::Path::new(remote_path), 0o755));

        for entry in std::fs::read_dir(local_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = path.file_name().ok_or("Invalid file name")?.to_string_lossy();
            // Ensure forward slashes for remote path
            let new_remote = if remote_path.ends_with('/') {
                format!("{}{}", remote_path, name)
            } else {
                format!("{}/{}", remote_path, name)
            };
            upload_recursive(sftp, &path, &new_remote)?;
        }
    } else {
        let mut local_file = std::fs::File::open(local_path).map_err(|e| e.to_string())?;
        let mut remote_file = block_on(|| sftp.create(std::path::Path::new(remote_path))).map_err(|e| e.to_string())?;
        
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
    
    upload_recursive(&sftp, std::path::Path::new(&local_path), &remote_path)?;
    
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
pub async fn cancel_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), String> {
    if let Some(flag) = state.transfers.lock().map_err(|e| e.to_string())?.get(&transfer_id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    id: String,
    transferred: u64,
    total: u64,
}

fn get_remote_file_hash(sess: &Session, path: &str) -> Result<Option<String>, String> {
    let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
    // Try sha256sum first
    let cmd = format!("sha256sum '{}'", path);
    channel.exec(&cmd).map_err(|e| e.to_string())?;
    
    let mut s = String::new();
    channel.read_to_string(&mut s).map_err(|e| e.to_string())?;
    channel.wait_close().map_err(|e| e.to_string())?;

    if channel.exit_status().unwrap_or(-1) == 0 {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if let Some(hash) = parts.get(0) {
            return Ok(Some(hash.to_string()));
        }
    }
    
    // Fallback to md5sum
    let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
    let cmd = format!("md5sum '{}'", path);
    channel.exec(&cmd).map_err(|e| e.to_string())?;
    
    let mut s = String::new();
    channel.read_to_string(&mut s).map_err(|e| e.to_string())?;
    channel.wait_close().map_err(|e| e.to_string())?;

    if channel.exit_status().unwrap_or(-1) == 0 {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if let Some(hash) = parts.get(0) {
            return Ok(Some(hash.to_string()));
        }
    }

    Ok(None)
}

fn compute_local_file_hash(path: &std::path::Path, limit: u64) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    let mut read = 0u64;
    
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        
        let to_hash = if read + (n as u64) > limit {
            (limit - read) as usize
        } else {
            n
        };
        
        hasher.update(&buf[..to_hash]);
        read += to_hash as u64;
        
        if read >= limit { break; }
    }
    
    Ok(hex::encode(hasher.finalize()))
}

fn get_dir_size(path: &std::path::Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    size += get_dir_size(&entry.path());
                } else {
                    size += meta.len();
                }
            }
        }
    }
    size
}

fn upload_recursive_progress(
    sess: &Session,
    sftp: &ssh2::Sftp,
    local_path: &std::path::Path,
    remote_path: &str,
    cancel_flag: &AtomicBool,
    app: &AppHandle,
    transfer_id: &str,
    total_size: u64,
    transferred: &mut u64,
    resume: bool,
) -> Result<(), String> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Cancelled".to_string());
    }

    if local_path.is_dir() {
        let _ = block_on(|| sftp.mkdir(std::path::Path::new(remote_path), 0o755));
        
        for entry in std::fs::read_dir(local_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let name = path.file_name().ok_or("Invalid file name")?.to_string_lossy();
            // Always use forward slashes for remote SFTP paths
            let new_remote = format!("{}/{}", remote_path.trim_end_matches('/'), name);
            
            upload_recursive_progress(sess, sftp, &path, &new_remote, cancel_flag, app, transfer_id, total_size, transferred, resume)?;
        }
    } else {
        let mut local_file = std::fs::File::open(local_path).map_err(|e| e.to_string())?;
        let file_size = local_file.metadata().map_err(|e| e.to_string())?.len();
        
        let mut offset = 0;
        let mut remote_file;
        
        if resume {
            // Check remote file size
            let remote_path_path = std::path::Path::new(remote_path);
             match block_on(|| sftp.stat(remote_path_path)) {
                Ok(attrs) => {
                    if let Some(size) = attrs.size {
                        if size >= file_size {
                            // Already uploaded
                            *transferred += file_size;
                             let _ = app.emit("transfer-progress", ProgressPayload {
                                id: transfer_id.to_string(),
                                transferred: *transferred,
                                total: total_size,
                            });
                            return Ok(());
                        }
                        
                        // Verify Checksum before resuming
                        let local_hash = compute_local_file_hash(local_path, size)?;
                        
                        // Use passed session
                        if let Ok(Some(remote_hash)) = get_remote_file_hash(sess, remote_path) {
                            if remote_hash.len() == 64 && local_hash == remote_hash {
                                offset = size;
                            } else if remote_hash.len() == 32 {
                                // Compute MD5 locally
                                use md5::Md5;
                                let mut hasher = Md5::new();
                                let mut file = std::fs::File::open(local_path).map_err(|e| e.to_string())?;
                                let mut buf = [0u8; 8192];
                                let mut read = 0u64;
                                loop {
                                    let n = file.read(&mut buf).map_err(|e| e.to_string())?;
                                    if n == 0 { break; }
                                    let to_hash = if read + (n as u64) > size { (size - read) as usize } else { n };
                                    hasher.update(&buf[..to_hash]);
                                    read += to_hash as u64;
                                    if read >= size { break; }
                                }
                                let local_md5 = hex::encode(hasher.finalize());
                                if local_md5 == remote_hash {
                                    offset = size;
                                } else {
                                    offset = 0;
                                }
                            } else {
                                offset = 0;
                            }
                        } else {
                            offset = 0; 
                        }
                    }
                },
                Err(_) => {} // File doesn't exist, start from 0
            }
        }

        if offset > 0 {
            // Resume: open for write and append? SFTP doesn't have O_APPEND exactly like POSIX, 
            // but we can write to offset.
            // ssh2 open_mode: (filename, flags, mode, open_type)
            // flags: Write | Read?
            // We use Write.
            remote_file = block_on(|| sftp.open_mode(
                std::path::Path::new(remote_path), 
                ssh2::OpenFlags::WRITE, 
                0o644, 
                ssh2::OpenType::File
            )).map_err(|e| e.to_string())?;
            
            // Seek local
            local_file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
            
            // For remote, sftp file implies we seek before write? 
            // ssh2::File doesn't have seek. It has write.
            // Wait, ssh2::File implements Write.
            // But SFTP write packet includes offset.
            // ssh2 crate handles offset internally if we just write? 
            // No, if we open a file, the offset starts at 0 usually unless we seek.
            // Does ssh2::File have seek?
            // It implements std::io::Seek? 
            // Let's check imports. I imported Seek.
            // If ssh2::File implements Seek, we are good.
            // If not, we might be in trouble.
            // Assuming it does or we just recreate it.
            // Actually, if `resume` logic is complex with ssh2, we might just overwrite for now or use a simpler logic.
            // But user asked for "Continue".
            // Let's try to seek. If it fails to compile, I'll fix it.
            // ssh2::File implements Seek.
            use std::io::Seek;
            remote_file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;

        } else {
            remote_file = block_on(|| sftp.create(std::path::Path::new(remote_path))).map_err(|e| e.to_string())?;
        }
        
        // Update global transferred count with skipped bytes
        *transferred += offset;

        let mut buf = [0u8; 32768];
        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err("Cancelled".to_string());
            }
            
            match local_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let mut pos = 0;
                    while pos < n {
                         match remote_file.write(&buf[pos..n]) {
                            Ok(written) => {
                                pos += written;
                                *transferred += written as u64;
                                // Emit progress periodically? Or every chunk?
                                // 32KB is small. Maybe every 1MB or just every chunk.
                                // Emitting every chunk might be too much for frontend IPC?
                                // Let's try every chunk for smoothness, Tauri is fast.
                                let _ = app.emit("transfer-progress", ProgressPayload {
                                    id: transfer_id.to_string(),
                                    transferred: *transferred,
                                    total: total_size,
                                });
                            },
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
    }
    Ok(())
}

#[tauri::command]
pub async fn upload_file_with_progress(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    transfer_id: String,
    local_path: String,
    remote_path: String,
    resume: bool,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };
    
    let cancel_flag = Arc::new(AtomicBool::new(false));
    state.transfers.lock().map_err(|e| e.to_string())?.insert(transfer_id.clone(), cancel_flag.clone());

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    
    let local_p = std::path::Path::new(&local_path);
    let total_size = if local_p.is_dir() {
        get_dir_size(local_p)
    } else {
        local_p.metadata().map_err(|e| e.to_string())?.len()
    };
    
    let mut transferred = 0;
    
    let res = upload_recursive_progress(&sess, &sftp, local_p, &remote_path, &cancel_flag, &app, &transfer_id, total_size, &mut transferred, resume);
    
    state.transfers.lock().map_err(|e| e.to_string())?.remove(&transfer_id);
    
    res
}

#[tauri::command]
pub async fn download_file_with_progress(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    transfer_id: String,
    remote_path: String,
    local_path: String,
    resume: bool,
) -> Result<(), String> {
    let client_sess = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.session.clone()
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    state.transfers.lock().map_err(|e| e.to_string())?.insert(transfer_id.clone(), cancel_flag.clone());

    let sess = client_sess.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    
    // Get remote size
    let remote_path_path = std::path::Path::new(&remote_path);
    let total_size = block_on(|| sftp.stat(remote_path_path)).map_err(|e| e.to_string())?
        .size.ok_or("Unknown size")?;

    let mut offset = 0;
    let mut local_file;
    
    if resume {
        if let Ok(meta) = std::fs::metadata(&local_path) {
            let local_size = meta.len();
            if local_size < total_size {
                offset = local_size;
                local_file = std::fs::OpenOptions::new().write(true).append(true).open(&local_path).map_err(|e| e.to_string())?;
            } else {
                // Already done
                let _ = app.emit("transfer-progress", ProgressPayload {
                    id: transfer_id.clone(),
                    transferred: total_size,
                    total: total_size,
                });
                state.transfers.lock().map_err(|e| e.to_string())?.remove(&transfer_id);
                return Ok(());
            }
        } else {
             local_file = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;
        }
    } else {
        local_file = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;
    }
    
    let mut remote_file = block_on(|| sftp.open(remote_path_path)).map_err(|e| e.to_string())?;
    if offset > 0 {
        remote_file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    }

    let mut transferred = offset;
    let mut buf = [0u8; 32768];
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            state.transfers.lock().map_err(|e| e.to_string())?.remove(&transfer_id);
            return Err("Cancelled".to_string());
        }

        match remote_file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                local_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                transferred += n as u64;
                let _ = app.emit("transfer-progress", ProgressPayload {
                    id: transfer_id.clone(),
                    transferred,
                    total: total_size,
                });
            },
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            },
            Err(e) => {
                state.transfers.lock().map_err(|e| e.to_string())?.remove(&transfer_id);
                return Err(e.to_string());
            }
        }
    }
    
    state.transfers.lock().map_err(|e| e.to_string())?.remove(&transfer_id);
    Ok(())
}

#[tauri::command]
pub async fn edit_remote_file(
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

    // Download first
    {
        let sess = client_sess.lock().map_err(|e| e.to_string())?;
        let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
        let mut remote_file = block_on(|| sftp.open(Path::new(&remote_path))).map_err(|e| e.to_string())?;
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

    // Watcher setup
    let sess_clone = client_sess.clone();
    let local_p = local_path.clone();
    let remote_p = remote_path.clone();
    let app_handle = app.clone();
    
    // Remove existing watcher if any
    if let Ok(mut watchers) = state.watchers.lock() {
        watchers.remove(&local_path_str);
    }

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        match res {
            Ok(event) => {
                // Check for both data modification and attribute changes if needed, but Modify(Data) is key
                if let EventKind::Modify(_) = event.kind {
                     let sess_clone2 = sess_clone.clone();
                    let local_p2 = local_p.clone();
                    let remote_p2 = remote_p.clone();
                    let app_h = app_handle.clone();
                    
                    thread::spawn(move || {
                        // Debounce slightly
                        thread::sleep(Duration::from_millis(500));
                        
                        if let Ok(sess) = sess_clone2.lock() {
                            if let Ok(sftp) = block_on(|| sess.sftp()) {
                                // Read local
                                if let Ok(mut local_file) = std::fs::File::open(&local_p2) {
                                     // Overwrite remote
                                     // Use create() to truncate and overwrite
                                     if let Ok(mut remote_file) = block_on(|| sftp.create(Path::new(&remote_p2))) {
                                         let mut buf = [0u8; 32768];
                                         loop {
                                             match local_file.read(&mut buf) {
                                                 Ok(0) => break,
                                                 Ok(n) => {
                                                     let mut pos = 0;
                                                     while pos < n {
                                                         match remote_file.write(&buf[pos..n]) {
                                                             Ok(w) => pos += w,
                                                             Err(e) if e.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(10)),
                                                             Err(_) => break,
                                                         }
                                                     }
                                                 },
                                                 Err(_) => break,
                                             }
                                         }
                                         // Notify frontend
                                         let _ = app_h.emit("file-synced", remote_p2);
                                     }
                                }
                            }
                        }
                    });
                }
            },
            Err(e) => eprintln!("watch error: {:?}", e),
        }
    }).map_err(|e| e.to_string())?;

    watcher.watch(&local_path, RecursiveMode::NonRecursive).map_err(|e| e.to_string())?;
    state.watchers.lock().map_err(|e| e.to_string())?.insert(local_path_str.clone(), watcher);

    // Launch VS Code
    // Try "code" first
    let code_status = std::process::Command::new("code.cmd").arg(&local_path).spawn();
    
    if code_status.is_err() {
         // Try plain "code" (linux/mac or valid path)
         let code_status2 = std::process::Command::new("code").arg(&local_path).spawn();
         
         if code_status2.is_err() {
             let _ = app.emit("installing-vscode", ());
             // Try to install via winget
             // Non-blocking? Or blocking? User needs to wait.
             // Spawning it.
             let local_p_install = local_path.clone();
             thread::spawn(move || {
                 let install_status = std::process::Command::new("winget")
                    .args(&["install", "-e", "--id", "Microsoft.VisualStudioCode", "--source", "winget", "--accept-source-agreements", "--accept-package-agreements"])
                    .output();
                    
                 if let Ok(output) = install_status {
                     if output.status.success() {
                         let _ = std::process::Command::new("code.cmd").arg(&local_p_install).spawn();
                     }
                 }
             });
         }
    }

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
