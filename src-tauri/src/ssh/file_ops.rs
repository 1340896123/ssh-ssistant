use super::client::{AppState, ClientType};
use super::utils::{
    compute_local_file_hash, get_dir_size, get_remote_file_hash, get_sftp_buffer_size,
};
use crate::models::FileEntry;
use crate::ssh::{execute_ssh_operation, ssh2_retry};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
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

fn to_wsl_path(distro: &str, path: &str) -> PathBuf {
    let clean_path = path.replace("/", "\\");
    let trimmed = clean_path.trim_start_matches('\\');
    PathBuf::from(format!("\\\\wsl$\\{}\\{}", distro, trimmed))
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
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;
                let sess = bg_session.lock().unwrap();
                let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

                let mut file =
                    ssh2_retry(|| sftp.open(Path::new(&path))).map_err(|e| e.to_string())?;
                let mut buf = Vec::new();

                let mut temp_buf = vec![0u8; 32 * 1024];
                loop {
                    match file.read(&mut temp_buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&temp_buf[..n]);
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
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let wsl_path = to_wsl_path(&distro, &path);
                let mut file = std::fs::File::open(wsl_path).map_err(|e| e.to_string())?;
                let mut buf = Vec::new();
                if let Some(max) = max_bytes {
                    let mut handle = file.take(max);
                    handle.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                } else {
                    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                }
                String::from_utf8(buf).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
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
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
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
                    })
                    .map_err(|e| e.to_string())?
                } else {
                    use ssh2::OpenFlags;
                    ssh2_retry(|| {
                        sftp.open_mode(
                            Path::new(&path),
                            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                            0o644,
                            ssh2::OpenType::File,
                        )
                    })
                    .map_err(|e| e.to_string())?
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
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let wsl_path = to_wsl_path(&distro, &path);
                let open_mode = mode.unwrap_or_else(|| "overwrite".to_string());

                let mut options = std::fs::OpenOptions::new();
                options.write(true).create(true);
                if open_mode == "append" {
                    options.append(true);
                } else {
                    options.truncate(true);
                }

                let mut file = options.open(wsl_path).map_err(|e| e.to_string())?;
                file.write_all(content.as_bytes())
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Vec<FileEntry>, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
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
                            let owner = {
                                if let Ok(mut cache) = owner_cache.lock() {
                                    if let Some(cached) = cache.get(&uid) {
                                        cached.clone()
                                    } else {
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
                                                            Ok(n) => username_data.push_str(
                                                                &String::from_utf8_lossy(&buf[..n]),
                                                            ),
                                                            Err(e)
                                                                if e.kind()
                                                                    == ErrorKind::WouldBlock =>
                                                            {
                                                                thread::sleep(
                                                                    Duration::from_millis(10),
                                                                );
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
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let wsl_path = to_wsl_path(&distro, &path);
                let entries = std::fs::read_dir(wsl_path).map_err(|e| e.to_string())?;
                let mut file_entries = Vec::new();
                for entry in entries {
                    let entry = entry.map_err(|e| e.to_string())?;
                    let meta = entry.metadata().map_err(|e| e.to_string())?;
                    let name = entry.file_name().to_string_lossy().to_string();

                    file_entries.push(FileEntry {
                        name,
                        is_dir: meta.is_dir(),
                        size: meta.len(),
                        mtime: meta
                            .modified()
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                        permissions: 0o755,
                        uid: 0,
                        owner: "root".to_string(),
                    });
                }

                file_entries.sort_by(|a, b| {
                    if a.is_dir == b.is_dir {
                        a.name.cmp(&b.name)
                    } else {
                        b.is_dir.cmp(&a.is_dir)
                    }
                });
                Ok(file_entries)
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
}

#[tauri::command]
pub async fn create_directory(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
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
                            Err(format!("Failed to create directory '{}': {}", path, error_msg))
                        }
                    }
                }
            }).await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let wsl_path = to_wsl_path(&distro, &path);
                std::fs::create_dir(wsl_path).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
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
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
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
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let wsl_path = to_wsl_path(&distro, &path);
                std::fs::File::create(wsl_path).map_err(|e| e.to_string())?;
                Ok(())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
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
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;
                let sess = bg_session.lock().unwrap();
                let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;
                if is_dir {
                    rm_recursive(&sftp, Path::new(&path))
                } else {
                    ssh2_retry(|| sftp.unlink(Path::new(&path))).map_err(|e| e.to_string())
                }
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let wsl_path = to_wsl_path(&distro, &path);
                if is_dir {
                    std::fs::remove_dir_all(wsl_path).map_err(|e| e.to_string())
                } else {
                    std::fs::remove_file(wsl_path).map_err(|e| e.to_string())
                }
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
}

