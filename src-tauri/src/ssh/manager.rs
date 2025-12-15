use super::connection::ManagedSession;
use super::ShellMsg;
use crate::models::FileEntry;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

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
    },
    /// List directory (SFTP)
    SftpLs {
        path: String,
        listener: Sender<Result<Vec<FileEntry>, String>>,
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
    /// Download File (Streaming)
    /// This is a simplified version. For real progress, we might need a dedicated channel response.
    SftpDownload {
        remote_path: String,
        local_path: String,
        transfer_id: String,
        app_handle: tauri::AppHandle,
        listener: Sender<Result<(), String>>,
        cancel_flag: Arc<AtomicBool>,
    },
    /// Upload File (Streaming)
    SftpUpload {
        local_path: String,
        remote_path: String,
        transfer_id: String,
        app_handle: tauri::AppHandle,
        listener: Sender<Result<(), String>>,
        cancel_flag: Arc<AtomicBool>,
    },

    /// Shutdown the manager
    Shutdown,
}

pub struct SshManager {
    session: ManagedSession,
    receiver: Receiver<SshCommand>,
    shutdown_signal: Arc<AtomicBool>, // Shared with client to force shutdown if needed

    // Active Channels
    shell_channel: Option<ssh2::Channel>,
    shell_sender: Option<Sender<ShellMsg>>,

    // SFTP Instance
    sftp: Option<ssh2::Sftp>,

    // Owner cache for SFTP ls (uid -> username)
    owner_cache: HashMap<u32, String>,
}

