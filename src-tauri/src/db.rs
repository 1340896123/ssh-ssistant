use rusqlite::{params, Connection, Result};
use crate::models::Connection as SshConnection;
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
    
    Ok(())
}

#[tauri::command]
pub fn get_connections(app_handle: AppHandle) -> Result<Vec<SshConnection>, String> {
    let db_path = get_db_path(&app_handle);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare("SELECT id, name, host, port, username, password FROM connections")
        .map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |row| {
        Ok(SshConnection {
            id: row.get(0)?,
            name: row.get(1)?,
            host: row.get(2)?,
            port: row.get(3)?,
            username: row.get(4)?,
            password: row.get(5)?,
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
        "INSERT INTO connections (name, host, port, username, password) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![conn.name, conn.host, conn.port, conn.username, conn.password],
    ).map_err(|e| {
        println!("Error inserting connection: {}", e);
        e.to_string()
    })?;
    println!("Connection created successfully");
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
