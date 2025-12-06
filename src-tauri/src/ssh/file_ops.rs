use crate::models::FileEntry;
use crate::ssh::{ssh2_retry, execute_ssh_operation};
use super::utils::{get_sftp_buffer_size, compute_local_file_hash, get_dir_size, get_remote_file_hash};
use super::client::AppState;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    id: String,
    transferred: u64,
    total: u64,
}

#[tauri::command]
pub async fn read_remote_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
    max_bytes: Option<u64>,
) -> Result<String, String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        let mut file = ssh2_retry(|| sftp.open(Path::new(&path)))
            .map_err(|e| e.to_string())?;
        let mut buf = Vec::new();

        // Read in chunks to handle WouldBlock
        let mut temp_buf = vec![0u8; 32 * 1024]; // 32KB chunks
        loop {
            match file.read(&mut temp_buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    buf.extend_from_slice(&temp_buf[..n]);
                    // Check limit if set
                    if let Some(max) = max_bytes {
                        if buf.len() as u64 > max {
                            buf.truncate(max as usize);
                            break;
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        String::from_utf8(buf).map_err(|e| e.to_string())
    }).await
}

#[tauri::command]
pub async fn write_remote_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
    content: String,
    mode: Option<String>,
) -> Result<(), String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        let open_mode = mode.unwrap_or_else(|| "overwrite".to_string());
        let mut file = if open_mode == "append" {
            use ssh2::OpenFlags;
            ssh2_retry(|| {
                sftp.open_mode(
                    Path::new(&path),
                    OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::APPEND,
                    0o644,
                    ssh2::OpenType::File,
                )
            }).map_err(|e| e.to_string())?
        } else {
            use ssh2::OpenFlags;
            ssh2_retry(|| {
                sftp.open_mode(
                    Path::new(&path),
                    OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                    0o644,
                    ssh2::OpenType::File,
                )
            }).map_err(|e| e.to_string())?
        };

        let bytes = content.as_bytes();
        let mut pos = 0;

        while pos < bytes.len() {
            match file.write(&bytes[pos..]) {
                Ok(0) => return Err("Write returned 0 bytes".to_string()),
                Ok(n) => pos += n,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        Ok(())
    }).await
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let owner_cache = client.owner_cache.clone();

        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;
        let path_path = Path::new(&path);
        let files = ssh2_retry(|| sftp.readdir(path_path)).map_err(|e| e.to_string())?;

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
    }).await
}

#[tauri::command]
pub async fn create_directory(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        match ssh2_retry(|| sftp.mkdir(Path::new(&path), 0o755))
            .map_err(|e| e.to_string()) {
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
    }).await
}

#[tauri::command]
pub async fn create_file(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        match ssh2_retry(|| sftp.create(Path::new(&path)))
            .map_err(|e| e.to_string()) {
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
    }).await
}

fn rm_recursive(sftp: &ssh2::Sftp, path: &Path) -> Result<(), String> {
    let files = ssh2_retry(|| sftp.readdir(path)).map_err(|e| e.to_string())?;

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
                    ssh2_retry(|| sftp.unlink(&full_path))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    ssh2_retry(|| sftp.rmdir(path))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_item(
    state: State<'_, AppState>,
    id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        if is_dir {
            rm_recursive(&sftp, Path::new(&path))?;
        } else {
            ssh2_retry(|| sftp.unlink(Path::new(&path)))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }).await
}

#[tauri::command]
pub async fn rename_item(
    state: State<'_, AppState>,
    id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        ssh2_retry(|| {
            sftp.rename(
                Path::new(&old_path),
                Path::new(&new_path),
                None,
            )
        }).map_err(|e| e.to_string())?;

        Ok(())
    }).await
}

#[tauri::command]
pub async fn change_file_permission(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
    path: String,
    perms: String,
) -> Result<(), String> {
    let command = format!("chmod {} \"{}\"", perms, path);
    super::command::exec_command(app_handle, state, id, command, None)
        .await
        .map(|_| ())
}