// SSH recursive delete helper
fn rm_recursive(sftp: &ssh2::Sftp, path: &Path) -> Result<(), String> {
    // Basic implementation: read dir, unlink files, rmdir subdirs, then rmdir self
    let files = ssh2_retry(|| sftp.readdir(path)).map_err(|e| e.to_string())?;
    for (path_buf, stat) in files {
        if let Some(name) = path_buf.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str == "." || name_str == ".." {
                    continue;
                }
                if stat.is_dir() {
                    rm_recursive(sftp, &path_buf)?;
                } else {
                    ssh2_retry(|| sftp.unlink(&path_buf)).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    ssh2_retry(|| sftp.rmdir(path)).map_err(|e| e.to_string())
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
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;
                let sess = bg_session.lock().unwrap();
                let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;
                ssh2_retry(|| sftp.rename(Path::new(&old_path), Path::new(&new_path), None))
                    .map_err(|e| e.to_string())
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let wsl_old = to_wsl_path(&distro, &old_path);
                let wsl_new = to_wsl_path(&distro, &new_path);
                std::fs::rename(wsl_old, wsl_new).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
}

#[tauri::command]
pub async fn change_file_permission(
    state: State<'_, AppState>,
    id: String,
    path: String,
    permission: u32,
) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool.get_background_session().map_err(|e| e.to_string())?;
                let sess = bg_session.lock().unwrap();
                let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;
                ssh2_retry(|| {
                    sftp.setstat(
                        Path::new(&path),
                        ssh2::FileStat {
                            size: None,
                            uid: None,
                            gid: None,
                            perm: Some(permission),
                            atime: None,
                            mtime: None,
                        },
                    )
                })
                .map_err(|e| e.to_string())
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                // wsl -d distro chmod octal path
                let octal = format!("{:o}", permission);
                let output = std::process::Command::new("wsl")
                    .arg("-d")
                    .arg(&distro)
                    .arg("chmod")
                    .arg(octal)
                    .arg(&path)
                    .output()
                    .map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err(String::from_utf8_lossy(&output.stderr).to_string());
                }
                Ok(())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
}

#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut transfers = state.transfers.lock().map_err(|e| e.to_string())?;
        transfers.insert(id.clone(), cancel_flag.clone());
    }

    let id_ssh = id.clone();
    let id_wsl = id.clone();

    let result = match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let id = id_ssh;
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;
                let sess = bg_session.lock().unwrap();
                let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

                let mut remote =
                    ssh2_retry(|| sftp.open(Path::new(&remote_path))).map_err(|e| e.to_string())?;
                let mut local = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;
                let file_stat = remote.stat().map_err(|e| e.to_string())?;
                let total_size = file_stat.size.unwrap_or(0);

                let buffer_size = get_sftp_buffer_size(Some(&app));
                let mut buffer = vec![0u8; buffer_size];
                let mut transferred = 0u64;

                loop {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err("Download cancelled".to_string());
                    }
                    match remote.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            local.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                            transferred += n as u64;
                            let _ = app.emit(
                                "download-progress",
                                ProgressPayload {
                                    id: id.clone(),
                                    transferred,
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
                Ok(())
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let id = id_wsl;
                let wsl_path = to_wsl_path(&distro, &remote_path);
                let mut remote = std::fs::File::open(wsl_path).map_err(|e| e.to_string())?;
                let mut local = std::fs::File::create(&local_path).map_err(|e| e.to_string())?;
                let metadata = remote.metadata().map_err(|e| e.to_string())?;
                let total_size = metadata.len();

                let mut buffer = [0u8; 8192];
                let mut transferred = 0u64;

                loop {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err("Download cancelled".to_string());
                    }
                    let n = remote.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    local.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                    transferred += n as u64;
                    let _ = app.emit(
                        "download-progress",
                        ProgressPayload {
                            id: id.clone(),
                            transferred,
                            total: total_size,
                        },
                    );
                }
                Ok(())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    };

    {
        let mut transfers = state.transfers.lock().map_err(|e| e.to_string())?;
        transfers.remove(&id);
    }
    result
}

#[tauri::command]
pub async fn upload_file(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    upload_file_with_progress(app, state, id, local_path, remote_path).await
}

