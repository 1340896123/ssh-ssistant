use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SshKey {
    pub id: Option<i64>,
    pub name: String,
    pub content: String,
    pub passphrase: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: Option<i64>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub auth_type: Option<String>, // "password" or "key", default "password"
    pub ssh_key_id: Option<i64>,
    // Jump host configuration
    pub jump_host: Option<String>,
    pub jump_port: Option<u16>,
    pub jump_username: Option<String>,
    pub jump_password: Option<String>,
    pub group_id: Option<i64>,
    pub os_type: Option<String>, // Default "Linux" for backward compatibility

    // Internal use for connection (not stored in connections table)
    pub key_content: Option<String>,
    pub key_passphrase: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionGroup {
    pub id: Option<i64>,
    pub name: String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: i64, // Unix timestamp
    pub permissions: u32,
    pub uid: u32,
    pub owner: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AIConfig {
    pub api_url: String,
    pub api_key: String,
    pub model_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminalAppearanceSettings {
    pub font_size: i32,
    pub font_family: String,
    pub cursor_style: String,
    pub line_height: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileManagerSettings {
    pub view_mode: String,
    pub sftp_buffer_size: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SshPoolSettings {
    pub max_background_sessions: i32,
    pub enable_auto_cleanup: bool,
    pub cleanup_interval_minutes: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub ai: AIConfig,
    pub terminal_appearance: TerminalAppearanceSettings,
    pub file_manager: FileManagerSettings,
    pub ssh_pool: SshPoolSettings,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub local_path: String,
    pub remote_path: String,
    pub transfer_type: String, // "upload" | "download"
    pub status: String, // "pending" | "running" | "paused" | "completed" | "error" | "cancelled"
    pub total_size: u64,
    pub transferred: u64,
    pub created_at: i64,
    pub error: Option<String>,
}