#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;
        let mut remote_file =
            ssh2_retry(|| sftp.open(Path::new(&remote_path)))
                .map_err(|e| e.to_string())?;
        let mut local_file = std::fs::File::create(&local_path)
            .map_err(|e| e.to_string())?;

        let buffer_size = get_sftp_buffer_size(Some(&app));
        let mut buf = vec![0u8; buffer_size];
        loop {
            match remote_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    local_file.write_all(&buf[..n])
                        .map_err(|e| e.to_string())?;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }).await
}

fn upload_recursive(
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_path: &str,
    buffer_size: usize,
) -> Result<(), String> {
    if local_path.is_dir() {
        // Create remote directory
        // Ignore error if it already exists
        let _ = ssh2_retry(|| sftp.mkdir(Path::new(remote_path), 0o755));

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
            upload_recursive(sftp, &path, &new_remote, buffer_size)?;
        }
    } else {
        let mut local_file = std::fs::File::open(local_path)
            .map_err(|e| e.to_string())?;
        let mut remote_file = ssh2_retry(|| sftp.create(Path::new(remote_path)))
            .map_err(|e| e.to_string())?;

        let mut buf = vec![0u8; buffer_size];
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
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        let buffer_size = get_sftp_buffer_size(Some(&app));
        upload_recursive(
            &sftp,
            Path::new(&local_path),
            &remote_path,
            buffer_size,
        )?;

        Ok(())
    }).await
}


