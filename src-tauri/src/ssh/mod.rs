// Connection timeout constants
pub const DEFAULT_CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
pub const JUMP_HOST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub const LOCAL_FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub const CONNECTION_RETRY_BASE_DELAY: std::time::Duration = std::time::Duration::from_millis(500);
pub const CONNECTION_RETRY_MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone)]
pub enum ShellMsg {
    Data(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Exit,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandOutputEvent {
    pub data: String,
}

pub mod client;
pub mod connection;
pub mod file_ops;
pub mod terminal;
pub mod command;
pub mod utils;

// Re-export main types and functions for backward compatibility
pub use client::{SshClient, AppState};
pub use connection::{ManagedSession, SessionSshPool, ForwardingThreadHandle};
pub use utils::{ssh2_retry, execute_ssh_operation, get_sftp_buffer_size, compute_local_file_hash, get_dir_size};