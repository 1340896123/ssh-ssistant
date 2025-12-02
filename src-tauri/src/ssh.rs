use crate::models::{Connection as SshConnConfig, FileEntry};
use hex;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sha2::{Digest, Sha256};
use ssh2::Session;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum ShellMsg {
    Data(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Exit,
}

pub struct ManagedSession {
    pub session: Session,
    pub jump_session: Option<Session>,
    pub forward_listener: Option<TcpListener>,
}

impl std::ops::Deref for ManagedSession {
    type Target = Session;
    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

/// 会话级SSH连接池：1个主会话（终端专用）+ N个后台会话（文件操作、命令执行）
#[derive(Clone)]
pub struct SessionSshPool {
    config: SshConnConfig,
    main_session: Arc<Mutex<ManagedSession>>, // 主会话，专用于终端
    background_sessions: Arc<Mutex<Vec<Arc<Mutex<ManagedSession>>>>>, // 后台会话池
    max_background_sessions: usize,           // 最大后台会话数量
    next_bg_index: Arc<Mutex<usize>>,         // 轮询索引
}

impl SessionSshPool {
    pub fn new(config: SshConnConfig, max_background_sessions: usize) -> Result<Self, String> {
        // 创建主会话
        let main_session = establish_connection(&config)?;

        // 创建初始后台会话
        let initial_bg_session = establish_connection(&config)?;

        Ok(Self {
            config,
            main_session: Arc::new(Mutex::new(main_session)),
            background_sessions: Arc::new(Mutex::new(vec![Arc::new(Mutex::new(
                initial_bg_session,
            ))])),
            max_background_sessions,
            next_bg_index: Arc::new(Mutex::new(0)),
        })
    }

    /// 获取主会话（终端专用）
    pub fn get_main_session(&self) -> Arc<Mutex<ManagedSession>> {
        self.main_session.clone()
    }

    /// 获取后台会话（轮询分配）
    pub fn get_background_session(&self) -> Result<Arc<Mutex<ManagedSession>>, String> {
        let mut sessions = self.background_sessions.lock().map_err(|e| e.to_string())?;

        // 如果没有后台会话或需要扩容，创建新会话
        if sessions.is_empty() || sessions.len() < self.max_background_sessions {
            let new_session = establish_connection(&self.config)?;
            sessions.push(Arc::new(Mutex::new(new_session)));
        }

        // 轮询选择会话
        let mut index = self.next_bg_index.lock().map_err(|e| e.to_string())?;
        let session = sessions[*index % sessions.len()].clone();
        *index = (*index + 1) % sessions.len();
        drop(index);

        Ok(session)
    }

    /// 检查并清理断开的连接
    pub fn cleanup_disconnected(&self) {
        // 检查后台会话
        if let Ok(mut sessions) = self.background_sessions.lock() {
            sessions.retain(|session| {
                if let Ok(sess) = session.lock() {
                    // 简单检查：尝试ping
                    let result = sess.channel_session();
                    result.is_ok()
                } else {
                    false
                }
            });

            // 确保至少有一个后台会话
            if sessions.is_empty() {
                if let Ok(new_session) = establish_connection(&self.config) {
                    sessions.push(Arc::new(Mutex::new(new_session)));
                }
            }
        }
    }

    /// 关闭所有SSH连接
    pub fn close_all(&self) {
        // 关闭主会话
        if let Ok(main_sess) = self.main_session.lock() {
            let _ = main_sess.disconnect(None, "", None);
        }

        // 关闭所有后台会话
        if let Ok(mut sessions) = self.background_sessions.lock() {
            for session in sessions.drain(..) {
                if let Ok(sess) = session.lock() {
                    let _ = sess.disconnect(None, "", None);
                }
            }
        }
    }

    /// 重建所有连接
    pub fn rebuild_all(&self) -> Result<(), String> {
        // 重建主会话
        {
            let mut main_sess = self.main_session.lock().map_err(|e| e.to_string())?;
            *main_sess = establish_connection(&self.config)?;
        }

        // 重建后台会话
        {
            let mut sessions = self.background_sessions.lock().map_err(|e| e.to_string())?;
            sessions.clear();
            let initial_bg_session = establish_connection(&self.config)?;
            sessions.push(Arc::new(Mutex::new(initial_bg_session)));
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct SshClient {
    ssh_pool: Arc<SessionSshPool>,                 // SSH连接池
    shell_tx: Option<Sender<ShellMsg>>,            // 终端消息通道
    owner_cache: Arc<Mutex<HashMap<u32, String>>>, // UID缓存
    shutdown_signal: Arc<AtomicBool>,              // 用于通知后台监控任务停止
    os_info: Option<String>,                       // Remote OS information
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

// 异步执行SSH操作，避免阻塞主线程
async fn execute_ssh_operation<F, T>(operation: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || operation())
        .await
        .map_err(|e| {
            // 转换 JoinError 为适当的错误类型
            format!("Task join error: {}", e)
        })?
}

fn establish_connection(config: &SshConnConfig) -> Result<ManagedSession, String> {
    let mut sess = Session::new().map_err(|e| e.to_string())?;
    let mut jump_session_holder = None;
    let mut listener_holder = None;

    if let Some(jump_host) = &config.jump_host {
        if !jump_host.trim().is_empty() {
            // Jump Host Logic
            let jump_port = config.jump_port.unwrap_or(22);
            let jump_addr = format!("{}:{}", jump_host, jump_port);
            let jump_tcp = TcpStream::connect(&jump_addr)
                .map_err(|e| format!("Jump host connection failed: {}", e))?;

            let mut jump_sess = Session::new().map_err(|e| e.to_string())?;
            jump_sess.set_tcp_stream(jump_tcp);
            jump_sess
                .handshake()
                .map_err(|e| format!("Jump handshake failed: {}", e))?;

            jump_sess
                .userauth_password(
                    config.jump_username.as_deref().unwrap_or(""),
                    config.jump_password.as_deref().unwrap_or(""),
                )
                .map_err(|e| format!("Jump auth failed: {}", e))?;

            // Local Port Forwarding Pattern using direct TCP channel
            // Create a local listener on a random port
            let listener = TcpListener::bind("127.0.0.1:0")
                .map_err(|e| format!("Failed to bind local port: {}", e))?;

            let local_port = listener
                .local_addr()
                .map_err(|e| format!("Failed to get local port: {}", e))?
                .port();

            // Robust connect with retry
            let connect_addr = format!("127.0.0.1:{}", local_port);
            let mut tcp_stream = None;
            let start = std::time::Instant::now();
            let timeout = Duration::from_secs(5);

            while start.elapsed() < timeout {
                match TcpStream::connect(&connect_addr) {
                    Ok(s) => {
                        tcp_stream = Some(s);
                        break;
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }

            let tcp_stream = tcp_stream.ok_or_else(|| {
                format!("Failed to connect to local forwarded port {}", local_port)
            })?;

            sess.set_tcp_stream(tcp_stream);

            // Start port forwarding thread
            let jump_sess_clone = jump_sess.clone();
            let target_host = config.host.clone();
            let target_port = config.port;
            let listener_clone = listener
                .try_clone()
                .map_err(|e| format!("Failed to clone listener: {}", e))?;

            thread::spawn(move || {
                for stream in listener_clone.incoming() {
                    match stream {
                        Ok(local_stream) => {
                            let jump_sess = jump_sess_clone.clone();
                            let host = target_host.clone();
                            let port = target_port;
                            thread::spawn(move || {
                                if let Ok(mut channel) =
                                    jump_sess.channel_direct_tcpip(&host, port, None)
                                {
                                    let mut local_stream = local_stream;
                                    let mut buf = [0u8; 8192];
                                    loop {
                                        match local_stream.read(&mut buf) {
                                            Ok(0) => break,
                                            Ok(n) => {
                                                if let Err(_) = channel.write_all(&buf[..n]) {
                                                    break;
                                                }
                                            }
                                            Err(_) => break,
                                        }

                                        match channel.read(&mut buf) {
                                            Ok(0) => break,
                                            Ok(n) => {
                                                if let Err(_) = local_stream.write_all(&buf[..n]) {
                                                    break;
                                                }
                                            }
                                            Err(_) => break,
                                        }
                                    }
                                }
                            });
                        }
                        Err(_) => break,
                    }
                }
            });

            // Keep alive
            jump_session_holder = Some(jump_sess);
            listener_holder = Some(listener);
        } else {
            // Direct connection (empty jump host string)
            let addr_str = format!("{}:{}", config.host, config.port);
            let addr = addr_str
                .to_socket_addrs()
                .map_err(|e| e.to_string())?
                .next()
                .ok_or("Invalid address")?;
            let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
                .map_err(|e| format!("Connection failed: {}", e))?;
            sess.set_tcp_stream(tcp);
        }
    } else {
        // Direct connection (no jump host config)
        let addr_str = format!("{}:{}", config.host, config.port);
        let addr = addr_str
            .to_socket_addrs()
            .map_err(|e| e.to_string())?
            .next()
            .ok_or("Invalid address")?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
            .map_err(|e| format!("Connection failed: {}", e))?;
        sess.set_tcp_stream(tcp);
    };

    sess.handshake().map_err(|e| e.to_string())?;

    sess.userauth_password(&config.username, config.password.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;

    // Enable keepalive to avoid idle disconnects
    sess.set_keepalive(true, 30);

    // Set non-blocking mode for concurrency
    sess.set_blocking(false);

    Ok(ManagedSession {
        session: sess,
        jump_session: jump_session_holder,
        forward_listener: listener_holder,
    })
}

fn detect_os(session: &Session) -> String {
    // Try uname -s first (Linux/macOS)
    // Wrap in a scope to ensure channel is dropped before next attempt
    {
        if let Ok(mut channel) = block_on(|| session.channel_session()) {
            if let Ok(_) = block_on(|| channel.exec("uname -s")) {
                let mut buf = Vec::new();
                if let Ok(_) = channel.read_to_end(&mut buf) {
                    let s = String::from_utf8_lossy(&buf);
                    let os = s.trim();
                    if !os.is_empty() && !os.to_lowercase().contains("command not found") {
                        // Check for Windows-like output from uname (e.g., MINGW, CYGWIN)
                        if os.to_uppercase().contains("MINGW") 
                           || os.to_uppercase().contains("CYGWIN") 
                           || os.to_uppercase().contains("MSYS") {
                            return "Windows".to_string();
                        }
                        return os.to_string();
                    }
                }
            }
            // Explicitly close if we fall through
            let _ = channel.close();
            let _ = channel.wait_close();
        }
    }

    // If uname fails, try to detect Windows
    // We can try to run 'ver' or check for environment variables
    let mut channel = match block_on(|| session.channel_session()) {
        Ok(c) => c,
        Err(_) => return "Unknown".to_string(),
    };

    if let Ok(_) = block_on(|| channel.exec("cmd.exe /c ver")) {
        let mut buf = Vec::new();
        if let Ok(_) = channel.read_to_end(&mut buf) {
            let s = String::from_utf8_lossy(&buf);
            if s.contains("Microsoft Windows") {
                return "Windows".to_string();
            }
        }
    }

    "Unknown".to_string()
}

#[tauri::command]
pub async fn test_connection(config: SshConnConfig) -> Result<String, String> {
    execute_ssh_operation(move || {
        let session = establish_connection(&config)?;
        // Disconnect immediately as we only wanted to test credentials/reachability
        let _ = session.disconnect(None, "Connection Test", None);
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
                },
                ssh_pool: crate::models::SshPoolSettings {
                    max_background_sessions: 3,
                    enable_auto_cleanup: true,
                    cleanup_interval_minutes: 5,
                },
            });
        settings.ssh_pool.max_background_sessions as usize
    };

    // 创建SSH连接池
    let ssh_pool = SessionSshPool::new(config.clone(), max_bg_sessions)?;
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

    // 在主会话上启动shell线程
    let (tx, rx): (Sender<ShellMsg>, Receiver<ShellMsg>) = channel();
    let shell_sess = main_session.clone();
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
            };
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
                }
                Ok(_) => {
                    // EOF
                    break;
                }
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
                    }
                    ShellMsg::Resize { rows, cols } => {
                        let _ = channel.request_pty_size(cols.into(), rows.into(), None, None);
                    }
                    ShellMsg::Exit => return,
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
        let _ = app.emit(&format!("term-exit:{}", shell_id), ());
    });

    // Detect OS
    let os_info = detect_os(&main_session.lock().unwrap());
    println!("Detected OS: {}", os_info);

    let client = SshClient {
        ssh_pool: Arc::new(ssh_pool),
        shell_tx: Some(tx),
        owner_cache: Arc::new(Mutex::new(HashMap::new())),
        shutdown_signal,
        os_info: Some(os_info),
    };

    state
        .clients
        .lock()
        .map_err(|e| e.to_string())?
        .insert(id.clone(), client);

    Ok(id)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>, id: String) -> Result<(), String> {
    if let Some(client) = state.clients.lock().map_err(|e| e.to_string())?.remove(&id) {
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
pub async fn read_remote_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
    max_bytes: Option<u64>,
) -> Result<String, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients
            .get(&id)
            .ok_or("Session not found")
            .map_err(|e| e.to_string())?;
        client.clone()
    };

    // 使用后台会话进行文件操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    let remote_path = std::path::Path::new(&path);

    let mut file = block_on(|| sftp.open(remote_path)).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    use std::io::Read as _;
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    let limit = max_bytes.unwrap_or(16384) as usize;
    if buf.len() > limit {
        buf.truncate(limit);
    }

    String::from_utf8(buf).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_remote_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
    content: String,
    mode: Option<String>,
) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients
            .get(&id)
            .ok_or("Session not found")
            .map_err(|e| e.to_string())?;
        client.clone()
    };

    // 使用后台会话进行文件操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    let remote_path = std::path::Path::new(&path);

    let open_mode = mode.unwrap_or_else(|| "overwrite".to_string());
    let mut file = if open_mode == "append" {
        use ssh2::OpenFlags;
        block_on(|| {
            sftp.open_mode(
                remote_path,
                OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::APPEND,
                0o644,
                ssh2::OpenType::File,
            )
        })
        .map_err(|e| e.to_string())?
    } else {
        use ssh2::OpenFlags;
        block_on(|| {
            sftp.open_mode(
                remote_path,
                OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                0o644,
                ssh2::OpenType::File,
            )
        })
        .map_err(|e| e.to_string())?
    };

    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn search_remote_files(
    state: State<'_, AppState>,
    id: String,
    root: String,
    pattern: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients
            .get(&id)
            .ok_or("Session not found")
            .map_err(|e| e.to_string())?;
        client.clone()
    };

    // 在后台线程中执行搜索操作
    execute_ssh_operation(move || {
        // 使用后台会话进行搜索操作
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().map_err(|e| e.to_string())?;
        let mut channel = block_on(|| sess.channel_session()).map_err(|e| e.to_string())?;

        let limit = max_results.unwrap_or(200);
        // Use grep -R with head to limit number of lines
        let safe_root = root.replace("'", "'\\''");
        let safe_pattern = pattern.replace("'", "'\\''");
        let cmd = format!(
            "cd '{}' && grep -R -n --line-number --text -- '{}' | head -n {}",
            safe_root, safe_pattern, limit
        );

        block_on(|| channel.exec(&cmd)).map_err(|e| e.to_string())?;
        let mut output = String::new();
        let mut buf = [0u8; 1024];
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(15);

        loop {
            if start_time.elapsed() > timeout {
                return Err("Search command timeout".to_string());
            }

            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(output)
    })
    .await
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
pub async fn get_os_info(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let clients = state.clients.lock().map_err(|e| e.to_string())?;
    let client = clients.get(&id).ok_or("Session not found")?;
    Ok(client.os_info.clone().unwrap_or("Unknown".to_string()))
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行文件列表操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let owner_cache = client.owner_cache.clone();

    let sess = bg_session.lock().map_err(|e| e.to_string())?;

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
                                let mut name = if uid == 0 {
                                    "root".to_string()
                                } else {
                                    "-".to_string()
                                };
                                if let Ok(mut channel) = sess.channel_session() {
                                    let cmd = format!("id -nu {}", uid);
                                    if channel.exec(&cmd).is_ok() {
                                        let mut buf = [0u8; 256];
                                        let mut username_data = String::new();
                                        let start_time = std::time::Instant::now();
                                        let timeout = Duration::from_secs(5);

                                        loop {
                                            if start_time.elapsed() > timeout {
                                                break;
                                            }

                                            match channel.read(&mut buf) {
                                                Ok(0) => break,
                                                Ok(n) => username_data
                                                    .push_str(&String::from_utf8_lossy(&buf[..n])),
                                                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                                    thread::sleep(Duration::from_millis(10));
                                                }
                                                Err(_) => break,
                                            }
                                        }
                                        let _ = channel.wait_close();

                                        let trimmed = username_data.trim();
                                        if !trimmed.is_empty() {
                                            name = trimmed.to_string();
                                        }
                                    }
                                }
                                name
                            };
                            cache.insert(uid, username.clone());
                            username
                        }
                    } else {
                        if uid == 0 {
                            "root".to_string()
                        } else {
                            "-".to_string()
                        }
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行目录创建
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;

    match block_on(|| sftp.mkdir(std::path::Path::new(&path), 0o755)) {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("Permission denied") {
                Err(format!("Permission denied: Cannot create directory '{}'. Check if you have write permissions.", path))
            } else if error_msg.contains("No such file") {
                Err(format!("Parent directory does not exist: {}", path))
            } else {
                Err(format!(
                    "Failed to create directory '{}': {}",
                    path, error_msg
                ))
            }
        }
    }
}

