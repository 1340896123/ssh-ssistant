use super::connection::{ManagedSession, SessionSshPool};
use super::heartbeat::{HeartbeatAction, HeartbeatManager, HeartbeatResult};
use super::network_monitor::NetworkMonitor;
use super::{emit_command_output, ExecStreamContext, ShellMsg};
use crate::models::{
    DiskUsage, FileEntry, HeartbeatSettings, NetworkAdaptiveSettings, ServerStatus,
};
use crate::ssh::file_ops::FilePageResponse;

use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn is_wait_socket_timeout(err: &std::io::Error) -> bool {
    if err.kind() == ErrorKind::TimedOut {
        return true;
    }
    let msg = err.to_string().to_lowercase();
    msg.contains("timeout")
        || msg.contains("timed out")
        || msg.contains("time out")
        || msg.contains("wait socket")
}

struct SftpInitFailure {
    message: String,
    should_recycle_session: bool,
}

#[derive(Clone, Copy)]
pub enum ExecTarget {
    Ai,
    FileBrowser,
    Status,
}

/// Commands sent to the SSH Manager Actor
pub enum SshCommand {
    /// Open a shell channel
    ShellOpen {
        cols: u16,
        rows: u16,
        sender: Sender<ShellMsg>,
    },
    /// Write data to shell
    ShellWrite(Vec<u8>),
    /// Resize shell
    ShellResize { rows: u16, cols: u16 },
    /// Close shell
    ShellClose,
    /// Execute a single command
    Exec {
        command: String,
        listener: Sender<Result<String, String>>,
        cancel_flag: Option<Arc<AtomicBool>>,
        target: ExecTarget,
        stream: Option<ExecStreamContext>,
    },
    /// List directory (SFTP)
    SftpLs {
        path: String,
        listener: Sender<Result<Vec<FileEntry>, String>>,
    },
    /// List directory page (SFTP)
    SftpLsPage {
        path: String,
        cursor: u64,
        limit: usize,
        listener: Sender<Result<FilePageResponse, String>>,
    },
    /// Read file (SFTP)
    SftpRead {
        path: String,
        max_len: Option<usize>, // Added max_len support
        listener: Sender<Result<Vec<u8>, String>>,
    },
    /// Write file (SFTP)
    SftpWrite {
        path: String,
        content: Vec<u8>,
        mode: Option<String>,
        listener: Sender<Result<(), String>>,
    },
    /// Create directory (SFTP)
    SftpMkdir {
        path: String,
        listener: Sender<Result<(), String>>,
    },
    /// Create file (SFTP) - Empty
    SftpCreate {
        path: String,
        listener: Sender<Result<(), String>>,
    },
    /// Change permissions (SFTP)
    SftpChmod {
        path: String,
        mode: u32,
        listener: Sender<Result<(), String>>,
    },
    /// Delete item (SFTP)
    SftpDelete {
        path: String,
        is_dir: bool,
        listener: Sender<Result<(), String>>,
    },
    /// Rename item (SFTP)
    SftpRename {
        old_path: String,
        new_path: String,
        listener: Sender<Result<(), String>>,
    },
    /// Download File (Streaming) - uses transfer_pool to avoid blocking general operations
    SftpDownload {
        remote_path: String,
        local_path: String,
        transfer_id: String,
        app_handle: tauri::AppHandle,
        listener: Sender<Result<(), String>>,
        cancel_flag: Arc<AtomicBool>,
    },
    /// Upload File (Streaming) - uses transfer_pool to avoid blocking general operations
    SftpUpload {
        local_path: String,
        remote_path: String,
        transfer_id: String,
        app_handle: tauri::AppHandle,
        listener: Sender<Result<(), String>>,
        cancel_flag: Arc<AtomicBool>,
    },
    /// Get server status (uses status session pool)
    GetServerStatus {
        listener: Sender<Result<ServerStatus, String>>,
    },
    /// Get disk usage for a path (uses status session pool)
    GetDiskUsage {
        path: String,
        listener: Sender<Result<DiskUsage, String>>,
    },

    /// Shutdown the manager
    Shutdown,
}

pub struct SshManager {
    session: ManagedSession, // Main session for shell
    pool: SessionSshPool,    // Pool for background tasks
    receiver: Receiver<SshCommand>,
    shutdown_signal: Arc<AtomicBool>, // Shared with client to force shutdown if needed

    // Active Channels
    shell_channel: Option<ssh2::Channel>,
    shell_sender: Option<Sender<ShellMsg>>,

    // Heartbeat Manager
    heartbeat_manager: HeartbeatManager,

    // Network Monitor
    network_monitor: Arc<Mutex<NetworkMonitor>>,
}

type OperationTask = Box<dyn FnOnce(SessionSshPool) + Send + 'static>;

#[derive(Clone)]
struct WorkerPool {
    sender: Sender<OperationTask>,
}

impl WorkerPool {
    fn new(
        name: &str,
        size: usize,
        pool: SessionSshPool,
        shutdown_signal: Arc<AtomicBool>,
    ) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<OperationTask>();
        let shared_receiver = Arc::new(Mutex::new(receiver));

        for idx in 0..size.max(1) {
            let pool = pool.clone();
            let shutdown_signal = shutdown_signal.clone();
            let receiver = shared_receiver.clone();
            let worker_name = format!("ssh-{}-worker-{}", name, idx);

            let _ = thread::Builder::new()
                .name(worker_name)
                .spawn(move || loop {
                    if shutdown_signal.load(Ordering::Relaxed) {
                        break;
                    }

                    let task = {
                        let receiver = match receiver.lock() {
                            Ok(receiver) => receiver,
                            Err(_) => break,
                        };
                        receiver.recv_timeout(Duration::from_millis(100))
                    };

                    match task {
                        Ok(task) => task(pool.clone()),
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                });
        }

        Self { sender }
    }

    fn submit<F>(&self, task: F) -> Result<(), String>
    where
        F: FnOnce(SessionSshPool) + Send + 'static,
    {
        self.sender
            .send(Box::new(task))
            .map_err(|_| "Operation worker pool is unavailable".to_string())
    }
}

#[derive(Clone)]
struct OpsScheduler {
    ai: WorkerPool,
    metadata: WorkerPool,
    mutate: WorkerPool,
    transfer: WorkerPool,
    status: WorkerPool,
}

