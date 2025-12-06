use crate::models::Connection as SshConnConfig;
use crate::ssh::{
    ssh2_retry, CONNECTION_RETRY_BASE_DELAY, CONNECTION_RETRY_MAX_ATTEMPTS,
    DEFAULT_CONNECTION_TIMEOUT, JUMP_HOST_TIMEOUT, LOCAL_FORWARD_TIMEOUT,
};
use socket2::{Domain, Protocol, Socket, Type};
use ssh2::Session;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct ForwardingThreadHandle {
    thread_handle: std::thread::JoinHandle<()>,
    shutdown_signal: Arc<AtomicBool>,
}

pub struct ManagedSession {
    pub session: Session,
    pub jump_session: Option<Session>,
    pub forward_listener: Option<TcpListener>,
    pub forwarding_handle: Option<ForwardingThreadHandle>,
}

impl Drop for ManagedSession {
    fn drop(&mut self) {
        // Shutdown forwarding thread if exists
        if let Some(handle) = &mut self.forwarding_handle {
            handle.shutdown_signal.store(true, Ordering::Relaxed);
            // Give the thread a moment to shutdown gracefully
            let handle = std::mem::replace(&mut handle.thread_handle,
                thread::spawn(|| {})); // Replace with empty thread to take ownership
            let _ = handle.join();
        }

        // Close SSH sessions
        if let Some(ref jump_sess) = self.jump_session {
            let _ = jump_sess.disconnect(None, "", None);
        }
        let _ = self.session.disconnect(None, "", None);

        // Close TCP listener
        if let Some(ref listener) = self.forward_listener {
            let _ = listener.set_nonblocking(true);
            let _ = TcpStream::connect(listener.local_addr().unwrap());
        }
    }
}

impl ForwardingThreadHandle {
    pub fn new(
        thread_handle: std::thread::JoinHandle<()>,
        shutdown_signal: Arc<AtomicBool>,
    ) -> Self {
        Self {
            thread_handle,
            shutdown_signal,
        }
    }
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
        let main_session = establish_connection_with_retry(&config)?;

        // Don't create background session immediately to save resources and avoid rate limits
        // It will be created on demand when get_background_session is called