#[tauri::command]
pub async fn upload_file_with_progress(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut transfers = state.transfers.lock().map_err(|e| e.to_string())?;
        transfers.insert(id.clone(), cancel_flag.clone());
    }

    let id_ssh = id.clone();
    let id_wsl = id.clone();

    let result = match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let id = id_ssh;
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;
                let sess = bg_session.lock().unwrap();
                let sftp = ssh2_retry(|| sess.sftp()).map_err(|e| e.to_string())?;

                let mut local = std::fs::File::open(&local_path).map_err(|e| e.to_string())?;
                let metadata = local.metadata().map_err(|e| e.to_string())?;
                let total_size = metadata.len();

                let mut remote = ssh2_retry(|| sftp.create(Path::new(&remote_path)))
                    .map_err(|e| e.to_string())?;

                let buffer_size = get_sftp_buffer_size(Some(&app));
                let mut buffer = vec![0u8; buffer_size];
                let mut transferred = 0u64;

                loop {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err("Upload cancelled".to_string());
                    }
                    let n = local.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }

                    let mut pos = 0;
                    while pos < n {
                        match remote.write(&buffer[pos..n]) {
                            Ok(written) => {
                                pos += written;
                                transferred += written as u64;
                                let _ = app.emit(
                                    "upload-progress",
                                    ProgressPayload {
                                        id: id.clone(),
                                        transferred,
                                        total: total_size,
                                    },
                                );
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(10));
                                continue;
                            }
                            Err(e) => return Err(e.to_string()),
                        }
                    }
                }
                Ok(())
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let id = id_wsl;
                let wsl_path = to_wsl_path(&distro, &remote_path);

                if let Some(parent) = wsl_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                let mut local = std::fs::File::open(&local_path).map_err(|e| e.to_string())?;
                let metadata = local.metadata().map_err(|e| e.to_string())?;
                let total_size = metadata.len();

                let mut remote = std::fs::File::create(wsl_path).map_err(|e| e.to_string())?;

                let mut buffer = [0u8; 8192];
                let mut transferred = 0u64;
                loop {
                    if cancel_flag.load(Ordering::Relaxed) {
                        return Err("Upload cancelled".to_string());
                    }
                    let n = local.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    remote.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                    transferred += n as u64;
                    let _ = app.emit(
                        "upload-progress",
                        ProgressPayload {
                            id: id.clone(),
                            transferred,
                            total: total_size,
                        },
                    );
                }
                Ok(())
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    };

    {
        let mut transfers = state.transfers.lock().map_err(|e| e.to_string())?;
        transfers.remove(&id);
    }
    result
}

#[tauri::command]
pub async fn download_file_with_progress(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    download_file(app, state, id, remote_path, local_path).await
}

#[tauri::command]
pub async fn search_remote_files(
    state: State<'_, AppState>,
    id: String,
    path: String,
    query: String,
) -> Result<Vec<FileEntry>, String> {
    let client = {
        let clients = state.clients.lock().map_err(|e| e.to_string())?;
        clients.get(&id).ok_or("Session not found")?.clone()
    };

    match &client.client_type {
        ClientType::Ssh(pool) => {
            let pool = pool.clone();
            execute_ssh_operation(move || {
                let bg_session = pool
                    .get_background_session()
                    .map_err(|e| format!("Failed to get background session: {}", e))?;
                let sess = bg_session.lock().unwrap();
                let mut channel =
                    ssh2_retry(|| sess.channel_session()).map_err(|e| e.to_string())?;

                let cmd = format!("find \'{}\' -name \'*{}*\'", path, query);
                ssh2_retry(|| channel.exec(&cmd)).map_err(|e| e.to_string())?;

                let mut output = String::new();
                channel
                    .read_to_string(&mut output)
                    .map_err(|e| e.to_string())?;
                ssh2_retry(|| channel.wait_close()).ok();

                let mut entries = Vec::new();
                for line in output.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let path_buf = PathBuf::from(line);
                    let name = path_buf
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    entries.push(FileEntry {
                        name,
                        is_dir: false,
                        size: 0,
                        mtime: 0,
                        permissions: 0,
                        uid: 0,
                        owner: "".to_string(),
                    });
                }
                Ok(entries)
            })
            .await
        }
        ClientType::Wsl(distro) => {
            let distro = distro.clone();
            tokio::task::spawn_blocking(move || {
                let output = std::process::Command::new("wsl")
                    .arg("-d")
                    .arg(&distro)
                    .arg("find")
                    .arg(&path)
                    .arg("-name")
                    .arg(format!("*{}*", query))
                    .output()
                    .map_err(|e| e.to_string())?;

                let out_str = String::from_utf8_lossy(&output.stdout);
                let mut entries = Vec::new();
                for line in out_str.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let path_buf = PathBuf::from(line);
                    let name = path_buf
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    entries.push(FileEntry {
                        name,
                        is_dir: false,
                        size: 0,
                        mtime: 0,
                        permissions: 0,
                        uid: 0,
                        owner: "".to_string(),
                    });
                }
                Ok(entries)
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))?
        }
    }
}
