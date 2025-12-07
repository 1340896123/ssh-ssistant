use crate::db;
use crate::models::{Connection, ConnectionGroup};
use std::process::Command;
use tauri::AppHandle;

pub fn get_distributions() -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let output = Command::new("wsl")
            .arg("--list")
            .arg("--quiet")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to execute wsl command: {}", e))?;

        if !output.status.success() {
            return Err("WSL list command failed".to_string());
        }

        let raw_bytes = output.stdout;
        let is_utf16 = raw_bytes.len() >= 2 && raw_bytes[1] == 0;
        let mut distros = Vec::new();

        if is_utf16 {
            // Basic UTF-16 LE conversion
            let u16_vec: Vec<u16> = raw_bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let s = String::from_utf16_lossy(&u16_vec);
            for line in s.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    distros.push(trimmed.to_string());
                }
            }
        } else {
            let stdout = String::from_utf8_lossy(&raw_bytes);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    distros.push(trimmed.to_string());
                }
            }
        }

        Ok(distros)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}

pub fn import_wsl_to_db(app: &AppHandle) -> Result<(), String> {
    let distros = get_distributions()?;
    if distros.is_empty() {
        return Ok(());
    }

    // Get existing connections to avoid duplicates
    let current_connections = db::get_connections(app.clone())?;

    // Check if "WSL" group exists, if not create it
    let groups = db::get_groups(app.clone())?;
    let mut wsl_group_id = None;

    // Check for "WSL" or legacy "WSL (Auto Detected)"
    for group in &groups {
        if group.name == "WSL" {
            wsl_group_id = group.id;
            break;
        } else if group.name == "WSL (Auto Detected)" {
            // Rename legacy group to "WSL"
            let mut new_group = group.clone();
            new_group.name = "WSL".to_string();
            db::update_group(app.clone(), new_group)?;
            wsl_group_id = group.id;
            break;
        }
    }

    if wsl_group_id.is_none() {
        // Create group
        db::create_group(
            app.clone(),
            ConnectionGroup {
                id: None,
                name: "WSL".to_string(),
                parent_id: None,
            },
        )?;
        // Retrieve it back to get ID
        let updated_groups = db::get_groups(app.clone())?;
        for group in updated_groups {
            if group.name == "WSL" {
                wsl_group_id = group.id;
                break;
            }
        }
    }

    for distro in distros {
        let host_str = format!("wsl://{}", distro);

        // Check if exists
        let exists = current_connections.iter().any(|c| c.host == host_str);
        if exists {
            continue;
        }

        // Add new connection
        let new_conn = Connection {
            id: None,
            name: distro.clone(),
            host: host_str,
            port: 0,                      // Not used for WSL
            username: "root".to_string(), // Default usually
            password: None,
            jump_host: None,
            jump_port: None,
            jump_username: None,
            jump_password: None,
            group_id: wsl_group_id,
            os_type: Some("Linux".to_string()),
        };

        db::create_connection(app.clone(), new_conn)?;
    }

    Ok(())
}
