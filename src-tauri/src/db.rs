use crate::models::{
    AIConfig, AppSettings, Connection as SshConnection, ConnectionGroup, FileManagerSettings,
    SshPoolSettings, TerminalAppearanceSettings,
};
use rusqlite::{params, Connection, Result};
use tauri::{AppHandle, Manager};

pub fn get_db_path(app_handle: &AppHandle) -> std::path::PathBuf {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to get app data dir");
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");
    }
    app_dir.join("ssh_assistant.db")
}

pub fn init_db(app_handle: &AppHandle) -> Result<()> {
    let db_path = get_db_path(app_handle);
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            username TEXT NOT NULL,
            password TEXT
        )",
        [],
    )?;

    conn.execute(
        r#"CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            theme TEXT NOT NULL DEFAULT 'dark',
            language TEXT NOT NULL DEFAULT 'zh',
            ai_api_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1',
            ai_api_key TEXT NOT NULL DEFAULT '',
            ai_model_name TEXT NOT NULL DEFAULT 'gpt-3.5-turbo',
            terminal_font_size INTEGER NOT NULL DEFAULT 14,
            terminal_font_family TEXT NOT NULL DEFAULT 'Menlo, Monaco, "Courier New", monospace',
            terminal_cursor_style TEXT NOT NULL DEFAULT 'block',
            terminal_line_height REAL NOT NULL DEFAULT 1.0
        )"#,
        [],
    )?;

    // Ensure default row exists
    conn.execute("INSERT OR IGNORE INTO settings (id) VALUES (1)", [])?;

    // Migrations: Add jump host columns if they don't exist
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_host TEXT", []);
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_port INTEGER", []);
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_username TEXT", []);
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_password TEXT", []);

    // Migrations: Add terminal appearance columns if they don't exist
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN terminal_font_size INTEGER NOT NULL DEFAULT 14"#,
        [],
    );
    let _ = conn.execute(r#"ALTER TABLE settings ADD COLUMN terminal_font_family TEXT NOT NULL DEFAULT 'Menlo, Monaco, "Courier New", monospace'"#, []);
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN terminal_cursor_style TEXT NOT NULL DEFAULT 'block'"#,
        [],
    );
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN terminal_line_height REAL NOT NULL DEFAULT 1.0"#,
        [],
    );

    // Migration: Add file manager view mode
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN file_manager_view_mode TEXT NOT NULL DEFAULT 'flat'"#,
        [],
    );

    // Migration: Add SSH pool settings
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN ssh_max_background_sessions INTEGER NOT NULL DEFAULT 3"#,
        [],
    );
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN ssh_enable_auto_cleanup INTEGER NOT NULL DEFAULT 1"#,
        [],
    );
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN ssh_cleanup_interval_minutes INTEGER NOT NULL DEFAULT 5"#,
        [],
    );

    // Migration: Add SFTP buffer size
    let _ = conn.execute(
        r#"ALTER TABLE settings ADD COLUMN file_manager_sftp_buffer_size INTEGER NOT NULL DEFAULT 512"#,
        [],
    );

    // Groups table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS connection_groups (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            parent_id INTEGER,
            FOREIGN KEY(parent_id) REFERENCES connection_groups(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Migration: Add group_id to connections
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN group_id INTEGER REFERENCES connection_groups(id) ON DELETE SET NULL", []);

    // Migration: Add os_type to connections with default 'Linux'
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN os_type TEXT NOT NULL DEFAULT 'Linux'", []);

    Ok(())
}

#[tauri::command]
pub fn get_connections(app_handle: AppHandle) -> Result<Vec<SshConnection>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare("SELECT id, name, host, port, username, password, jump_host, jump_port, jump_username, jump_password, group_id, os_type FROM connections")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SshConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                port: row.get(3)?,
                username: row.get(4)?,
                password: row.get(5)?,
                jump_host: row.get(6)?,
                jump_port: row.get(7)?,
                jump_username: row.get(8)?,
                jump_password: row.get(9)?,
                group_id: row.get(10)?,
                os_type: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut connections = Vec::new();
    for row in rows {
        connections.push(row.map_err(|e| e.to_string())?);
    }
    Ok(connections)
}

#[tauri::command]
pub fn get_groups(app_handle: AppHandle) -> Result<Vec<ConnectionGroup>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, name, parent_id FROM connection_groups")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ConnectionGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut groups = Vec::new();
    for row in rows {
        groups.push(row.map_err(|e| e.to_string())?);
    }
    Ok(groups)
}

#[tauri::command]
pub fn create_connection(app_handle: AppHandle, conn: SshConnection) -> Result<(), String> {
    println!("Creating connection: {:?}", conn);
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    db_conn.execute(
        "INSERT INTO connections (name, host, port, username, password, jump_host, jump_port, jump_username, jump_password, group_id, os_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![conn.name, conn.host, conn.port, conn.username, conn.password, conn.jump_host, conn.jump_port, conn.jump_username, conn.jump_password, conn.group_id, conn.os_type],
    ).map_err(|e| {
        println!("Error inserting connection: {}", e);
        e.to_string()
    })?;
    println!("Connection created successfully");
    Ok(())
}