fn upload_recursive_progress(
    sess: &ssh2::Session,
    sftp: &ssh2::Sftp,
    local_path: &Path,
    remote_path: &str,
    cancel_flag: &AtomicBool,
    app: &AppHandle,
    transfer_id: &str,
    total_size: u64,
    transferred: &mut u64,
    resume: bool,
    last_emit_time: &mut std::time::Instant,
) -> Result<(), String> {
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Cancelled".to_string());
    }

    if local_path.is_dir() {
        let _ = ssh2_retry(|| sftp.mkdir(Path::new(remote_path), 0o755));

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
                last_emit_time,
            )?;
        }
    } else {
        let mut local_file = std::fs::File::open(local_path)
            .map_err(|e| e.to_string())?;
        let file_size = local_file.metadata()
            .map_err(|e| e.to_string())?
            .len();

        let mut offset = 0;
        let mut remote_file;

        if resume {
            // Check remote file size
            let remote_path_path = Path::new(remote_path);
            match ssh2_retry(|| sftp.stat(remote_path_path)) {
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
                                use md5::Digest;
                                let mut hasher = md5::Md5::new();
                                let mut file =
                                    std::fs::File::open(local_path)
                                        .map_err(|e| e.to_string())?;
                                let mut buf = [0u8; 8192];
                                let mut read = 0u64;
                                loop {
                                    let n = file.read(&mut buf)
                                        .map_err(|e| e.to_string())?;
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
            remote_file = ssh2_retry(|| {
                sftp.open_mode(
                    Path::new(remote_path),
                    ssh2::OpenFlags::WRITE,
                    0o644,
                    ssh2::OpenType::File,
                )
            }).map_err(|e| e.to_string())?;

            // Seek local
            local_file
                .seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;

            // ssh2::File implements Seek
            remote_file
                .seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
        } else {
            remote_file = ssh2_retry(|| sftp.create(Path::new(remote_path)))
                .map_err(|e| e.to_string())?;
        }

        // Update global transferred count with skipped bytes
        *transferred += offset;

        let buffer_size = get_sftp_buffer_size(Some(app));
        let mut buf = vec![0u8; buffer_size];
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

                                // Throttle progress updates: emit at most every 100ms
                                if last_emit_time.elapsed() >= Duration::from_millis(100) {
                                    let _ = app.emit(
                                        "transfer-progress",
                                        ProgressPayload {
                                            id: transfer_id.to_string(),
                                            transferred: *transferred,
                                            total: total_size,
                                        },
                                    );
                                    *last_emit_time = std::time::Instant::now();
                                }
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

        // Ensure final progress is sent
        let _ = app.emit(
            "transfer-progress",
            ProgressPayload {
                id: transfer_id.to_string(),
                transferred: *transferred,
                total: total_size,
            },
        );
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
    // Extract the client and set up cancellation flag before async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    // Add cancellation flag to state
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut transfers = state.transfers.lock().map_err(|e| e.to_string())?;
        transfers.insert(transfer_id.clone(), cancel_flag.clone());
    }

    let transfer_id_clone = transfer_id.clone();
    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;

        // Hold the SSH session lock throughout the entire upload operation
        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        let local_p = Path::new(&local_path);
        let total_size = if local_p.is_dir() {
            get_dir_size(local_p)
        } else {
            local_p.metadata()
                .map_err(|e| e.to_string())?
                .len()
        };

        let mut transferred = 0;
        let mut last_emit_time = std::time::Instant::now();

        let result = upload_recursive_progress(
            &sess,
            &sftp,
            local_p,
            &remote_path,
            &cancel_flag,
            &app,
            &transfer_id_clone,
            total_size,
            &mut transferred,
            resume,
            &mut last_emit_time,
        );

        result
    }).await?;

    // Remove cancellation flag after operation completes
    {
        let mut transfers = state.transfers.lock().map_err(|e| e.to_string())?;
        transfers.remove(&transfer_id);
    }

    Ok(())
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
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    // Set up cancellation flag
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut transfers = state.transfers.lock().map_err(|e| e.to_string())?;
        transfers.insert(transfer_id.clone(), cancel_flag.clone());
    }

    let transfer_id_clone = transfer_id.clone();
    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;

        let sess = bg_session.lock().unwrap();
        let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

        // Get remote size
        let remote_path_path = Path::new(&remote_path);
        let total_size = ssh2_retry(|| sftp.stat(remote_path_path))
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
                            id: transfer_id_clone.clone(),
                            transferred: total_size,
                            total: total_size,
                        },
                    );
                    return Ok(());
                }
            } else {
                local_file = std::fs::File::create(&local_path)
                    .map_err(|e| e.to_string())?;
            }
        } else {
            local_file = std::fs::File::create(&local_path)
                .map_err(|e| e.to_string())?;
        }

        let mut remote_file = ssh2_retry(|| sftp.open(remote_path_path))
            .map_err(|e| e.to_string())?;
        if offset > 0 {
            remote_file
                .seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
        }

        let mut transferred = offset;
        let buffer_size = get_sftp_buffer_size(Some(&app));
        let mut buf = vec![0u8; buffer_size];
        let mut last_emit_time = std::time::Instant::now();

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err("Cancelled".to_string());
            }

            match remote_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    local_file.write_all(&buf[..n])
                        .map_err(|e| e.to_string())?;
                    transferred += n as u64;

                    if last_emit_time.elapsed() >= Duration::from_millis(100) {
                        let _ = app.emit(
                            "transfer-progress",
                            ProgressPayload {
                                id: transfer_id_clone.clone(),
                                transferred,
                                total: total_size,
                            },
                        );
                        last_emit_time = std::time::Instant::now();
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }

        // Ensure final progress is sent
        let _ = app.emit(
            "transfer-progress",
            ProgressPayload {
                id: transfer_id_clone.clone(),
                transferred: total_size,
                total: total_size,
            },
        );
        Ok(())
    }).await
}

#[tauri::command]
pub async fn search_remote_files(
    state: State<'_, AppState>,
    id: String,
    root: String,
    pattern: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    // Extract the client before moving into async operation
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    execute_ssh_operation(move || {
        let bg_session = client
            .ssh_pool
            .get_background_session()
            .map_err(|e| format!("Failed to get background session: {}", e))?;
        let sess = bg_session.lock().unwrap();
        let mut channel = ssh2_retry(|| sess.channel_session())
            .map_err(|e| e.to_string())?;

        let limit = max_results.unwrap_or(200);
        // Use grep -R with head to limit number of lines
        let safe_root = root.replace("'", "'\\''");
        let safe_pattern = pattern.replace("'", "'\\''");
        let cmd = format!(
            "cd '{}' && grep -R -n --line-number --text -- '{}' | head -n {}",
            safe_root, safe_pattern, limit
        );

        ssh2_retry(|| channel.exec(&cmd))
            .map_err(|e| e.to_string())?;

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
    }).await
}