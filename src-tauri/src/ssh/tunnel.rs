use crate::db;
use crate::models::{Connection as SshConnection, SshKey, Tunnel, TunnelStatus};
use crate::ssh::client::AppState;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::MutexGuard;
use std::time::Duration;
use tauri::{AppHandle, State};
use tempfile::{Builder as TempBuilder, TempPath};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct TunnelRuntime {
    pub child: Child,
    pub key_path: Option<TempPath>,
    pub askpass_path: Option<TempPath>,
}

fn normalize_host(value: Option<&String>, default_value: &str) -> String {
    value
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_else(|| default_value.to_string())
}

fn require_port(value: Option<u16>, field: &str) -> Result<u16, String> {
    value.ok_or_else(|| format!("{} is required", field))
}

#[cfg(unix)]
fn set_executable(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(|e| e.to_string())?
        .permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(|e| e.to_string())?
        .permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn set_executable(_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn set_private_permissions(_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}

fn escape_sh_single_quotes(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

fn create_key_file(content: &str) -> Result<TempPath, String> {
    let mut file = TempBuilder::new()
        .prefix("ssh_key_")
        .tempfile()
        .map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    set_private_permissions(file.path())?;
    Ok(file.into_temp_path())
}

fn create_askpass_script(value: &str) -> Result<TempPath, String> {
    #[cfg(unix)]
    let mut file = TempBuilder::new()
        .prefix("ssh_askpass_")
        .suffix(".sh")
        .tempfile()
        .map_err(|e| e.to_string())?;

    #[cfg(windows)]
    let mut file = TempBuilder::new()
        .prefix("ssh_askpass_")
        .suffix(".cmd")
        .tempfile()
        .map_err(|e| e.to_string())?;

    #[cfg(unix)]
    let script = format!("#!/bin/sh\necho '{}'\n", escape_sh_single_quotes(value));

    #[cfg(windows)]
    let script = format!("@echo off\r\necho {}\r\n", value);

    file.write_all(script.as_bytes())
        .map_err(|e| e.to_string())?;
    set_executable(file.path())?;
    Ok(file.into_temp_path())
}

fn apply_askpass_env(cmd: &mut Command, askpass: &TempPath) {
    cmd.env("SSH_ASKPASS", askpass.as_os_str());
    cmd.env("SSH_ASKPASS_REQUIRE", "force");
    #[cfg(unix)]
    if std::env::var("DISPLAY").is_err() {
        cmd.env("DISPLAY", ":0");
    }
}

fn prepare_ssh_command(
    tunnel: &Tunnel,
    connection: &SshConnection,
    key: Option<&SshKey>,
) -> Result<(Command, Option<TempPath>, Option<TempPath>), String> {
    if connection.host.starts_with("wsl://") {
        return Err("WSL connections do not support SSH tunnel proxying".to_string());
    }

    let tunnel_type = tunnel.tunnel_type.trim().to_lowercase();
    let local_host = normalize_host(tunnel.local_host.as_ref(), "127.0.0.1");

    let forward_arg = match tunnel_type.as_str() {
        "local" => {
            let local_port = require_port(tunnel.local_port, "Local port")?;
            let remote_host = tunnel
                .remote_host
                .as_ref()
                .and_then(|s| {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .ok_or_else(|| "Remote host is required".to_string())?;
            let remote_port = require_port(tunnel.remote_port, "Remote port")?;
            format!("{}:{}:{}:{}", local_host, local_port, remote_host, remote_port)
        }
        "remote" => {
            let remote_bind_host =
                normalize_host(tunnel.remote_bind_host.as_ref(), "127.0.0.1");
            let remote_port = require_port(tunnel.remote_port, "Remote port")?;
            let local_port = require_port(tunnel.local_port, "Local port")?;
            format!(
                "{}:{}:{}:{}",
                remote_bind_host, remote_port, local_host, local_port
            )
        }
        "dynamic" => {
            let local_port = require_port(tunnel.local_port, "Local port")?;
            format!("{}:{}", local_host, local_port)
        }
        _ => return Err("Unsupported tunnel type".to_string()),
    };

    let auth_type = connection.auth_type.as_deref().unwrap_or("password");

    let mut key_path = None;
    let mut askpass_path = None;

    if auth_type == "key" {
        let key = key.ok_or_else(|| "SSH key not found for tunnel".to_string())?;
        let file = create_key_file(&key.content)?;
        key_path = Some(file);
        if let Some(passphrase) = &key.passphrase {
            if !passphrase.trim().is_empty() {
                askpass_path = Some(create_askpass_script(passphrase)?);
            }
        }
    } else if auth_type == "password" {
        let password = connection
            .password
            .as_ref()
            .ok_or_else(|| "Password is required for password authentication".to_string())?;
        askpass_path = Some(create_askpass_script(password)?);
    }

    let proxy_command = tunnel
        .proxy_command
        .as_ref()
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

    let mut proxy_jump = tunnel
        .proxy_jump
        .as_ref()
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

    if proxy_command.is_none() && proxy_jump.is_none() {
        if let Some(jump_host) = connection.jump_host.as_ref() {
            let trimmed = jump_host.trim();
            if !trimmed.is_empty() {
                let mut jump = String::new();
                if let Some(user) = connection.jump_username.as_ref() {
                    let user_trimmed = user.trim();
                    if !user_trimmed.is_empty() {
                        jump.push_str(user_trimmed);
                        jump.push('@');
                    }
                }
                jump.push_str(trimmed);
                if let Some(port) = connection.jump_port {
                    jump.push_str(&format!(":{}", port));
                }
                proxy_jump = Some(jump);
            }
        }
    }

    if proxy_command.is_some() {
        proxy_jump = None;
    }

    let mut cmd = Command::new("ssh");
    cmd.arg("-N")
        .arg("-T")
        .arg("-o")
        .arg("ExitOnForwardFailure=yes")
        .arg("-o")
        .arg("ServerAliveInterval=30")
        .arg("-o")
        .arg("ServerAliveCountMax=3")
        .arg("-p")
        .arg(connection.port.to_string());

    if auth_type == "password" {
        cmd.arg("-o").arg("PreferredAuthentications=password");
    }

    if let Some(ref proxy_command) = proxy_command {
        cmd.arg("-o")
            .arg(format!("ProxyCommand={}", proxy_command));
    } else if let Some(ref proxy_jump) = proxy_jump {
        cmd.arg("-J").arg(proxy_jump);
    }

    if tunnel.agent_forwarding.unwrap_or(false) {
        cmd.arg("-A").arg("-o").arg("ForwardAgent=yes");
    }

    if let Some(ref key_path) = key_path {
        cmd.arg("-i").arg(key_path.as_os_str());
    }

    match tunnel_type.as_str() {
        "local" => {
            cmd.arg("-L").arg(forward_arg);
        }
        "remote" => {
            cmd.arg("-R").arg(forward_arg);
        }
        "dynamic" => {
            cmd.arg("-D").arg(forward_arg);
        }
        _ => {}
    }

    if let Some(ref askpass_path) = askpass_path {
        apply_askpass_env(&mut cmd, askpass_path);
    }

    cmd.arg(format!("{}@{}", connection.username, connection.host))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    Ok((cmd, key_path, askpass_path))
}

fn tunnel_status_from_runtime(id: i64, runtime: &TunnelRuntime) -> TunnelStatus {
    TunnelStatus {
        id,
        active: true,
        pid: Some(runtime.child.id()),
    }
}

fn cleanup_inactive_tunnels(tunnels: &mut MutexGuard<'_, std::collections::HashMap<i64, TunnelRuntime>>) {
    let mut ended_ids = Vec::new();
    for (id, runtime) in tunnels.iter_mut() {
        if let Ok(Some(_)) = runtime.child.try_wait() {
            ended_ids.push(*id);
        }
    }
    for id in ended_ids {
        tunnels.remove(&id);
    }
}

#[tauri::command]
pub fn get_active_tunnels(state: State<'_, AppState>) -> Result<Vec<TunnelStatus>, String> {
    let mut tunnels = state.tunnels.lock().map_err(|e| e.to_string())?;
    cleanup_inactive_tunnels(&mut tunnels);
    let mut statuses = Vec::new();
    for (id, runtime) in tunnels.iter() {
        statuses.push(tunnel_status_from_runtime(*id, runtime));
    }
    Ok(statuses)
}

#[tauri::command]
pub fn start_tunnel(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<TunnelStatus, String> {
    let mut tunnels = state.tunnels.lock().map_err(|e| e.to_string())?;
    cleanup_inactive_tunnels(&mut tunnels);

    if let Some(runtime) = tunnels.get(&id) {
        return Ok(tunnel_status_from_runtime(id, runtime));
    }

    let tunnel = db::get_tunnel_by_id(&app_handle, id)?
        .ok_or_else(|| "Tunnel not found".to_string())?;

    let connection = db::get_connection_by_id(&app_handle, tunnel.connection_id)?
        .ok_or_else(|| "Connection not found for tunnel".to_string())?;

    let key = if connection.auth_type.as_deref() == Some("key") {
        if let Some(key_id) = connection.ssh_key_id {
            db::get_ssh_key_by_id(&app_handle, key_id)?
        } else {
            None
        }
    } else {
        None
    };

    let (mut cmd, key_path, askpass_path) = prepare_ssh_command(&tunnel, &connection, key.as_ref())?;

    let mut child = cmd.spawn().map_err(|e| format!("Failed to start SSH tunnel: {}", e))?;

    std::thread::sleep(Duration::from_millis(200));
    if let Ok(Some(status)) = child.try_wait() {
        let mut err_output = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut err_output);
        }
        let err_output = err_output.trim();
        let detail = if err_output.is_empty() {
            format!("SSH tunnel exited with status {}", status)
        } else {
            err_output.to_string()
        };
        return Err(detail);
    }

    let status = TunnelStatus {
        id,
        active: true,
        pid: Some(child.id()),
    };

    tunnels.insert(
        id,
        TunnelRuntime {
            child,
            key_path,
            askpass_path,
        },
    );

    Ok(status)
}

#[tauri::command]
pub fn stop_tunnel(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut tunnels = state.tunnels.lock().map_err(|e| e.to_string())?;

    if let Some(mut runtime) = tunnels.remove(&id) {
        let _ = runtime.child.kill();
        let _ = runtime.child.wait();
        return Ok(());
    }

    Err("Tunnel is not running".to_string())
}