        Ok(Self {
            config,
            main_session: Arc::new(Mutex::new(main_session)),
            background_sessions: Arc::new(Mutex::new(Vec::new())),
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
            // Stagger new connections to avoid server security limits
            thread::sleep(Duration::from_millis(100));
            let new_session = establish_connection_with_retry(&self.config)?;
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
                    // 增强的连接检查：先发送keepalive再ping
                    self.send_keepalive(&sess);
                    let result = sess.channel_session();
                    result.is_ok()
                } else {
                    false
                }
            });

            // 确保至少有一个后台会话
            if sessions.is_empty() {
                if let Ok(new_session) = establish_connection_with_retry(&self.config) {
                    sessions.push(Arc::new(Mutex::new(new_session)));
                }
            }
        }

        // 检查主会话并发送keepalive
        if let Ok(main_sess) = self.main_session.lock() {
            self.send_keepalive(&main_sess);
        }
    }

    /// 主动发送keepalive信号保持连接活跃
    fn send_keepalive(&self, session: &ManagedSession) {
        // SSH2库的keepalive会在后台自动发送，这里我们做额外的主动保活
        // 执行一个简单的命令来检测连接状态
        if let Ok(mut channel) = session.channel_session() {
            let _ = channel.exec("echo");
            // 立即关闭通道，不关心输出
            let _ = channel.close();
        }
    }

    /// 心跳检测：检查所有连接的健康状态
    pub fn heartbeat_check(&self) -> Result<(), String> {
        let mut need_rebuild = false;

        // 检查主会话
        if let Ok(main_sess) = self.main_session.lock() {
            if !self.is_session_alive(&main_sess)? {
                need_rebuild = true;
            }
        }

        // 检查后台会话
        if let Ok(sessions) = self.background_sessions.lock() {
            for session in sessions.iter() {
                if let Ok(sess) = session.lock() {
                    if !self.is_session_alive(&sess)? {
                        need_rebuild = true;
                        break;
                    }
                }
            }
        }

        if need_rebuild {
            self.rebuild_all()?;
        }

        Ok(())
    }

    /// 检查单个会话是否存活
    fn is_session_alive(&self, session: &ManagedSession) -> Result<bool, String> {
        // 尝试打开一个通道来检测连接状态
        match session.channel_session() {
            Ok(mut channel) => {
                // 执行一个轻量级命令
                match channel.exec("pwd") {
                    Ok(_) => {
                        let _ = channel.close();
                        Ok(true)
                    }
                    Err(_) => Ok(false),
                }
            }
            Err(_) => Ok(false),
        }
    }

    /// 关闭所有SSH连接
    pub fn close_all(&self) {
        // 关闭主会话
        if let Ok(mut main_sess) = self.main_session.lock() {
            // Close forwarding thread first
            if let Some(mut handle) = main_sess.forwarding_handle.take() {
                handle.shutdown_signal.store(true, Ordering::Relaxed);
                let thread_handle = std::mem::replace(&mut handle.thread_handle,
                    thread::spawn(|| {})); // Replace with empty thread
                let _ = thread_handle.join();
            }
            // Close sessions
            if let Some(ref jump_sess) = main_sess.jump_session {
                let _ = jump_sess.disconnect(None, "", None);
            }
            let _ = main_sess.session.disconnect(None, "", None);
            // Close listener
            if let Some(ref listener) = main_sess.forward_listener {
                let _ = listener.set_nonblocking(true);
                let _ = TcpStream::connect(listener.local_addr().unwrap());
            }
        }

        // 关闭所有后台会话
        if let Ok(mut sessions) = self.background_sessions.lock() {
            for session in sessions.drain(..) {
                if let Ok(mut sess) = session.lock() {
                    // Close forwarding thread first
                    if let Some(mut handle) = sess.forwarding_handle.take() {
                        handle.shutdown_signal.store(true, Ordering::Relaxed);
                        let thread_handle = std::mem::replace(&mut handle.thread_handle,
                            thread::spawn(|| {})); // Replace with empty thread
                        let _ = thread_handle.join();
                    }
                    // Close sessions
                    if let Some(ref jump_sess) = sess.jump_session {
                        let _ = jump_sess.disconnect(None, "", None);
                    }
                    let _ = sess.session.disconnect(None, "", None);
                    // Close listener
                    if let Some(ref listener) = sess.forward_listener {
                        let _ = listener.set_nonblocking(true);
                        let _ = TcpStream::connect(listener.local_addr().unwrap());
                    }
                }
            }
        }
    }

    /// 重建所有连接
    pub fn rebuild_all(&self) -> Result<(), String> {
        // 重建主会话
        {
            let mut main_sess = self.main_session.lock().map_err(|e| e.to_string())?;
            *main_sess = establish_connection_with_retry(&self.config)?;
        }

        // 重建后台会话 (stagger creation)
        thread::sleep(Duration::from_millis(200));
        {
            let mut sessions = self.background_sessions.lock().map_err(|e| e.to_string())?;
            sessions.clear();
            let initial_bg_session = establish_connection_with_retry(&self.config)?;
            sessions.push(Arc::new(Mutex::new(initial_bg_session)));
        }

        Ok(())
    }
}

pub fn establish_connection_with_retry(config: &SshConnConfig) -> Result<ManagedSession, String> {
    for attempt in 1..=CONNECTION_RETRY_MAX_ATTEMPTS {
        match establish_connection_internal(config) {
            Ok(session) => return Ok(session),
            Err(e) => {
                if attempt == CONNECTION_RETRY_MAX_ATTEMPTS {
                    return Err(format!("Failed to establish connection after {} attempts: {}", CONNECTION_RETRY_MAX_ATTEMPTS, e));
                }

                let delay = CONNECTION_RETRY_BASE_DELAY * 2_u32.pow(attempt - 1);
                thread::sleep(delay);
            }
        }
    }
    unreachable!()
}