impl OpsScheduler {
    fn new(pool: SessionSshPool, shutdown_signal: Arc<AtomicBool>) -> Self {
        let metadata_capacity = pool.file_browser_capacity();
        let transfer_capacity = pool.transfer_capacity();
        let mutate_capacity = metadata_capacity.min(2).max(1);

        Self {
            ai: WorkerPool::new(
                "ai",
                pool.ai_capacity(),
                pool.clone(),
                shutdown_signal.clone(),
            ),
            metadata: WorkerPool::new(
                "metadata",
                metadata_capacity,
                pool.clone(),
                shutdown_signal.clone(),
            ),
            mutate: WorkerPool::new(
                "mutate",
                mutate_capacity,
                pool.clone(),
                shutdown_signal.clone(),
            ),
            transfer: WorkerPool::new(
                "transfer",
                transfer_capacity,
                pool.clone(),
                shutdown_signal.clone(),
            ),
            status: WorkerPool::new("status", pool.status_capacity(), pool, shutdown_signal),
        }
    }

    fn dispatch(&self, cmd: SshCommand) {
        match cmd {
            SshCommand::Exec {
                command,
                listener,
                cancel_flag,
                target,
                stream,
            } => {
                let worker = match target {
                    ExecTarget::Ai => &self.ai,
                    ExecTarget::FileBrowser => &self.metadata,
                    ExecTarget::Status => &self.status,
                };
                let reply = listener.clone();
                if let Err(error) = worker.submit(move |pool| {
                    let res = SshManager::bg_exec(
                        pool,
                        &command,
                        cancel_flag.as_ref(),
                        target,
                        stream.as_ref(),
                    );
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpLs { path, listener } => {
                let reply = listener.clone();
                if let Err(error) = self.metadata.submit(move |pool| {
                    let res = SshManager::bg_sftp_ls(pool, &path);
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpLsPage {
                path,
                cursor,
                limit,
                listener,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.metadata.submit(move |pool| {
                    let res = SshManager::bg_sftp_ls_page(pool, &path, cursor, limit);
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpRead {
                path,
                max_len,
                listener,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.metadata.submit(move |pool| {
                    let res = SshManager::bg_sftp_read(pool, &path, max_len);
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpWrite {
                path,
                content,
                mode,
                listener,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.mutate.submit(move |pool| {
                    let res = SshManager::bg_sftp_write(pool, &path, &content, mode.as_deref());
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpMkdir { path, listener } => {
                let reply = listener.clone();
                if let Err(error) = self.mutate.submit(move |pool| {
                    let res = SshManager::bg_sftp_simple(pool, &path, |sftp, p| {
                        sftp.mkdir(p, 0o755).map_err(|e| e.to_string())
                    });
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpCreate { path, listener } => {
                let reply = listener.clone();
                if let Err(error) = self.mutate.submit(move |pool| {
                    let res = SshManager::bg_sftp_simple(pool, &path, |sftp, p| {
                        sftp.create(p).map_err(|e| e.to_string()).map(|_| ())
                    });
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpChmod {
                path,
                mode,
                listener,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.mutate.submit(move |pool| {
                    let res = SshManager::bg_sftp_simple(pool, &path, move |sftp, p| {
                        sftp.setstat(
                            p,
                            ssh2::FileStat {
                                perm: Some(mode),
                                size: None,
                                uid: None,
                                gid: None,
                                atime: None,
                                mtime: None,
                            },
                        )
                        .map_err(|e| e.to_string())
                    });
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpDelete {
                path,
                is_dir,
                listener,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.mutate.submit(move |pool| {
                    let res = SshManager::bg_sftp_delete(pool, &path, is_dir);
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpRename {
                old_path,
                new_path,
                listener,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.mutate.submit(move |pool| {
                    let res = SshManager::bg_sftp_rename(pool, &old_path, &new_path);
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpDownload {
                remote_path,
                local_path,
                transfer_id,
                app_handle,
                listener,
                cancel_flag,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.transfer.submit(move |pool| {
                    let res = SshManager::bg_sftp_download_with_pool(
                        pool,
                        &remote_path,
                        &local_path,
                        &transfer_id,
                        &app_handle,
                        &cancel_flag,
                    );
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::SftpUpload {
                local_path,
                remote_path,
                transfer_id,
                app_handle,
                listener,
                cancel_flag,
            } => {
                let reply = listener.clone();
                if let Err(error) = self.transfer.submit(move |pool| {
                    let res = SshManager::bg_sftp_upload_with_pool(
                        pool,
                        &local_path,
                        &remote_path,
                        &transfer_id,
                        &app_handle,
                        &cancel_flag,
                    );
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::GetServerStatus { listener } => {
                let reply = listener.clone();
                if let Err(error) = self.status.submit(move |pool| {
                    let res = SshManager::bg_get_server_status(pool);
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::GetDiskUsage { path, listener } => {
                let reply = listener.clone();
                if let Err(error) = self.status.submit(move |pool| {
                    let res = SshManager::bg_get_disk_usage(pool, &path);
                    let _ = reply.send(res);
                }) {
                    let _ = listener.send(Err(error));
                }
            }
            SshCommand::Shutdown
            | SshCommand::ShellOpen { .. }
            | SshCommand::ShellWrite(_)
            | SshCommand::ShellResize { .. }
            | SshCommand::ShellClose => {}
        }
    }
}

impl SshManager {
    pub fn new(
        session: ManagedSession,
        pool: SessionSshPool,
        receiver: Receiver<SshCommand>,
        shutdown_signal: Arc<AtomicBool>,
    ) -> Self {
        Self::with_heartbeat_settings(
            session,
            pool,
            receiver,
            shutdown_signal,
            HeartbeatSettings::default(),
        )
    }

    pub fn with_heartbeat_settings(
        session: ManagedSession,
        pool: SessionSshPool,
        receiver: Receiver<SshCommand>,
        shutdown_signal: Arc<AtomicBool>,
        heartbeat_settings: HeartbeatSettings,
    ) -> Self {
        let heartbeat_manager =
            HeartbeatManager::with_shutdown(heartbeat_settings, shutdown_signal.clone());
        let network_monitor = Arc::new(Mutex::new(NetworkMonitor::with_default_settings()));

        Self {
            session,
            pool,
            receiver,
            shutdown_signal,
            shell_channel: None,
            shell_sender: None,
            heartbeat_manager,
            network_monitor,
        }
    }

    /// Update heartbeat settings at runtime
    pub fn update_heartbeat_settings(&mut self, settings: HeartbeatSettings) {
        self.heartbeat_manager.update_settings(settings);
    }

    /// Update network adaptive settings at runtime
    pub fn update_network_adaptive_settings(&mut self, settings: NetworkAdaptiveSettings) {
        if let Ok(mut monitor) = self.network_monitor.lock() {
            monitor.update_settings(settings);
        }
    }

    /// Get current network status
    pub fn get_network_status(&self) -> crate::models::NetworkStatus {
        // Note: Return a cloned status to avoid lifetime issues
        self.network_monitor.lock().unwrap().get_status().clone()
    }

    /// Get recommended adaptive parameters
    pub fn get_adaptive_params(&self) -> crate::models::AdaptiveParams {
        self.network_monitor
            .lock()
            .unwrap()
            .get_recommended_params()
    }

    /// Dedicated loop for non-interactive SSH operations.
    /// This loop is intentionally isolated from terminal I/O to avoid head-of-line blocking.
    pub fn run_ops_loop(
        pool: SessionSshPool,
        receiver: Receiver<SshCommand>,
        shutdown_signal: Arc<AtomicBool>,
    ) {
        let scheduler = OpsScheduler::new(pool, shutdown_signal.clone());

        loop {
            if shutdown_signal.load(Ordering::Relaxed) {
                break;
            }

            let cmd = match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(cmd) => cmd,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            };

            match cmd {
                SshCommand::Shutdown => {
                    shutdown_signal.store(true, Ordering::Relaxed);
                    break;
                }
                other => scheduler.dispatch(other),
            }
        }
    }

    pub fn run(&mut self) {
        loop {
            // 1. Check for shutdown
            if self.shutdown_signal.load(Ordering::Relaxed) {
                break;
            }

            let mut activity = false;

            // 2. Process Incoming Commands (Batch process up to a limit to avoid starving I/O)
            // We use try_recv to avoid blocking, since we also need to poll SSH socket
            for _ in 0..64 {
                match self.receiver.try_recv() {
                    Ok(cmd) => {
                        self.handle_command(cmd);
                        activity = true;
                    }
                    Err(_) => break, // Empty or disconnected
                }
            }

            // 3. Poll Shell Channel Output
            // Correct logic attempt 2:
            // We can't easily `take` and match without putting back in every branch.
            // But `shell_channel` is `Option`.
            // Let's use `if let Some(channel) = &mut self.shell_channel`
            // But `read` requires `&mut Channel`.

            let mut shell_channel_closed = false;
            if let Some(channel) = &mut self.shell_channel {
                let mut buf = [0u8; 4096];
                match channel.read(&mut buf) {
                    Ok(0) => {
                        // EOF
                        let _ = channel.close();
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Exit);
                        }
                        shell_channel_closed = true;
                    }
                    Ok(n) => {
                        activity = true;
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Data(buf[..n].to_vec()));
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        // wait
                        // thread::sleep(Duration::from_millis(5)); // sleep at end of loop
                    }
                    Err(e) => {
                        eprintln!("Shell error: {}", e);
                        let _ = channel.close();
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Exit);
                        }
                        shell_channel_closed = true;
                    }
                }
            }
            if shell_channel_closed {
                self.shell_channel = None;
                self.shell_sender = None;
            }

            // 4. Maintenance checks.
            // Important: when terminal is active, avoid running potentially blocking heartbeat checks
            // in this loop to keep command input responsive.
            if self.shell_channel.is_none() {
                let heartbeat_result = self.heartbeat_manager.perform_heartbeat(&self.session);

                self.tick_network_monitor();

                match heartbeat_result {
                    HeartbeatResult::Success => {
                        // Connection is healthy, also check pool
                        let _ = self.pool.heartbeat_check();
                    }
                    HeartbeatResult::Timeout => {
                        // Log timeout but don't take action yet
                        let status = self.heartbeat_manager.get_status();
                        if status.consecutive_failures > 0 {
                            eprintln!(
                                "[Heartbeat] Timeout detected (failures: {})",
                                status.consecutive_failures
                            );
                        }
                    }
                    HeartbeatResult::Failed(msg) => {
                        eprintln!("[Heartbeat] Check failed: {}", msg);
                    }
                    HeartbeatResult::SessionDead => {
                        eprintln!("[Heartbeat] Session appears dead");
                    }
                }

                let action = self.heartbeat_manager.get_recommended_action();
                match action {
                    HeartbeatAction::None => {
                        // All good
                    }
                    HeartbeatAction::SendKeepalive => {
                        // Send immediate keepalive
                        let _ = crate::ssh::utils::ssh2_retry(|| self.session.keepalive_send());
                    }
                    HeartbeatAction::ReconnectBackground => {
                        eprintln!("[Heartbeat] Attempting background reconnection...");
                        // Try to rebuild pool connections silently
                        if let Err(e) = self.pool.rebuild_all() {
                            eprintln!("[Heartbeat] Background reconnect failed: {}", e);
                        } else {
                            // Reset heartbeat status on successful reconnect
                            self.heartbeat_manager.reset();
                        }
                    }
                    HeartbeatAction::NotifyUser => {
                        // In a real implementation, this would emit an event to the frontend
                        eprintln!(
                            "[Heartbeat] Connection unstable - user notification recommended"
                        );
                        // Still try to reconnect
                        if let Err(e) = self.pool.rebuild_all() {
                            eprintln!("[Heartbeat] Reconnect attempt failed: {}", e);
                        }
                    }
                    HeartbeatAction::ForceReconnect => {
                        eprintln!("[Heartbeat] Force reconnecting...");
                        // Force rebuild all connections
                        let _ = self.pool.rebuild_all();
                        // Reset heartbeat status
                        self.heartbeat_manager.reset();
                    }
                }
            }

            // 5. Sleep if idle
            if !activity {
                let sleep_duration = if self.shell_channel.is_some() {
                    // Active terminal loop should stay highly responsive.
                    Duration::from_millis(5)
                } else {
                    self.heartbeat_manager
                        .get_min_check_interval()
                        .min(Duration::from_millis(100))
                };
                thread::sleep(sleep_duration);
            }
        }

        // Cleanup
        if let Some(mut channel) = self.shell_channel.take() {
            let _ = channel.close();
        }
        let _ = self.session.disconnect(None, "Shutdown", None);
        self.pool.close_all();
    }

    fn handle_command(&mut self, cmd: SshCommand) {
        match cmd {
            SshCommand::Shutdown => {
                self.shutdown_signal.store(true, Ordering::Relaxed);
            }
            SshCommand::ShellOpen { cols, rows, sender } => {
                // If shell exists, close it
                if let Some(mut c) = self.shell_channel.take() {
                    let _ = c.close();
                }

                // Create new channel using the main session
                match crate::ssh::utils::ssh2_retry(|| self.session.channel_session()) {
                    Ok(mut channel) => {
                        // Non-blocking is already set on session
                        // Standard setup
                        if let Err(e) = crate::ssh::utils::ssh2_retry(|| {
                            channel.request_pty(
                                "xterm",
                                None,
                                Some((cols.into(), rows.into(), 0, 0)),
                            )
                        }) {
                            eprintln!("Failed to request PTY: {}", e);
                            return;
                        }
                        if let Err(e) = crate::ssh::utils::ssh2_retry(|| channel.shell()) {
                            eprintln!("Failed to start shell: {}", e);
                            return;
                        }
                        self.shell_channel = Some(channel);
                        self.shell_sender = Some(sender);
                    }
                    Err(e) => eprintln!("Failed to create shell channel: {}", e),
                }
            }
            SshCommand::ShellWrite(data) => {
                if let Some(channel) = &mut self.shell_channel {
                    let _ = channel.write_all(&data);
                }
            }
            SshCommand::ShellResize { rows, cols } => {
                if let Some(channel) = &mut self.shell_channel {
                    let _ = channel.request_pty_size(cols.into(), rows.into(), None, None);
                }
            }
            SshCommand::ShellClose => {
                if let Some(mut channel) = self.shell_channel.take() {
                    let _ = channel.close();
                }
                self.shell_sender = None;
            }
            other => Self::handle_ops_command(self.pool.clone(), other),
        }
    }

    fn handle_ops_command(pool: SessionSshPool, cmd: SshCommand) {
        match cmd {
            SshCommand::Exec {
                command,
                listener,
                cancel_flag,
                target,
                stream,
            } => {
                let pool = pool.clone();
                thread::spawn(move || {
                    let res = Self::bg_exec(
                        pool,
                        &command,
                        cancel_flag.as_ref(),
                        target,
                        stream.as_ref(),
                    );
                    let _ = listener.send(res);
                });
            }
            SshCommand::SftpLs { path, listener } => {
                let res = Self::bg_sftp_ls(pool.clone(), &path);
                let _ = listener.send(res);
            }
            SshCommand::SftpLsPage {
                path,
                cursor,
                limit,
                listener,
            } => {
                let res = Self::bg_sftp_ls_page(pool.clone(), &path, cursor, limit);
                let _ = listener.send(res);
            }
            SshCommand::SftpRead {
                path,
                max_len,
                listener,
            } => {
                let res = Self::bg_sftp_read(pool.clone(), &path, max_len);
                let _ = listener.send(res);
            }
            SshCommand::SftpWrite {
                path,
                content,
                mode,
                listener,
            } => {
                let res = Self::bg_sftp_write(pool.clone(), &path, &content, mode.as_deref());
                let _ = listener.send(res);
            }
            SshCommand::SftpMkdir { path, listener } => {
                let res = Self::bg_sftp_simple(pool.clone(), &path, |sftp, p| {
                    sftp.mkdir(p, 0o755).map_err(|e| e.to_string())
                });
                let _ = listener.send(res);
            }
            SshCommand::SftpCreate { path, listener } => {
                let res = Self::bg_sftp_simple(pool.clone(), &path, |sftp, p| {
                    sftp.create(p).map_err(|e| e.to_string()).map(|_| ())
                });
                let _ = listener.send(res);
            }
            SshCommand::SftpChmod {
                path,
                mode,
                listener,
            } => {
                let res = Self::bg_sftp_simple(pool.clone(), &path, move |sftp, p| {
                    sftp.setstat(
                        p,
                        ssh2::FileStat {
                            perm: Some(mode),
                            size: None,
                            uid: None,
                            gid: None,
                            atime: None,
                            mtime: None,
                        },
                    )
                    .map_err(|e| e.to_string())
                });
                let _ = listener.send(res);
            }
            SshCommand::SftpDelete {
                path,
                is_dir,
                listener,
            } => {
                let res = Self::bg_sftp_delete(pool.clone(), &path, is_dir);
                let _ = listener.send(res);
            }
            SshCommand::SftpRename {
                old_path,
                new_path,
                listener,
            } => {
                let res = Self::bg_sftp_rename(pool.clone(), &old_path, &new_path);
                let _ = listener.send(res);
            }
            SshCommand::SftpDownload {
                remote_path,
                local_path,
                transfer_id,
                app_handle,
                listener,
                cancel_flag,
            } => {
                let pool = pool.clone();
                thread::spawn(move || {
                    let res = Self::bg_sftp_download_with_pool(
                        pool,
                        &remote_path,
                        &local_path,
                        &transfer_id,
                        &app_handle,
                        &cancel_flag,
                    );
                    let _ = listener.send(res);
                });
            }
            SshCommand::SftpUpload {
                local_path,
                remote_path,
                transfer_id,
                app_handle,
                listener,
                cancel_flag,
            } => {
                let pool = pool.clone();
                thread::spawn(move || {
                    let res = Self::bg_sftp_upload_with_pool(
                        pool,
                        &local_path,
                        &remote_path,
                        &transfer_id,
                        &app_handle,
                        &cancel_flag,
                    );
                    let _ = listener.send(res);
                });
            }
            SshCommand::GetServerStatus { listener } => {
                let res = Self::bg_get_server_status(pool.clone());
                let _ = listener.send(res);
            }
            SshCommand::GetDiskUsage { path, listener } => {
                let res = Self::bg_get_disk_usage(pool.clone(), &path);
                let _ = listener.send(res);
            }
            SshCommand::Shutdown => {}
            // Shell commands should not be routed to the ops loop.
            SshCommand::ShellOpen { sender, .. } => {
                let _ = sender.send(ShellMsg::Exit);
            }
            SshCommand::ShellWrite(_) | SshCommand::ShellResize { .. } | SshCommand::ShellClose => {
            }
        }
    }

    fn tick_network_monitor(&mut self) {
        let should_check = {
            if let Ok(monitor) = self.network_monitor.lock() {
                monitor.should_check()
            } else {
                false
            }
        };

        if !should_check {
            return;
        }

        let session_mutex = match self.pool.try_get_transfer_session() {
            Ok(Some(s)) => s,
            Ok(None) => return,
            Err(e) => {
                eprintln!("[NetworkMonitor] Failed to get transfer session: {}", e);
                return;
            }
        };

        let session_guard = match session_mutex.try_lock() {
            Ok(s) => s,
            Err(_) => return,
        };

        if let Ok(mut monitor) = self.network_monitor.lock() {
            if let Err(e) = monitor.measure_latency(&session_guard.session) {
                eprintln!("[NetworkMonitor] Failed to measure latency: {}", e);
            }
        }
    }

    // --- Static Background Helper Functions ---

    fn bg_exec(
        pool: SessionSshPool,
        command: &str,
        cancel_flag: Option<&Arc<AtomicBool>>,
        target: ExecTarget,
        stream: Option<&ExecStreamContext>,
    ) -> Result<String, String> {
        let session_mutex = match target {
            ExecTarget::Ai => pool.get_ai_session()?,
            ExecTarget::FileBrowser => pool.get_file_browser_session()?,
            ExecTarget::Status => pool.get_status_session()?,
        };
        let session = session_mutex.lock().map_err(|e| e.to_string())?;

        let mut channel = crate::ssh::utils::ssh2_retry(|| session.channel_session())
            .map_err(|e| e.to_string())?;

        crate::ssh::utils::ssh2_retry(|| channel.exec(command)).map_err(|e| e.to_string())?;

        let mut s = String::new();
        let mut stdout_buf = [0u8; 4096];
        let mut stderr_buf = [0u8; 4096];
        let mut stdout_closed = false;
        let mut stderr_closed = false;

        loop {
            // Check cancellation
            if let Some(flag) = cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    let _ = channel.close();
                    return Err("Command cancelled".to_string());
                }
            }

            let mut had_activity = false;

            if !stdout_closed {
                match channel.read(&mut stdout_buf) {
                    Ok(0) => stdout_closed = true,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&stdout_buf[..n]).into_owned();
                        s.push_str(&chunk);
                        emit_command_output(stream, chunk, "stdout", false);
                        had_activity = true;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.to_string()),
                }
            }

            if !stderr_closed {
                let stderr_result = {
                    let mut stderr = channel.stderr();
                    stderr.read(&mut stderr_buf)
                };

                match stderr_result {
                    Ok(0) => stderr_closed = true,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&stderr_buf[..n]).into_owned();
                        s.push_str(&chunk);
                        emit_command_output(stream, chunk, "stderr", false);
                        had_activity = true;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.to_string()),
                }
            }

            if stdout_closed && stderr_closed {
                break;
            }

            if !had_activity {
                thread::sleep(Duration::from_millis(5));
            }
        }

        crate::ssh::utils::ssh2_retry(|| channel.wait_close()).ok();
        emit_command_output(stream, String::new(), "stdout", true);
        Ok(s)
    }

    fn classify_sftp_init_error(err: &ssh2::Error, timeout: Duration) -> SftpInitFailure {
        let raw = err.to_string();
        let lower = raw.to_lowercase();
        let retryable = crate::ssh::utils::is_retryable_ssh2_error(err);
        let waiting_for_version = lower.contains("ssh_fxp_version");

        if retryable || waiting_for_version {
            return SftpInitFailure {
                message: format!(
                    "SFTP subsystem did not become ready within {}s. Original error: {}",
                    timeout.as_secs(),
                    raw
                ),
                should_recycle_session: true,
            };
        }

        SftpInitFailure {
            message: raw,
            should_recycle_session: false,
        }
    }

    fn bg_get_sftp(
        session: &ManagedSession,
        timeout: Duration,
    ) -> Result<ssh2::Sftp, SftpInitFailure> {
        crate::ssh::utils::open_sftp_with_timeout(&session.session, timeout)
            .map_err(|e| Self::classify_sftp_init_error(&e, timeout))
    }

    fn with_file_browser_sftp<R, F>(pool: SessionSshPool, mut op: F) -> Result<R, String>
    where
        F: FnMut(&ssh2::Sftp) -> Result<R, String>,
    {
        let timeout = pool.sftp_operation_timeout();
        let mut last_error = None;

        for attempt in 0..2 {
            let session_mutex = pool.get_file_browser_session_with_timeout(timeout)?;
            let mut should_recycle = false;

            let result = {
                let session = session_mutex.lock().map_err(|e| e.to_string())?;
                match Self::bg_get_sftp(&session, timeout) {
                    Ok(sftp) => op(&sftp),
                    Err(err) => {
                        should_recycle = err.should_recycle_session;
                        Err(err.message)
                    }
                }
            };

            if should_recycle {
                let _ = pool.recycle_file_browser_session(&session_mutex);
            }

            match result {
                Ok(value) => return Ok(value),
                Err(err) => {
                    last_error = Some(err);
                    if !should_recycle || attempt == 1 {
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "SFTP operation failed".to_string()))
    }

    fn bg_sftp_ls(pool: SessionSshPool, path: &str) -> Result<Vec<FileEntry>, String> {
        Self::with_file_browser_sftp(pool, |sftp| {
            let path_path = Path::new(path);
            let files = crate::ssh::utils::ssh2_retry(|| sftp.readdir(path_path))
                .map_err(|e| e.to_string())?;

            let mut entries = Vec::new();
            for (path_buf, stat) in files {
                if let Some(name) = path_buf.file_name() {
                    if let Some(name_str) = name.to_str() {
                        if name_str == "." || name_str == ".." {
                            continue;
                        }
                        // Simplified owner resolution (no cache/exec for now to avoid complexity)
                        let owner = if stat.uid.unwrap_or(0) == 0 {
                            "root"
                        } else {
                            "-"
                        }
                        .to_string();

                        entries.push(FileEntry {
                            name: name_str.to_string(),
                            is_dir: stat.is_dir(),
                            size: stat.size.unwrap_or(0),
                            mtime: stat.mtime.unwrap_or(0) as i64,
                            permissions: stat.perm.unwrap_or(0),
                            uid: stat.uid.unwrap_or(0),
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
        })
    }

    fn bg_sftp_ls_page(
        pool: SessionSshPool,
        path: &str,
        cursor: u64,
        limit: usize,
    ) -> Result<FilePageResponse, String> {
        Self::with_file_browser_sftp(pool, |sftp| {
            let mut dir = crate::ssh::utils::ssh2_retry(|| sftp.opendir(Path::new(path)))
                .map_err(|e| e.to_string())?;

            let mut skipped = 0u64;
            let mut entries = Vec::new();
            let mut has_more = false;

            loop {
                match dir.readdir() {
                    Ok((path_buf, stat)) => {
                        let Some(name) = path_buf.file_name().and_then(|name| name.to_str()) else {
                            continue;
                        };
                        if name == "." || name == ".." {
                            continue;
                        }

                        if skipped < cursor {
                            skipped += 1;
                            continue;
                        }

                        if entries.len() >= limit {
                            has_more = true;
                            break;
                        }

                        let owner = if stat.uid.unwrap_or(0) == 0 {
                            "root"
                        } else {
                            "-"
                        }
                        .to_string();

                        entries.push(FileEntry {
                            name: name.to_string(),
                            is_dir: stat.is_dir(),
                            size: stat.size.unwrap_or(0),
                            mtime: stat.mtime.unwrap_or(0) as i64,
                            permissions: stat.perm.unwrap_or(0),
                            uid: stat.uid.unwrap_or(0),
                            owner,
                        });
                    }
                    Err(ref e) if e.code() == ssh2::ErrorCode::Session(-16) => {
                        break;
                    }
                    Err(ref e) if e.code() == ssh2::ErrorCode::Session(-37) => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }

            entries.sort_by(|a, b| {
                if a.is_dir == b.is_dir {
                    a.name.cmp(&b.name)
                } else {
                    b.is_dir.cmp(&a.is_dir)
                }
            });

            let next_cursor = if has_more {
                Some(cursor + entries.len() as u64)
            } else {
                None
            };

            Ok(FilePageResponse {
                entries,
                next_cursor,
                has_more,
            })
        })
    }

    fn bg_sftp_read(
        pool: SessionSshPool,
        path: &str,
        max_len: Option<usize>,
    ) -> Result<Vec<u8>, String> {
        Self::with_file_browser_sftp(pool, |sftp| {
            let mut file = crate::ssh::utils::ssh2_retry(|| sftp.open(Path::new(path)))
                .map_err(|e| e.to_string())?;

            let mut buf = Vec::new();
            let mut temp_buf = [0u8; 8192];
            loop {
                if let Some(max) = max_len {
                    if buf.len() >= max {
                        break;
                    }
                }

                match file.read(&mut temp_buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&temp_buf[..n]);
                        if let Some(max) = max_len {
                            if buf.len() > max {
                                buf.truncate(max);
                                break;
                            }
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(buf)
        })
    }

    fn bg_sftp_write(
        pool: SessionSshPool,
        path: &str,
        content: &[u8],
        mode: Option<&str>,
    ) -> Result<(), String> {
        Self::with_file_browser_sftp(pool, |sftp| {
            use ssh2::OpenFlags;
            let mut file = if mode == Some("append") {
                crate::ssh::utils::ssh2_retry(|| {
                    sftp.open_mode(
                        Path::new(path),
                        OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::APPEND,
                        0o644,
                        ssh2::OpenType::File,
                    )
                })
            } else {
                crate::ssh::utils::ssh2_retry(|| {
                    sftp.open_mode(
                        Path::new(path),
                        OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                        0o644,
                        ssh2::OpenType::File,
                    )
                })
            }
            .map_err(|e| e.to_string())?;

            let mut pos = 0;
            while pos < content.len() {
                match file.write(&content[pos..]) {
                    Ok(n) => pos += n,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(())
        })
    }

    fn bg_sftp_simple<F>(pool: SessionSshPool, path: &str, op: F) -> Result<(), String>
    where
        F: FnOnce(&ssh2::Sftp, &Path) -> Result<(), String>,
    {
        let mut op = Some(op);
        Self::with_file_browser_sftp(pool, |sftp| {
            op.take().expect("file browser SFTP op should run once")(sftp, Path::new(path))
        })
    }

    fn bg_sftp_delete(pool: SessionSshPool, path: &str, is_dir: bool) -> Result<(), String> {
        Self::with_file_browser_sftp(pool, |sftp| {
            if is_dir {
                Self::rm_recursive_internal(sftp, Path::new(path))
            } else {
                crate::ssh::utils::ssh2_retry(|| sftp.unlink(Path::new(path)))
                    .map_err(|e| e.to_string())
            }
        })
    }

    fn rm_recursive_internal(sftp: &ssh2::Sftp, path: &Path) -> Result<(), String> {
        let files =
            crate::ssh::utils::ssh2_retry(|| sftp.readdir(path)).map_err(|e| e.to_string())?;

        for (child_path, stat) in files {
            if let Some(name) = child_path.file_name() {
                let name = name.to_string_lossy();
                if name == "." || name == ".." {
                    continue;
                }

                if stat.is_dir() {
                    Self::rm_recursive_internal(sftp, &child_path)?;
                } else {
                    crate::ssh::utils::ssh2_retry(|| sftp.unlink(&child_path))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        crate::ssh::utils::ssh2_retry(|| sftp.rmdir(path)).map_err(|e| e.to_string())
    }

    fn bg_sftp_rename(pool: SessionSshPool, old: &str, new: &str) -> Result<(), String> {
        Self::with_file_browser_sftp(pool, |sftp| {
            crate::ssh::utils::ssh2_retry(|| sftp.rename(Path::new(old), Path::new(new), None))
                .map_err(|e| e.to_string())
        })
    }

    // --- Transfer Functions using dedicated Transfer Pool ---
    // These functions use get_transfer_session() instead of get_file_browser_session()
    // to avoid blocking regular SFTP operations (ls, read, etc.) during file transfers

    fn bg_sftp_download_with_pool(
        pool: SessionSshPool,
        remote_path: &str,
        local_path: &str,
        transfer_id: &str,
        app: &tauri::AppHandle,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        use crate::ssh::ProgressPayload;
        use tauri::Emitter;

        eprintln!(
            "[DEBUG] bg_sftp_download_with_pool ENTER: transfer_id={}, remote={}",
            transfer_id, remote_path
        );

        // Timeout configuration (default 5 minutes)
        let sftp_timeout = Duration::from_secs(300); // 5 minutes default
        let no_progress_timeout = Duration::from_secs(30); // 30 seconds without progress

        // 关键修复：使用传输专用会话池，而不是后台会话池
        // 这样大文件传输不会阻塞目录浏览等普通操作
        let session_mutex = pool.get_transfer_session()?;
        eprintln!(
            "[DEBUG] bg_sftp_download_with_pool: Got transfer session for transfer_id={}",
            transfer_id
        );

        let session_guard = session_mutex.lock().map_err(|e| e.to_string())?;

        struct BlockingRestoreGuard<'a> {
            sess: &'a ssh2::Session,
            was_blocking: bool,
        }

        impl<'a> Drop for BlockingRestoreGuard<'a> {
            fn drop(&mut self) {
                if !self.was_blocking {
                    self.sess.set_blocking(false);
                }
            }
        }

        let was_blocking = session_guard.session.is_blocking();
        if !was_blocking {
            session_guard.session.set_blocking(true);
        }
        let _restore_guard = BlockingRestoreGuard {
            sess: &session_guard.session,
            was_blocking,
        };
        let sftp = Self::bg_get_sftp(&session_guard, pool.sftp_operation_timeout())
            .map_err(|e| e.message)?;

        let mut remote = crate::ssh::utils::ssh2_retry(|| sftp.open(Path::new(remote_path)))
            .map_err(|e| e.to_string())?;

        let file_stat =
            crate::ssh::utils::ssh2_retry(|| remote.stat()).map_err(|e| e.to_string())?;
        let total = file_stat.size.unwrap_or(0);

        let mut local = std::fs::File::create(local_path).map_err(|e| e.to_string())?;

        let mut buf = [0u8; 16384];
        let mut transferred = 0u64;
        let mut last_emit = Instant::now();
        let mut last_emit_transferred = 0u64;

        // Timeout tracking
        let transfer_start = Instant::now();
        let mut last_progress_time = Instant::now();
        let mut would_block_count = 0u32;

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err("Cancelled".to_string());
            }

            // Check overall timeout
            if transfer_start.elapsed() > sftp_timeout {
                return Err(format!(
                    "Download timeout after {}s",
                    sftp_timeout.as_secs()
                ));
            }

            // Check no-progress timeout
            if last_progress_time.elapsed() > no_progress_timeout {
                return Err(format!(
                    "No progress for {}s, connection may be dead",
                    no_progress_timeout.as_secs()
                ));
            }

            match remote.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    local.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                    transferred += n as u64;
                    last_progress_time = Instant::now(); // Update progress time
                    would_block_count = 0; // Reset WouldBlock counter on success

                    if last_emit.elapsed().as_millis() > 250
                        || transferred.saturating_sub(last_emit_transferred) >= 256 * 1024
                    {
                        let _ = app.emit(
                            "transfer-progress",
                            ProgressPayload {
                                id: transfer_id.to_string(),
                                transferred,
                                total,
                            },
                        );
                        last_emit = Instant::now();
                        last_emit_transferred = transferred;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    would_block_count += 1;
                    if would_block_count > 100 {
                        return Err(format!(
                            "Too many WouldBlock errors ({}), connection may be dead",
                            would_block_count
                        ));
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) if is_wait_socket_timeout(&e) => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        let _ = app.emit(
            "transfer-progress",
            ProgressPayload {
                id: transfer_id.to_string(),
                transferred: total,
                total,
            },
        );

        Ok(())
    }

    fn bg_sftp_upload_with_pool(
        pool: SessionSshPool,
        local_path: &str,
        remote_path: &str,
        transfer_id: &str,
        app: &tauri::AppHandle,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        use crate::ssh::ProgressPayload;
        use tauri::Emitter;

        eprintln!(
            "[DEBUG] bg_sftp_upload_with_pool ENTER: transfer_id={}, remote={}",
            transfer_id, remote_path
        );

        // Timeout configuration (default 5 minutes)
        let sftp_timeout = Duration::from_secs(300); // 5 minutes default
        let no_progress_timeout = Duration::from_secs(30); // 30 seconds without progress

        // 关键修复：使用传输专用会话池，而不是后台会话池
        // 这样大文件传输不会阻塞目录浏览等普通操作
        let session_mutex = pool.get_transfer_session()?;
        eprintln!(
            "[DEBUG] bg_sftp_upload_with_pool: Got transfer session for transfer_id={}",
            transfer_id
        );

        let session_guard = session_mutex.lock().map_err(|e| e.to_string())?;

        struct BlockingRestoreGuard<'a> {
            sess: &'a ssh2::Session,
            was_blocking: bool,
        }

        impl<'a> Drop for BlockingRestoreGuard<'a> {
            fn drop(&mut self) {
                if !self.was_blocking {
                    self.sess.set_blocking(false);
                }
            }
        }

        let was_blocking = session_guard.session.is_blocking();
        if !was_blocking {
            session_guard.session.set_blocking(true);
        }
        let _restore_guard = BlockingRestoreGuard {
            sess: &session_guard.session,
            was_blocking,
        };
        let sftp = Self::bg_get_sftp(&session_guard, pool.sftp_operation_timeout())
            .map_err(|e| e.message)?;

        let mut local = std::fs::File::open(local_path).map_err(|e| e.to_string())?;
        let metadata = local.metadata().map_err(|e| e.to_string())?;
        let total = metadata.len();

        // Recursively create parent dirs if needed
        if let Some(parent) = Path::new(remote_path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = Self::create_remote_dir_recursive(&sftp, parent);
            }
        }

        let mut remote = crate::ssh::utils::ssh2_retry(|| sftp.create(Path::new(remote_path)))
            .map_err(|e| e.to_string())?;

        let buffer_size = crate::ssh::utils::get_sftp_buffer_size(Some(app));
        let mut buf = vec![0u8; buffer_size];
        let mut transferred = 0u64;
        let mut last_emit = Instant::now();
        let mut last_emit_transferred = 0u64;

        // Timeout tracking
        let transfer_start = Instant::now();
        let mut last_progress_time = Instant::now();
        let mut would_block_count = 0u32;

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err("Cancelled".to_string());
            }

            // Check overall timeout
            if transfer_start.elapsed() > sftp_timeout {
                return Err(format!("Upload timeout after {}s", sftp_timeout.as_secs()));
            }

            // Check no-progress timeout
            if last_progress_time.elapsed() > no_progress_timeout {
                return Err(format!(
                    "No progress for {}s, connection may be dead",
                    no_progress_timeout.as_secs()
                ));
            }

            let n = local.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }

            let mut pos = 0;
            while pos < n {
                match remote.write(&buf[pos..n]) {
                    Ok(written) => {
                        pos += written;
                        transferred += written as u64;
                        last_progress_time = Instant::now(); // Update progress time
                        would_block_count = 0; // Reset WouldBlock counter on success

                        if last_emit.elapsed().as_millis() > 250
                            || transferred.saturating_sub(last_emit_transferred) >= 256 * 1024
                        {
                            let _ = app.emit(
                                "transfer-progress",
                                ProgressPayload {
                                    id: transfer_id.to_string(),
                                    transferred,
                                    total,
                                },
                            );
                            last_emit = Instant::now();
                            last_emit_transferred = transferred;
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        would_block_count += 1;
                        if would_block_count > 100 {
                            return Err(format!(
                                "Too many WouldBlock errors ({}), connection may be dead",
                                would_block_count
                            ));
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) if is_wait_socket_timeout(&e) => {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
        }

        let _ = app.emit(
            "transfer-progress",
            ProgressPayload {
                id: transfer_id.to_string(),
                transferred: total,
                total,
            },
        );
        Ok(())
    }

    fn bg_sftp_download(
        pool: SessionSshPool,
        remote_path: &str,
        local_path: &str,
        transfer_id: &str,
        app: &tauri::AppHandle,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        // Delegate to the new transfer pool implementation
        Self::bg_sftp_download_with_pool(
            pool,
            remote_path,
            local_path,
            transfer_id,
            app,
            cancel_flag,
        )
    }

    fn bg_sftp_upload(
        pool: SessionSshPool,
        local_path: &str,
        remote_path: &str,
        transfer_id: &str,
        app: &tauri::AppHandle,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        // Delegate to the new transfer pool implementation
        Self::bg_sftp_upload_with_pool(pool, local_path, remote_path, transfer_id, app, cancel_flag)
    }

    fn create_remote_dir_recursive(sftp: &ssh2::Sftp, path: &Path) -> Result<(), ssh2::Error> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        // Try to stat the directory. If it fails, try to create parent then create it.
        if sftp.stat(path).is_err() {
            if let Some(parent) = path.parent() {
                let _ = Self::create_remote_dir_recursive(sftp, parent);
            }
            let _ = sftp.mkdir(path, 0o755);
        }
        Ok(())
    }

    // --- Status Bar Query Functions ---
    // These functions use the dedicated status session pool for isolation

    /// Get server status using the status session pool
    fn bg_get_server_status(pool: SessionSshPool) -> Result<ServerStatus, String> {
        let session_mutex = pool.get_status_session()?;
        let session = session_mutex.lock().map_err(|e| e.to_string())?;

        // Helper function to run a command and parse output
        let run_command = |cmd: &str| -> Result<String, String> {
            let mut channel = crate::ssh::utils::ssh2_retry(|| session.channel_session())
                .map_err(|e| e.to_string())?;
            crate::ssh::utils::ssh2_retry(|| channel.exec(cmd)).map_err(|e| e.to_string())?;

            let mut output = String::new();
            let mut buf = [0u8; 4096];
            loop {
                match channel.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        output.push_str(&String::from_utf8_lossy(&buf[..n]));
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            let _ = channel.wait_close();
            Ok(output.trim().to_string())
        };

        // Get CPU usage
        let cpu_usage = run_command(
            "top -bn1 | grep \"Cpu(s)\" | awk '{print $2}' | sed 's/%us,//' | sed 's/%id,.*//'",
        )
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok());

        // Get memory info from /proc/meminfo
        let memory_output = run_command(
            "awk '/MemTotal:/ {total=$2} /MemAvailable:/ {avail=$2} END {print total \" \" avail}' /proc/meminfo 2>/dev/null"
        ).ok();
        let (memory_used, memory_total) = if let Some(mem) = memory_output {
            let parts: Vec<&str> = mem.split_whitespace().collect();
            if parts.len() >= 2 {
                let total: u64 = parts[0].parse().unwrap_or(0);
                let avail: u64 = parts[1].parse().unwrap_or(0);
                let used = total.saturating_sub(avail);
                (Some(used), Some(total))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Get uptime in seconds
        let uptime = run_command("cat /proc/uptime | awk '{print int($1)}'")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok());

        // Get load average
        let load_average = run_command("cat /proc/loadavg | awk '{print $1 \" \" $2 \" \" $3}'")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(ServerStatus {
            cpu_usage,
            memory_used,
            memory_total,
            uptime,
            load_average,
        })
    }

    /// Get disk usage for a specific path using the status session pool
    fn bg_get_disk_usage(pool: SessionSshPool, path: &str) -> Result<DiskUsage, String> {
        let session_mutex = pool.get_status_session()?;
        let session = session_mutex.lock().map_err(|e| e.to_string())?;

        let cmd = format!("df -B1 \"{}\" 2>/dev/null | tail -1", path);
        let mut channel = crate::ssh::utils::ssh2_retry(|| session.channel_session())
            .map_err(|e| e.to_string())?;
        crate::ssh::utils::ssh2_retry(|| channel.exec(&cmd)).map_err(|e| e.to_string())?;

        let mut output = String::new();
        let mut buf = [0u8; 4096];
        loop {
            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    output.push_str(&String::from_utf8_lossy(&buf[..n]));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        let _ = channel.wait_close();

        // Parse df output: filesystem total used avail percent mount
        let parts: Vec<&str> = output.split_whitespace().collect();
        if parts.len() >= 6 {
            let total: u64 = parts[1]
                .parse()
                .map_err(|_| "Failed to parse total".to_string())?;
            let used: u64 = parts[2]
                .parse()
                .map_err(|_| "Failed to parse used".to_string())?;
            let available: u64 = parts[3]
                .parse()
                .map_err(|_| "Failed to parse available".to_string())?;
            let percent_str = parts[4].trim_end_matches('%');
            let usage_percent: f32 = percent_str
                .parse()
                .map_err(|_| "Failed to parse percent".to_string())?;

            Ok(DiskUsage {
                path: path.to_string(),
                total,
                used,
                available,
                usage_percent,
            })
        } else {
            Err(format!("Invalid df output for path: {}", path))
        }
    }
}
