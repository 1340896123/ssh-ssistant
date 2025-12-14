// Connection timeout constants
pub const DEFAULT_CONNECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
pub const JUMP_HOST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub const LOCAL_FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub const CONNECTION_RETRY_BASE_DELAY: std::time::Duration = std::time::Duration::from_millis(1000);
pub const CONNECTION_RETRY_MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone)]
pub enum ShellMsg {
    Data(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Exit,
}

pub mod client;
pub mod command;
pub mod connection;
pub mod file_ops;
pub mod keys;
pub mod system;
pub mod terminal;
pub mod utils;
pub mod wsl;

// Re-export main types and functions for backward compatibility
pub use client::{AppState, SshClient};
pub use connection::{ForwardingThreadHandle, ManagedSession, SessionSshPool};
pub use utils::{
    compute_local_file_hash, execute_ssh_operation, get_dir_size, get_sftp_buffer_size, ssh2_retry,
};