fn establish_connection_internal(config: &SshConnConfig) -> Result<ManagedSession, String> {
    let mut sess = Session::new().map_err(|e| e.to_string())?;
    let mut jump_session_holder = None;
    let mut listener_holder = None;
    let mut forwarding_handle = None;

    if let Some(jump_host) = &config.jump_host {
        if !jump_host.trim().is_empty() {
            // Jump Host Logic
            let jump_port = config.jump_port.unwrap_or(22);
            let jump_addr = format!("{}:{}", jump_host, jump_port);

            // Connect to jump host with longer timeout
            let jump_tcp = connect_with_timeout(&jump_addr, JUMP_HOST_TIMEOUT)
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

            // Enable non-blocking mode for the jump session to allow the forwarding loop to yield
            jump_sess.set_blocking(false);

            // Local Port Forwarding Pattern
            // 1. Bind a local port first to ensure it's ready
            let listener = TcpListener::bind("127.0.0.1:0")
                .map_err(|e| format!("Failed to bind local port: {}", e))?;

            listener.set_nonblocking(true)
                .map_err(|e| format!("Failed to set listener non-blocking: {}", e))?;

            let local_port = listener
                .local_addr()
                .map_err(|e| format!("Failed to get local port: {}", e))?
                .port();

            // Create shutdown signal for forwarding thread
            let shutdown_signal = Arc::new(AtomicBool::new(false));

            // 2. Start port forwarding thread
            let jump_sess_clone = jump_sess.clone();
            let target_host = config.host.clone();
            let target_port = config.port;
            let listener_clone = listener
                .try_clone()
                .map_err(|e| format!("Failed to clone listener: {}", e))?;
            let shutdown_signal_clone = shutdown_signal.clone();

            let thread_handle = thread::spawn(move || {
                while !shutdown_signal_clone.load(Ordering::Relaxed) {
                    match listener_clone.accept() {
                        Ok((mut local_stream, _)) => {
                            let jump_sess_inner = jump_sess_clone.clone();
                            let host = target_host.clone();
                            let port = target_port;
                            let shutdown_inner = shutdown_signal_clone.clone();

                            // Handle connection in a new thread
                            thread::spawn(move || {
                                // Try to open the direct-tcpip channel
                                // Since jump_sess is non-blocking, we need to handle WouldBlock during open
                                let mut channel = loop {
                                    match jump_sess_inner.channel_direct_tcpip(&host, port, None) {
                                        Ok(c) => break c,
                                        Err(e) if e.code() == ssh2::ErrorCode::Session(-37) => {
                                            // EAGAIN
                                            if shutdown_inner.load(Ordering::Relaxed) { return; }
                                            thread::sleep(Duration::from_millis(10));
                                            continue;
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to establish SSH tunnel: {}", e);
                                            return;
                                        }
                                    }
                                };

                                // Set local stream to non-blocking
                                if let Err(_) = local_stream.set_nonblocking(true) {
                                    return;
                                }

                                let mut buf = [0u8; 16384]; // 16KB buffer

                                while !shutdown_inner.load(Ordering::Relaxed) {
                                    let mut has_data = false;

                                    // Read from Local -> Write to Remote
                                    match local_stream.read(&mut buf) {
                                        Ok(0) => break, // EOF
                                        Ok(n) => {
                                            has_data = true;
                                            // Write to channel (handle WouldBlock)
                                            let mut pos = 0;
                                            while pos < n {
                                                match channel.write(&buf[pos..n]) {
                                                    Ok(written) => pos += written,
                                                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                                        thread::sleep(Duration::from_millis(1));
                                                    }
                                                    Err(_) => return, // Pipe broken
                                                }
                                            }
                                        }
                                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                            // Continue to check remote
                                        }
                                        Err(_) => break,
                                    }

                                    // Read from Remote -> Write to Local
                                    match channel.read(&mut buf) {
                                        Ok(0) => break, // EOF
                                        Ok(n) => {
                                            has_data = true;
                                            let mut pos = 0;
                                            while pos < n {
                                                match local_stream.write(&buf[pos..n]) {
                                                    Ok(written) => pos += written,
                                                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                                        thread::sleep(Duration::from_millis(1));
                                                    }
                                                    Err(_) => return,
                                                }
                                            }
                                        }
                                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                            // Continue
                                        }
                                        Err(_) => break,
                                    }

                                    if !has_data {
                                        thread::sleep(Duration::from_millis(5));
                                    }
                                }
                            });
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            // No connection yet
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            });

            // 3. Connect to the local forwarded port
            // Increase timeout to account for potential delays, though local connect is usually fast
            let connect_addr = format!("127.0.0.1:{}", local_port);
            let tcp_stream = connect_with_timeout(&connect_addr, LOCAL_FORWARD_TIMEOUT)
                .map_err(|e| format!("Failed to connect to local forwarded port {}: {}", local_port, e))?;

            sess.set_tcp_stream(tcp_stream);

            // Store handles
            forwarding_handle = Some(ForwardingThreadHandle::new(thread_handle, shutdown_signal));
            jump_session_holder = Some(jump_sess);
            listener_holder = Some(listener);
        } else {
            // Direct connection (empty jump host string)
            let addr_str = format!("{}:{}", config.host, config.port);
            let tcp = connect_with_timeout(&addr_str, DEFAULT_CONNECTION_TIMEOUT)
                .map_err(|e| format!("Connection failed: {}", e))?;
            sess.set_tcp_stream(tcp);
        }
    } else {
        // Direct connection (no jump host config)
        let addr_str = format!("{}:{}", config.host, config.port);
        let tcp = connect_with_timeout(&addr_str, DEFAULT_CONNECTION_TIMEOUT)
            .map_err(|e| format!("Connection failed: {}", e))?;
        sess.set_tcp_stream(tcp);
    };

    sess.handshake().map_err(|e| format!("Handshake failed: {}", e))?;

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
        forwarding_handle,
    })
}