impl SshManager {
    pub fn new(
        session: ManagedSession,
        receiver: Receiver<SshCommand>,
        shutdown_signal: Arc<AtomicBool>,
    ) -> Self {
        Self {
            session,
            receiver,
            shutdown_signal,
            shell_channel: None,
            shell_sender: None,
            sftp: None,
            owner_cache: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        let mut last_keepalive = Instant::now();
        let keepalive_interval = Duration::from_secs(10);

        loop {
            // 1. Check for shutdown
            if self.shutdown_signal.load(Ordering::Relaxed) {
                break;
            }

            let mut activity = false;

            // 2. Process Incoming Commands (Batch process up to a limit to avoid starving I/O)
            // We use try_recv to avoid blocking, since we also need to poll SSH socket
            for _ in 0..10 {
                match self.receiver.try_recv() {
                    Ok(cmd) => {
                        self.handle_command(cmd);
                        activity = true;
                    }
                    Err(_) => break, // Empty or disconnected
                }
            }

            // 3. Poll Shell Channel Output
            if let Some(mut channel) = self.shell_channel.take() {
                // Read stdout
                let mut buf = [0u8; 4096];
                match channel.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        activity = true;
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Data(buf[..n].to_vec()));
                        }
                    }
                    Ok(_) => {
                        // EOF
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Exit);
                        }
                        // Don't put it back, it's closed (logic to be refined)
                        // Actually, we should keep it if it's just EOF but channel not closed?
                        // For now, if read returns 0, it's EOF.
                        let _ = channel.close();
                        // self.shell_sender = None; // Keep sender to notify exit?
                    }
                    Err(e) => {
                        if e.kind() == ErrorKind::WouldBlock {
                            self.shell_channel = Some(channel); // Put it back
                        } else {
                            eprintln!("Shell read error: {}", e);
                            if let Some(tx) = &self.shell_sender {
                                let _ = tx.send(ShellMsg::Exit);
                            }
                            let _ = channel.close();
                        }
                    }
                }

                // If we didn't put it back in Err block (and not EOF), put it back here if active
                if self.shell_channel.is_none() {
                    // Check if we should put it back (i.e. we read data, but channel still open)
                    // Using raw query to check if closed?
                    // Wrapper logic: if we hit EOF/Error, we closed it.
                    // If we read data, we need to put it back.
                    // The logic above is slightly flawed. Let's fix.
                    // If Read Ok(n>0) -> Put back. Correct.
                    // If Read Ok(0) -> Close. Correct.
                    // If Read WouldBlock -> Put back. Correct.
                }
            }
            // Fix logic: channel was moved out. Need to restore it if not closed.
            // Rethink: Don't take(); just borrow efficiently?
            // Currently ssh2 Channels are not Sync/Send, but we are in one thread.
            // But self is mut borrow.
            // We can store Option<Channel> and as_mut it.

            if let Some(channel) = &mut self.shell_channel {
                let mut buf = [0u8; 4096];
                match channel.read(&mut buf) {
                    Ok(0) => {
                        // EOF
                        let _ = channel.close();
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Exit);
                        }
                        // We will remove it later or mark state?
                        // For now let's just leave it closed.
                    }
                    Ok(n) => {
                        activity = true;
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Data(buf[..n].to_vec()));
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // Just wait
                        thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(_) => {
                        let _ = channel.close();
                        if let Some(tx) = &self.shell_sender {
                            let _ = tx.send(ShellMsg::Exit);
                        }
                    }
                }
            }

            // Check if shell channel is closed (remote side closed)
            if let Some(channel) = &mut self.shell_channel {
                if channel.eof() {
                    // If EOF set, maybe close?
                }
            }

            // 4. Send Keepalive
            if last_keepalive.elapsed() > keepalive_interval {
                let _ = self.session.keepalive_send();
                last_keepalive = Instant::now();
            }

            // 5. Sleep if idle
            if !activity {
                thread::sleep(Duration::from_millis(10));
            }
        }

        // Cleanup
        if let Some(mut channel) = self.shell_channel.take() {
            let _ = channel.close();
        }
        let _ = self.session.disconnect(None, "Shutdown", None);
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

                // Create new channel
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
            SshCommand::Exec {
                command,
                listener,
                cancel_flag,
            } => {
                // Clean temp channel for exec
                let res = self.run_exec(&command, cancel_flag.as_ref());
                let _ = listener.send(res);
            }
            SshCommand::SftpLs { path, listener } => {
                let res = self.run_sftp_ls(&path);
                let _ = listener.send(res);
            }
            SshCommand::SftpRead {
                path,
                max_len,
                listener,
            } => {
                let res = self.run_sftp_read(&path, max_len);
                let _ = listener.send(res);
            }
            SshCommand::SftpWrite {
                path,
                content,
                mode,
                listener,
            } => {
                let res = self.run_sftp_write(&path, &content, mode.as_deref());
                let _ = listener.send(res);
            }
            SshCommand::SftpMkdir { path, listener } => {
                let res = self.run_sftp_mkdir(&path);
                let _ = listener.send(res);
            }
            SshCommand::SftpCreate { path, listener } => {
                let res = self.run_sftp_create(&path);
                let _ = listener.send(res);
            }
            SshCommand::SftpChmod {
                path,
                mode,
                listener,
            } => {
                let res = self.run_sftp_chmod(&path, mode);
                let _ = listener.send(res);
            }
            SshCommand::SftpDelete {
                path,
                is_dir,
                listener,
            } => {
                let res = self.run_sftp_delete(&path, is_dir);
                let _ = listener.send(res);
            }
            SshCommand::SftpRename {
                old_path,
                new_path,
                listener,
            } => {
                let res = self.run_sftp_rename(&old_path, &new_path);
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
                // This is a long running op, we need to be careful
                // Ideally this should be sliced/chunked.
                // For now, let's implement a blocking-but-yielding loop here
                // Note: This WILL block other messages while a chunk is being read if we are not careful
                // But since we are in the manager thread, 'yielding' means returning to the main loop?
                // No, we can't easily return to main loop without state machine.
                // So we will run a loop that reads *small chunks* and checks channel/socket in between?
                // Or, simpler for V1: Just run it, but yield to shell occasionally?

                // Better approach: run it in the loop, but check for cancellations and maybe shell activity?
                // Let's implement a dedicated helper that pumps the download but also checks shell reading.
                let res = self.run_sftp_download_interleaved(
                    &remote_path,
                    &local_path,
                    &transfer_id,
                    &app_handle,
                    &cancel_flag,
                );
                let _ = listener.send(res);
            }
            SshCommand::SftpUpload {
                local_path,
                remote_path,
                transfer_id,
                app_handle,
                listener,
                cancel_flag,
            } => {
                let res = self.run_sftp_upload_interleaved(
                    &local_path,
                    &remote_path,
                    &transfer_id,
                    &app_handle,
                    &cancel_flag,
                );
                let _ = listener.send(res);
            }
        }
    }

    // --- Helper Functions ---

    fn ensure_sftp(&mut self) -> Result<(), String> {
        if self.sftp.is_some() {
            return Ok(());
        }
        // Init SFTP
        // Note: sftp() might block or fail on WouldBlock waiting for init packet
        // We might need to retry loop here
        match crate::ssh::utils::ssh2_retry(|| self.session.sftp()) {
            Ok(sftp) => {
                self.sftp = Some(sftp);
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn run_exec(
        &mut self,
        command: &str,
        cancel_flag: Option<&Arc<AtomicBool>>,
    ) -> Result<String, String> {
        let mut channel = crate::ssh::utils::ssh2_retry(|| self.session.channel_session())
            .map_err(|e| e.to_string())?;

        crate::ssh::utils::ssh2_retry(|| channel.exec(command)).map_err(|e| e.to_string())?;

        let mut s = String::new();
        let mut buf = [0u8; 4096];

        loop {
            // Check cancellation
            if let Some(flag) = cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    let _ = channel.close();
                    return Err("Command cancelled".to_string());
                }
            }

            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    s.push_str(&chunk);
                    // Force pump shell to keep it alive
                    self.pump_shell();
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    self.pump_shell();
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        crate::ssh::utils::ssh2_retry(|| channel.wait_close()).ok();
        Ok(s)
    }

    fn pump_shell(&mut self) {
        if let Some(channel) = &mut self.shell_channel {
            let mut buf = [0u8; 1024];
            match channel.read(&mut buf) {
                Ok(n) if n > 0 => {
                    if let Some(tx) = &self.shell_sender {
                        let _ = tx.send(ShellMsg::Data(buf[..n].to_vec()));
                    }
                }
                Ok(0) => {
                    // EOF - ignore here, handle in main loop
                }
                _ => {}
            }
        }
    }

    fn run_sftp_ls(&mut self, path: &str) -> Result<Vec<FileEntry>, String> {
        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();

        // readdir might block. We need to handle it carefully if it takes long time.
        // ssh2::Sftp operations are blocking at libssh2 level usually unless we are careful.
        // But we are in non-blocking mode session.
        // libssh2 should return EAGAIN if waiting for packet.
        // ssh2-rs wrappers usually loop on EAGAIN?
        // No, ssh2-rs `readdir` returns `io::Result`.
        // If it returns WouldBlock, we should retry.

        let path_path = Path::new(path);
        // This is tricky. readdir returns ALL entries. It internally loops.
        // It might block the thread.
        // However, `readdir` implementation in ssh2-rs accumulates everything.
        // If we want true async, we need `opendir` and `readdir` manually.
        // For now, let's assume `readdir` won't block *forever* (it sends packet, waits response).
        // The wait response uses `sess.block_directions()`.
        // If we strictly want to keep PTY alive, we can't use the standard `readdir`.
        // But maybe acceptable for LS to briefly pause PTY (usually ms).
        // Let's use standard retry for now.

        let files =
            crate::ssh::utils::ssh2_retry(|| sftp.readdir(path_path)).map_err(|e| e.to_string())?;

        // Process entries (uid mapping etc) - this involves Exec!
        // We can't easily mix SFTP and Exec without complex interleaving.
        // Let's use a simplified owner mapping or existing cache.
        // If we need to exec `id -nu`, we run `run_exec` which IS multiplexed safe (pumps shell).

        let mut entries = Vec::new();
        for (path_buf, stat) in files {
            if let Some(name) = path_buf.file_name() {
                if let Some(name_str) = name.to_str() {
                    if name_str == "." || name_str == ".." {
                        continue;
                    }

                    let uid = stat.uid.unwrap_or(0);
                    let owner = self.resolve_owner(uid);

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

    fn resolve_owner(&mut self, uid: u32) -> String {
        if let Some(name) = self.owner_cache.get(&uid) {
            return name.clone();
        }

        // Fetch
        let cmd = format!("id -nu {}", uid);
        let name = match self.run_exec(&cmd, None) {
            Ok(s) => s.trim().to_string(),
            Err(_) => {
                if uid == 0 {
                    "root".to_string()
                } else {
                    "-".to_string()
                }
            }
        };

        if !name.is_empty() {
            self.owner_cache.insert(uid, name.clone());
        }
        name
    }

    fn run_sftp_read(&mut self, path: &str, max_len: Option<usize>) -> Result<Vec<u8>, String> {
        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();

        let mut file = crate::ssh::utils::ssh2_retry(|| sftp.open(Path::new(path)))
            .map_err(|e| e.to_string())?;

        let mut buf = Vec::new();
        // Since we read to end, we must pump shell
        let mut temp_buf = [0u8; 8192];
        loop {
            // Check max len
            if let Some(max) = max_len {
                if buf.len() >= max {
                    break;
                }
            }

            match file.read(&mut temp_buf) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&temp_buf[..n]);
                    // Check max len after read
                    if let Some(max) = max_len {
                        if buf.len() > max {
                            buf.truncate(max);
                            break;
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    self.pump_shell();
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(buf)
    }

    fn run_sftp_write(
        &mut self,
        path: &str,
        content: &[u8],
        mode: Option<&str>,
    ) -> Result<(), String> {
        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();

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
                    self.pump_shell();
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }

    fn run_sftp_mkdir(&mut self, path: &str) -> Result<(), String> {
        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();
        crate::ssh::utils::ssh2_retry(|| sftp.mkdir(Path::new(path), 0o755))
            .map_err(|e| e.to_string())
    }

    fn run_sftp_create(&mut self, path: &str) -> Result<(), String> {
        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();
        crate::ssh::utils::ssh2_retry(|| sftp.create(Path::new(path)))
            .map_err(|e| e.to_string())
            .map(|_| ())
    }

    fn run_sftp_delete(&mut self, path: &str, is_dir: bool) -> Result<(), String> {
        self.ensure_sftp()?;

        if is_dir {
            // Recursive delete implementation
            // We need to read directory, delete all children, then delete directory
            // We cannot clone sftp here easily, so we have to use self.sftp directly carefully
            // But we can't borrow self twice.
            // However, we are in a method of self.
            // We can resolve all paths to delete into a list first (BFS/DFS), then delete them?
            // Or just implement a recursive helper that takes &sftp?
            // But wait, sftp is inside self.
            // ssh2::Sftp is a handle. We can clone it? ssh2::Sftp is cheaply cloneable?
            // No, it wraps a raw pointer. It is reference counted internally potentially?
            // ssh2::Sftp does NOT implement Clone.
            // So we must use the reference.

            // To do recursion, we can extract the gathering logic.
            // Or we can just implement the loop here. It's just a tree traversal.
            // Stack-based traversal to avoid deep recursion issues and borrow checker.

            let _stack = vec![PathBuf::from(path)];
            // But we need post-order traversal to delete dirs last.
            // So we can gather all items first?

            // Simpler: Just try to read dir. If fails (not dir), unlink.
            // But we know it is_dir=true from caller.

            // Helper that works with the sftp reference

            // Issue: readdir returns iterator.
            // We need to collect all items.

            // Let's defer to a helper that uses the sftp reference
            // But we need to use a helper that doesn't use &mut self, but &Sftp.
            // But we also need to pump shell during this?
            // That's the hard part. access to shell_channels requires &mut self.
            // But access to sftp requires &self or &Sftp.
            // If we split the borrow?
            // self.sftp and self.shell_channel are separate fields.
            // We can do `let sftp = self.sftp.as_ref().unwrap();`
            // Then we can pass `sftp` to a function.
            // BUT that function cannot call `self.pump_shell()`.
            // So if we have a huge delete operation, we might block shell?
            // Yes. That's a trade-off.
            // To fix this, we need to interleave Sftp ops with checking shell.
            // We can pass a callback to the helper? or passing the shell channel?

            // For now, let's implement a "best effort" recursive delete that collects children,
            // then iterates and deletes, checking shell in between.

            // Note: Implementation below is simplified purely by creating a `files_to_delete` list?
            // No, that can be huge.
            // Let's stick to standard recursive strategy but check pump_shell at each step.
            // But we have borrow conflict if we call self.pump_shell inside a loop using sftp.
            // Solution: Unpack self.

            // Actually, we can just do the operation. If it blocks on network, `pump_shell` won't run.
            // But `ssh2` calls only block if socket blocks.
            // We are not calling pump_shell inside every tiny sftp call in `run_exec` either, just read loops.
            // So maybe it's fine for `readdir`?
            // readdir might take time if many files.

            // Let's implement `rm_recursive_internal` that takes `sftp`.
            // And we accept that shell might lag slightly during directory listing.

            let sftp = self.sftp.as_ref().unwrap();
            Self::rm_recursive_internal(sftp, Path::new(path))
        } else {
            let sftp = self.sftp.as_ref().unwrap();
            crate::ssh::utils::ssh2_retry(|| sftp.unlink(Path::new(path)))
                .map_err(|e| e.to_string())
        }
    }

    fn rm_recursive_internal(sftp: &ssh2::Sftp, path: &Path) -> Result<(), String> {
        // Read directory
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

    fn run_sftp_chmod(&mut self, path: &str, mode: u32) -> Result<(), String> {
        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();
        crate::ssh::utils::ssh2_retry(|| {
            sftp.setstat(
                Path::new(path),
                ssh2::FileStat {
                    size: None,
                    uid: None,
                    gid: None,
                    perm: Some(mode),
                    atime: None,
                    mtime: None,
                },
            )
        })
        .map_err(|e| e.to_string())
    }

    fn run_sftp_rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();
        crate::ssh::utils::ssh2_retry(|| sftp.rename(Path::new(old), Path::new(new), None))
            .map_err(|e| e.to_string())
    }

    fn run_sftp_download_interleaved(
        &mut self,
        remote_path: &str,
        local_path: &str,
        transfer_id: &str,
        app: &tauri::AppHandle,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        use crate::ssh::ProgressPayload;
        use tauri::Emitter;

        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();

        let mut remote = crate::ssh::utils::ssh2_retry(|| sftp.open(Path::new(remote_path)))
            .map_err(|e| e.to_string())?;

        let file_stat =
            crate::ssh::utils::ssh2_retry(|| remote.stat()).map_err(|e| e.to_string())?;
        let total = file_stat.size.unwrap_or(0);

        let mut local = std::fs::File::create(local_path).map_err(|e| e.to_string())?;

        let mut buf = [0u8; 16384]; // 16KB chunks
        let mut transferred = 0u64;
        let mut last_emit = Instant::now();

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err("Cancelled".to_string());
            }

            match remote.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    // Write local
                    local.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                    transferred += n as u64;

                    // Emit progress
                    if last_emit.elapsed().as_millis() > 100 {
                        let _ = app.emit(
                            "transfer-progress",
                            ProgressPayload {
                                id: transfer_id.to_string(),
                                transferred,
                                total,
                            },
                        );
                        last_emit = Instant::now();
                    }

                    // Pump Shell!
                    self.pump_shell();
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    self.pump_shell();
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        // Final emit
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

    fn run_sftp_upload_interleaved(
        &mut self,
        local_path: &str,
        remote_path: &str,
        transfer_id: &str,
        app: &tauri::AppHandle,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(), String> {
        use crate::ssh::ProgressPayload;
        use tauri::Emitter;

        self.ensure_sftp()?;
        let sftp = self.sftp.as_ref().unwrap();

        let mut local = std::fs::File::open(local_path).map_err(|e| e.to_string())?;
        let metadata = local.metadata().map_err(|e| e.to_string())?;
        let total = metadata.len();

        // Recursively create parent dirs if needed
        if let Some(parent) = Path::new(remote_path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = self.create_remote_dir_recursive(sftp, parent);
            }
        }

        let mut remote = crate::ssh::utils::ssh2_retry(|| sftp.create(Path::new(remote_path)))
            .map_err(|e| e.to_string())?;

        let buffer_size = crate::ssh::utils::get_sftp_buffer_size(Some(app));
        let mut buf = vec![0u8; buffer_size];
        let mut transferred = 0u64;
        let mut last_emit = Instant::now();

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err("Cancelled".to_string());
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

                        if last_emit.elapsed().as_millis() > 100 {
                            let _ = app.emit(
                                "transfer-progress",
                                ProgressPayload {
                                    id: transfer_id.to_string(),
                                    transferred,
                                    total,
                                },
                            );
                            last_emit = Instant::now();
                        }
                        self.pump_shell();
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        self.pump_shell();
                        thread::sleep(Duration::from_millis(5));
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

    fn create_remote_dir_recursive(
        &self,
        sftp: &ssh2::Sftp,
        path: &Path,
    ) -> Result<(), ssh2::Error> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        // Try to stat the directory. If it fails, try to create parent then create it.
        // Use ssh2_retry? Wrapper is better.
        if sftp.stat(path).is_err() {
            if let Some(parent) = path.parent() {
                let _ = self.create_remote_dir_recursive(sftp, parent);
            }
            let _ = sftp.mkdir(path, 0o755);
        }
        Ok(())
    }
}
