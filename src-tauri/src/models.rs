use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: Option<i64>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    // Jump host configuration
    pub jump_host: Option<String>,
    pub jump_port: Option<u16>,
    pub jump_username: Option<String>,
    pub jump_password: Option<String>,
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
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub ai: AIConfig,
    pub terminal_appearance: TerminalAppearanceSettings,
}
