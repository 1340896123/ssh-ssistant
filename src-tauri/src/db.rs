use rusqlite::{params, Connection, Result};
use crate::models::{Connection as SshConnection, AppSettings, AIConfig, TerminalAppearanceSettings};
use tauri::{AppHandle, Manager};

pub fn get_db_path(app_handle: &AppHandle) -> std::path::PathBuf {
    let app_dir = app_handle.path().app_data_dir().expect("failed to get app data dir");
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
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            theme TEXT NOT NULL DEFAULT 'dark',
            language TEXT NOT NULL DEFAULT 'zh',
            ai_api_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1',
            ai_api_key TEXT NOT NULL DEFAULT '',
            ai_model_name TEXT NOT NULL DEFAULT 'gpt-3.5-turbo'
        )",
        [],
    )?;

    // Ensure default row exists
    conn.execute(
        "INSERT OR IGNORE INTO settings (id) VALUES (1)",
        [],
    )?;

    // Migrations: Add jump host columns if they don't exist
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_host TEXT", []);
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_port INTEGER", []);
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_username TEXT", []);
    let _ = conn.execute("ALTER TABLE connections ADD COLUMN jump_password TEXT", []);
    
    Ok(())
}

#[tauri::command]
pub fn get_connections(app_handle: AppHandle) -> Result<Vec<SshConnection>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare("SELECT id, name, host, port, username, password, jump_host, jump_port, jump_username, jump_password FROM connections")
        .map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |row| {
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
        })
    }).map_err(|e| e.to_string())?;
    
    let mut connections = Vec::new();
    for row in rows {
        connections.push(row.map_err(|e| e.to_string())?);
    }
    Ok(connections)
}

#[tauri::command]
pub fn create_connection(app_handle: AppHandle, conn: SshConnection) -> Result<(), String> {
    println!("Creating connection: {:?}", conn);
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    db_conn.execute(
        "INSERT INTO connections (name, host, port, username, password, jump_host, jump_port, jump_username, jump_password) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![conn.name, conn.host, conn.port, conn.username, conn.password, conn.jump_host, conn.jump_port, conn.jump_username, conn.jump_password],
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
        "UPDATE connections SET name=?1, host=?2, port=?3, username=?4, password=?5, jump_host=?6, jump_port=?7, jump_username=?8, jump_password=?9 WHERE id=?10",
        params![conn.name, conn.host, conn.port, conn.username, conn.password, conn.jump_host, conn.jump_port, conn.jump_username, conn.jump_password, conn.id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_connection(app_handle: AppHandle, id: i64) -> Result<(), String> {
    let db_path = get_db_path(&app_handle);
    let db_conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    db_conn.execute("DELETE FROM connections WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(app_handle: AppHandle) -> Result<AppSettings, String> {
    let db_path = get_db_path(&app_handle);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare("SELECT theme, language, ai_api_url, ai_api_key, ai_model_name FROM settings WHERE id = 1")
        .map_err(|e| e.to_string())?;
        
    let mut rows = stmt.query_map([], |row| {
        Ok(AppSettings {
            theme: row.get(0)?,
            language: row.get(1)?,
            ai: AIConfig {
                api_url: row.get(2)?,
                api_key: row.get(3)?,
                model_name: row.get(4)?,
            }
        })
    }).map_err(|e| e.to_string())?;
    
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
        "UPDATE settings SET theme=?1, language=?2, ai_api_url=?3, ai_api_key=?4, ai_model_name=?5 WHERE id = 1",
        params![
            settings.theme, 
            settings.language, 
            settings.ai.api_url, 
            settings.ai.api_key, 
            settings.ai.model_name
        ],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}