#[tauri::command]
pub fn update_connection(app_handle: AppHandle, conn: SshConnection) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    db_conn.execute(
        "UPDATE connections SET name=?1, host=?2, port=?3, username=?4, password=?5, jump_host=?6, jump_port=?7, jump_username=?8, jump_password=?9, group_id=?10, os_type=?11 WHERE id=?12",
        params![conn.name, conn.host, conn.port, conn.username, conn.password, conn.jump_host, conn.jump_port, conn.jump_username, conn.jump_password, conn.group_id, conn.os_type, conn.id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_connection(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    db_conn
        .execute("DELETE FROM connections WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn create_group(app_handle: AppHandle, group: ConnectionGroup) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    db_conn
        .execute(
            "INSERT INTO connection_groups (name, parent_id) VALUES (?1, ?2)",
            params![group.name, group.parent_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_group(app_handle: AppHandle, group: ConnectionGroup) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    db_conn
        .execute(
            "UPDATE connection_groups SET name=?1, parent_id=?2 WHERE id=?3",
            params![group.name, group.parent_id, group.id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_group(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Note: ON DELETE CASCADE on parent_id handles subgroups
    // But for connections, we set group_id to NULL (ON DELETE SET NULL)
    db_conn
        .execute("DELETE FROM connection_groups WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(app_handle: AppHandle) -> Result<AppSettings, String> {
    let db_path = get_db_path(&app_handle);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare("SELECT theme, language, ai_api_url, ai_api_key, ai_model_name, terminal_font_size, terminal_font_family, terminal_cursor_style, terminal_line_height, file_manager_view_mode, ssh_max_background_sessions, ssh_enable_auto_cleanup, ssh_cleanup_interval_minutes, file_manager_sftp_buffer_size FROM settings WHERE id = 1")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt
        .query_map([], |row| {
            Ok(AppSettings {
                theme: row.get(0)?,
                language: row.get(1)?,
                ai: AIConfig {
                    api_url: row.get(2)?,
                    api_key: row.get(3)?,
                    model_name: row.get(4)?,
                },
                terminal_appearance: TerminalAppearanceSettings {
                    font_size: row.get::<_, Option<i32>>(5)?.unwrap_or(14),
                    font_family: row
                        .get::<_, Option<String>>(6)?
                        .unwrap_or_else(|| "Menlo, Monaco, \"Courier New\", monospace".to_string()),
                    cursor_style: row
                        .get::<_, Option<String>>(7)?
                        .unwrap_or_else(|| "block".to_string()),
                    line_height: row.get::<_, Option<f32>>(8)?.unwrap_or(1.0),
                },
                file_manager: FileManagerSettings {
                    view_mode: row
                        .get::<_, Option<String>>(9)?
                        .unwrap_or_else(|| "flat".to_string()),
                    sftp_buffer_size: row.get::<_, Option<i32>>(13)?.unwrap_or(512),
                },
                ssh_pool: SshPoolSettings {
                    max_background_sessions: row.get::<_, Option<i32>>(10)?.unwrap_or(3),
                    enable_auto_cleanup: row.get::<_, Option<bool>>(11)?.unwrap_or(true),
                    cleanup_interval_minutes: row.get::<_, Option<i32>>(12)?.unwrap_or(5),
                },
            })
        })
        .map_err(|e| e.to_string())?;

    if let Some(row) = rows.next() {
        row.map_err(|e| e.to_string())
    } else {
        Err("Settings not found".to_string())
    }
}

#[tauri::command]
pub fn save_settings(app_handle: AppHandle, settings: AppSettings) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE settings SET theme=?1, language=?2, ai_api_url=?3, ai_api_key=?4, ai_model_name=?5, terminal_font_size=?6, terminal_font_family=?7, terminal_cursor_style=?8, terminal_line_height=?9, file_manager_view_mode=?10, ssh_max_background_sessions=?11, ssh_enable_auto_cleanup=?12, ssh_cleanup_interval_minutes=?13, file_manager_sftp_buffer_size=?14 WHERE id = 1",
        params![
            settings.theme,
            settings.language,
            settings.ai.api_url,
            settings.ai.api_key,
            settings.ai.model_name,
            settings.terminal_appearance.font_size,
            settings.terminal_appearance.font_family,
            settings.terminal_appearance.cursor_style,
            settings.terminal_appearance.line_height,
            settings.file_manager.view_mode,
            settings.ssh_pool.max_background_sessions,
            settings.ssh_pool.enable_auto_cleanup,
            settings.ssh_pool.cleanup_interval_minutes,
            settings.file_manager.sftp_buffer_size,
        ],
    ).map_err(|e| e.to_string())?;

    Ok(())
}