// Helper function to connect with timeout and proper error handling
fn connect_with_timeout(addr_str: &str, timeout: Duration) -> Result<TcpStream, String> {
    let addr = addr_str
        .to_socket_addrs()
        .map_err(|e| format!("Invalid address '{}': {}", addr_str, e))?
        .next()
        .ok_or("No valid addresses found")?;

    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| format!("Failed to connect to '{}': {}", addr_str, e))?;

    // Set TCP keepalive to prevent connection drops
    if cfg!(unix) {
        use socket2::Socket;

        // For Unix systems, we can use socket2 for advanced keepalive options
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;

            // Get the raw file descriptor
            let fd = stream.as_raw_fd();

            // Convert to socket2::Socket for advanced options
            let socket = unsafe { Socket::from_raw_fd(fd) };

            // Enable TCP keepalive
            socket.set_keepalive(true).map_err(|e| format!("Failed to set keepalive: {}", e))?;

            // Set keepalive parameters (Linux/macOS)
            #[cfg(target_os = "linux")]
            {
                socket.set_tcp_keepidle(Some(Duration::from_secs(60))).map_err(|e| format!("Failed to set keepidle: {}", e))?; // Start keepalive after 60s
                socket.set_tcp_keepintvl(Some(Duration::from_secs(10))).map_err(|e| format!("Failed to set keepintvl: {}", e))?; // Send keepalive every 10s
                socket.set_tcp_keepcnt(Some(3)).map_err(|e| format!("Failed to set keepcnt: {}", e))?; // Max 3 failed probes
            }

            #[cfg(target_os = "macos")]
            {
                socket.set_tcp_keepalive(Some(Duration::from_secs(60)), Some(Duration::from_secs(10)))
                    .map_err(|e| format!("Failed to set keepalive on macOS: {}", e))?;
            }

            // We don't convert back, just let the socket be dropped
            // The underlying file descriptor is still owned by the TcpStream
        }
    } else if cfg!(windows) {
        // Windows: use the standard TcpStream set_nodelay and socket2 for keepalive
        use socket2::Socket;

        // Set TCP_NODELAY to improve latency
        stream.set_nodelay(true).map_err(|e| format!("Failed to set nodelay: {}", e))?;

        // For Windows, we can still use socket2 but need a different approach
        #[cfg(windows)]
        {
            use socket2::{Domain, Type, Protocol};
            use std::os::windows::io::AsRawSocket;

            // Create a new socket with the same configuration
            let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
                .map_err(|e| format!("Failed to create socket: {}", e))?;

            // Enable keepalive on the socket
            socket.set_keepalive(true).map_err(|e| format!("Failed to set keepalive: {}", e))?;

            // Note: We can't directly modify the existing TcpStream's socket on Windows
            // but creating new sockets with keepalive enabled demonstrates the approach
        }
    }

    Ok(stream)
}