#[tauri::command]
pub async fn create_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行文件创建
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;

    match block_on(|| sftp.create(std::path::Path::new(&path))) {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("Permission denied") {
                Err(format!("Permission denied: Cannot create file '{}'. Check if you have write permissions.", path))
            } else if error_msg.contains("No such file") {
                Err(format!("Parent directory does not exist: {}", path))
            } else {
                Err(format!("Failed to create file '{}': {}", path, error_msg))
            }
        }
    }
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行删除操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行重命名操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    block_on(|| {
        sftp.rename(
            std::path::Path::new(&old_path),
            std::path::Path::new(&new_path),
            None,
        )
    })
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行下载操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
    let mut remote_file =
        block_on(|| sftp.open(std::path::Path::new(&remote_path))).map_err(|e| e.to_string())?;
    let mut local_file = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;

    let mut buf = [0u8; 32768];
    loop {
        match remote_file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                local_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
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
            let name = path
                .file_name()
                .ok_or("Invalid file name")?
                .to_string_lossy();
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
        let mut remote_file = block_on(|| sftp.create(std::path::Path::new(remote_path)))
            .map_err(|e| e.to_string())?;

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
                            }
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行上传操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    let temp_dir = std::env::temp_dir();
    let local_path = temp_dir.join(&remote_name);
    let local_path_str = local_path.to_str().ok_or("Invalid path")?.to_string();

    {
        // 使用后台会话进行下载操作
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().map_err(|e| e.to_string())?;
        let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
        let mut remote_file = block_on(|| sftp.open(std::path::Path::new(&remote_path)))
            .map_err(|e| e.to_string())?;
        let mut local_file = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;

        let mut buf = [0u8; 32768];
        loop {
            match remote_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    local_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(local_path_str, None::<String>)
        .map_err(|e| e.to_string())?;

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
    let mut buf = [0u8; 1024];
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(10);

    loop {
        if start_time.elapsed() > timeout {
            return Err("Command timeout".to_string());
        }

        match channel.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => s.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
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
    let mut buf = [0u8; 1024];
    let start_time = std::time::Instant::now();

    loop {
        if start_time.elapsed() > timeout {
            return Err("Command timeout".to_string());
        }

        match channel.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => s.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
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
        if n == 0 {
            break;
        }

        let to_hash = if read + (n as u64) > limit {
            (limit - read) as usize
        } else {
            n
        };

        hasher.update(&buf[..to_hash]);
        read += to_hash as u64;

        if read >= limit {
            break;
        }
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
            let name = path
                .file_name()
                .ok_or("Invalid file name")?
                .to_string_lossy();
            // Always use forward slashes for remote SFTP paths
            let new_remote = format!("{}/{}", remote_path.trim_end_matches('/'), name);

            upload_recursive_progress(
                sess,
                sftp,
                &path,
                &new_remote,
                cancel_flag,
                app,
                transfer_id,
                total_size,
                transferred,
                resume,
            )?;
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
                            let _ = app.emit(
                                "transfer-progress",
                                ProgressPayload {
                                    id: transfer_id.to_string(),
                                    transferred: *transferred,
                                    total: total_size,
                                },
                            );
                            return Ok(());
                        }

                        // Verify Checksum before resuming
                        let local_hash = compute_local_file_hash(local_path, size)?;

                        // Use the already locked session for remote hash
                        let remote_hash_result = get_remote_file_hash(sess, remote_path);

                        if let Ok(Some(remote_hash)) = remote_hash_result {
                            if remote_hash.len() == 64 && local_hash == remote_hash {
                                offset = size;
                            } else if remote_hash.len() == 32 {
                                // Compute MD5 locally
                                use md5::Md5;
                                let mut hasher = Md5::new();
                                let mut file =
                                    std::fs::File::open(local_path).map_err(|e| e.to_string())?;
                                let mut buf = [0u8; 8192];
                                let mut read = 0u64;
                                loop {
                                    let n = file.read(&mut buf).map_err(|e| e.to_string())?;
                                    if n == 0 {
                                        break;
                                    }
                                    let to_hash = if read + (n as u64) > size {
                                        (size - read) as usize
                                    } else {
                                        n
                                    };
                                    hasher.update(&buf[..to_hash]);
                                    read += to_hash as u64;
                                    if read >= size {
                                        break;
                                    }
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
                }
                Err(_) => {} // File doesn't exist, start from 0
            }
        }

        if offset > 0 {
            // Resume: open for write and append? SFTP doesn't have O_APPEND exactly like POSIX,
            // but we can write to offset.
            // ssh2 open_mode: (filename, flags, mode, open_type)
            // flags: Write | Read?
            // We use Write.
            remote_file = block_on(|| {
                sftp.open_mode(
                    std::path::Path::new(remote_path),
                    ssh2::OpenFlags::WRITE,
                    0o644,
                    ssh2::OpenType::File,
                )
            })
            .map_err(|e| e.to_string())?;

            // Seek local
            local_file
                .seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;

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
            remote_file
                .seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
        } else {
            remote_file = block_on(|| sftp.create(std::path::Path::new(remote_path)))
                .map_err(|e| e.to_string())?;
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
                                let _ = app.emit(
                                    "transfer-progress",
                                    ProgressPayload {
                                        id: transfer_id.to_string(),
                                        transferred: *transferred,
                                        total: total_size,
                                    },
                                );
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(10));
                            }
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行上传操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    state
        .transfers
        .lock()
        .map_err(|e| e.to_string())?
        .insert(transfer_id.clone(), cancel_flag.clone());

    // Hold the SSH session lock throughout the entire upload operation
    // to prevent concurrent access that causes channel read errors
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;

    let local_p = std::path::Path::new(&local_path);
    let total_size = if local_p.is_dir() {
        get_dir_size(local_p)
    } else {
        local_p.metadata().map_err(|e| e.to_string())?.len()
    };

    let mut transferred = 0;

    let res = upload_recursive_progress(
        &sess,
        &sftp,
        local_p,
        &remote_path,
        &cancel_flag,
        &app,
        &transfer_id,
        total_size,
        &mut transferred,
        resume,
    );

    state
        .transfers
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&transfer_id);

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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行下载操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    state
        .transfers
        .lock()
        .map_err(|e| e.to_string())?
        .insert(transfer_id.clone(), cancel_flag.clone());

    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;

    // Get remote size
    let remote_path_path = std::path::Path::new(&remote_path);
    let total_size = block_on(|| sftp.stat(remote_path_path))
        .map_err(|e| e.to_string())?
        .size
        .ok_or("Unknown size")?;

    let mut offset = 0;
    let mut local_file;

    if resume {
        if let Ok(meta) = std::fs::metadata(&local_path) {
            let local_size = meta.len();
            if local_size < total_size {
                offset = local_size;
                local_file = std::fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(&local_path)
                    .map_err(|e| e.to_string())?;
            } else {
                // Already done
                let _ = app.emit(
                    "transfer-progress",
                    ProgressPayload {
                        id: transfer_id.clone(),
                        transferred: total_size,
                        total: total_size,
                    },
                );
                state
                    .transfers
                    .lock()
                    .map_err(|e| e.to_string())?
                    .remove(&transfer_id);
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
        remote_file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
    }

    let mut transferred = offset;
    let mut buf = [0u8; 32768];
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            state
                .transfers
                .lock()
                .map_err(|e| e.to_string())?
                .remove(&transfer_id);
            return Err("Cancelled".to_string());
        }

        match remote_file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                local_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                transferred += n as u64;
                let _ = app.emit(
                    "transfer-progress",
                    ProgressPayload {
                        id: transfer_id.clone(),
                        transferred,
                        total: total_size,
                    },
                );
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                state
                    .transfers
                    .lock()
                    .map_err(|e| e.to_string())?
                    .remove(&transfer_id);
                return Err(e.to_string());
            }
        }
    }

    state
        .transfers
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&transfer_id);
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话进行文件编辑操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;

    let temp_dir = std::env::temp_dir();
    let local_path = temp_dir.join(&remote_name);
    let local_path_str = local_path.to_str().ok_or("Invalid path")?.to_string();

    // Download first
    {
        let sess = bg_session.lock().map_err(|e| e.to_string())?;
        let sftp = block_on(|| sess.sftp()).map_err(|e| e.to_string())?;
        let mut remote_file =
            block_on(|| sftp.open(Path::new(&remote_path))).map_err(|e| e.to_string())?;
        let mut local_file = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;

        let mut buf = [0u8; 32768];
        loop {
            match remote_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    local_file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    // Watcher setup
    let sess_clone = bg_session.clone();
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
                                    if let Ok(mut remote_file) =
                                        block_on(|| sftp.create(Path::new(&remote_p2)))
                                    {
                                        let mut buf = [0u8; 32768];
                                        loop {
                                            match local_file.read(&mut buf) {
                                                Ok(0) => break,
                                                Ok(n) => {
                                                    let mut pos = 0;
                                                    while pos < n {
                                                        match remote_file.write(&buf[pos..n]) {
                                                            Ok(w) => pos += w,
                                                            Err(e)
                                                                if e.kind()
                                                                    == ErrorKind::WouldBlock =>
                                                            {
                                                                thread::sleep(
                                                                    Duration::from_millis(10),
                                                                )
                                                            }
                                                            Err(_) => break,
                                                        }
                                                    }
                                                }
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
            }
            Err(e) => eprintln!("watch error: {:?}", e),
        }
    })
    .map_err(|e| e.to_string())?;

    watcher
        .watch(&local_path, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;
    state
        .watchers
        .lock()
        .map_err(|e| e.to_string())?
        .insert(local_path_str.clone(), watcher);

    // Launch VS Code
    // Try "code" first
    let code_status = std::process::Command::new("code.cmd")
        .arg(&local_path)
        .spawn();

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
                    .args(&[
                        "install",
                        "-e",
                        "--id",
                        "Microsoft.VisualStudioCode",
                        "--source",
                        "winget",
                        "--accept-source-agreements",
                        "--accept-package-agreements",
                    ])
                    .output();

                if let Ok(output) = install_status {
                    if output.status.success() {
                        let _ = std::process::Command::new("code.cmd")
                            .arg(&local_p_install)
                            .spawn();
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
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话执行命令，避免阻塞SFTP操作
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
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
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    block_on(|| channel.wait_close()).ok();
    Ok(s)
}

#[tauri::command]
pub async fn get_working_directory(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        let client = clients.get(&id).ok_or("Session not found")?;
        client.clone()
    };

    // 使用后台会话执行pwd命令获取工作目录
    let bg_session = client
        .ssh_pool
        .get_background_session()
        .map_err(|e| format!("Failed to get background session: {}", e))?;
    let sess = bg_session.lock().map_err(|e| e.to_string())?;
    let mut channel = block_on(|| sess.channel_session()).map_err(|e| e.to_string())?;
    block_on(|| channel.exec("pwd")).map_err(|e| e.to_string())?;

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
    block_on(|| channel.wait_close()).ok();

    // 清理换行符和空白字符
    Ok(working_dir.trim().to_string())
}
