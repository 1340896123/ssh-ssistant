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
pub struct HostAsset {
    pub id: Option<i64>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub platform: String,
    pub folder_id: Option<i64>,
    pub env_id: Option<i64>,
    #[serde(default)]
    pub labels: Vec<String>,
    pub owner: Option<String>,
    pub criticality: String,
    pub default_workspace_path: Option<String>,
    pub access_endpoint_id: Option<i64>,
    pub bastion_chain_id: Option<String>,
    pub health_summary: Option<String>,
    pub last_accessed_at: Option<i64>,
    pub is_favorite: Option<bool>,
    pub group_id: Option<i64>,
}

impl Default for HostAsset {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            host: String::new(),
            port: 22,
            platform: "Linux".to_string(),
            folder_id: None,
            env_id: None,
            labels: Vec::new(),
            owner: None,
            criticality: "medium".to_string(),
            default_workspace_path: None,
            access_endpoint_id: None,
            bastion_chain_id: None,
            health_summary: None,
            last_accessed_at: None,
            is_favorite: Some(false),
            group_id: None,
        }
    }
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
    pub auth_type: Option<String>,
    pub ssh_key_id: Option<i64>,
    pub jump_host: Option<String>,
    pub jump_port: Option<u16>,
    pub jump_username: Option<String>,
    pub jump_password: Option<String>,
    pub group_id: Option<i64>,
    pub os_type: Option<String>,
    pub key_content: Option<String>,
    pub key_passphrase: Option<String>,
}

impl From<HostAsset> for Connection {
    fn from(value: HostAsset) -> Self {
        Self {
            id: value.id,
            name: value.name,
            host: value.host,
            port: value.port,
            username: value.owner.unwrap_or_default(),
            password: None,
            auth_type: None,
            ssh_key_id: None,
            jump_host: None,
            jump_port: None,
            jump_username: None,
            jump_password: None,
            group_id: value.folder_id.or(value.group_id),
            os_type: Some(value.platform),
            key_content: None,
            key_passphrase: None,
        }
    }
}

impl From<Connection> for HostAsset {
    fn from(value: Connection) -> Self {
        Self {
            id: value.id,
            name: value.name,
            host: value.host,
            port: value.port,
            platform: value.os_type.clone().unwrap_or_else(|| "Linux".to_string()),
            folder_id: value.group_id,
            env_id: None,
            labels: Vec::new(),
            owner: Some(value.username),
            criticality: "medium".to_string(),
            default_workspace_path: None,
            access_endpoint_id: value.id,
            bastion_chain_id: None,
            health_summary: None,
            last_accessed_at: None,
            is_favorite: Some(false),
            group_id: value.group_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetFolder {
    pub id: Option<i64>,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionGroup {
    pub id: Option<i64>,
    pub name: String,
    pub parent_id: Option<i64>,
}

impl From<AssetFolder> for ConnectionGroup {
    fn from(value: AssetFolder) -> Self {
        Self {
            id: value.id,
            name: value.name,
            parent_id: value.parent_id,
        }
    }
}

impl From<ConnectionGroup> for AssetFolder {
    fn from(value: ConnectionGroup) -> Self {
        Self {
            id: value.id,
            name: value.name,
            parent_id: value.parent_id,
            color: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: Option<i64>,
    pub name: String,
    pub slug: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetTag {
    pub id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccessEndpoint {
    pub id: Option<i64>,
    pub asset_id: i64,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: Option<String>,
    pub credential_ref_id: Option<i64>,
    pub ssh_key_id: Option<i64>,
    pub jump_host: Option<String>,
    pub jump_port: Option<u16>,
    pub jump_username: Option<String>,
    pub jump_password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRef {
    pub id: Option<i64>,
    pub name: String,
    pub credential_kind: String,
    pub username: Option<String>,
    pub secret: Option<String>,
    pub ssh_key_id: Option<i64>,
    pub asset_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetUpsertPayload {
    pub asset: HostAsset,
    pub default_access_endpoint: AccessEndpoint,
    pub default_credential_ref: Option<CredentialRef>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetSessionConnectResult {
    pub session_id: String,
    pub asset_id: i64,
    pub asset_name: String,
    pub env_id: Option<i64>,
    pub access_endpoint_id: Option<i64>,
    pub credential_ref_id: Option<i64>,
    pub bastion_chain_id: Option<String>,
    pub risk_level: String,
    pub health_summary: Option<String>,
    pub os_info: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpsSession {
    pub id: String,
    pub asset_id: i64,
    pub asset_name: String,
    pub access_endpoint_id: Option<i64>,
    pub credential_ref_id: Option<i64>,
    pub bastion_chain_id: Option<String>,
    pub current_path: Option<String>,
    pub risk_level: String,
    pub last_job_run_id: Option<i64>,
    pub health_summary: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SavedAssetView {
    pub id: Option<i64>,
    pub name: String,
    pub query_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub command: String,
    pub scope_type: String,
    pub scope_value: Option<String>,
    pub risk_level: String,
    pub requires_confirmation: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobRun {
    pub id: Option<i64>,
    pub asset_id: Option<i64>,
    pub session_id: Option<String>,
    pub template_id: Option<i64>,
    pub command: String,
    pub status: String,
    pub output: Option<String>,
    pub risk_level: String,
    pub initiated_by: Option<String>,
    pub source: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: Option<i64>,
    pub event_type: String,
    pub asset_id: Option<i64>,
    pub session_id: Option<String>,
    pub job_run_id: Option<i64>,
    pub title: String,
    pub detail: Option<String>,
    pub severity: String,
    pub metadata_json: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub id: Option<i64>,
    pub state_key: String,
    pub status: String,
    pub version: i64,
    pub endpoint_url: Option<String>,
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
    pub metadata_json: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tunnel {
    pub id: Option<i64>,
    pub name: String,
    pub connection_id: i64,
    pub tunnel_type: String, // "local" | "remote" | "dynamic"
    pub local_host: Option<String>,
    pub local_port: Option<u16>,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
    pub remote_bind_host: Option<String>,
    pub proxy_jump: Option<String>,
    pub proxy_command: Option<String>,
    pub agent_forwarding: Option<bool>,
    pub created_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub id: i64,
    pub active: bool,
    pub pid: Option<u32>,
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
    pub provider_type: String,
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
    pub layout: String,
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
pub struct ConnectionTimeoutSettings {
    pub connection_timeout_secs: u32,
    pub jump_host_timeout_secs: u32,
    pub local_forward_timeout_secs: u32,
    pub command_timeout_secs: u32,
    pub sftp_operation_timeout_secs: u32,
}

impl Default for ConnectionTimeoutSettings {
    fn default() -> Self {
        Self {
            connection_timeout_secs: 15,
            jump_host_timeout_secs: 30,
            local_forward_timeout_secs: 10,
            command_timeout_secs: 30,
            sftp_operation_timeout_secs: 60,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReconnectSettings {
    pub max_reconnect_attempts: u32, // 最大重连次数，默认 5
    pub initial_delay_ms: u32,       // 初始延迟，默认 1000ms
    pub max_delay_ms: u32,           // 最大延迟，默认 30000ms
    pub backoff_multiplier: f32,     // 退避倍数，默认 2.0
    pub enable_auto_reconnect: bool, // 是否启用自动重连，默认 true
}

impl Default for ReconnectSettings {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            enable_auto_reconnect: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatSettings {
    pub tcp_keepalive_interval_secs: u32, // TCP keepalive 间隔，默认 60
    pub ssh_keepalive_interval_secs: u32, // SSH keepalive 间隔，默认 15
    pub app_heartbeat_interval_secs: u32, // 应用层心跳间隔，默认 30
    pub heartbeat_timeout_secs: u32,      // 心跳超时，默认 5
    pub failed_heartbeats_before_action: u32, // 触发动作前的心跳失败次数，默认 3
}

impl Default for HeartbeatSettings {
    fn default() -> Self {
        Self {
            tcp_keepalive_interval_secs: 60,
            ssh_keepalive_interval_secs: 15,
            app_heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 5,
            failed_heartbeats_before_action: 3,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PoolHealthSettings {
    pub health_check_interval_secs: u32, // 健康检查间隔，默认 60
    pub session_warmup_count: u32,       // 预热会话数量，默认 1
    pub max_session_age_minutes: u32,    // 会话最大存活时间，默认 60
    pub unhealthy_threshold: u32,        // 判定为不健康的失败次数，默认 3
}

impl Default for PoolHealthSettings {
    fn default() -> Self {
        Self {
            health_check_interval_secs: 60,
            session_warmup_count: 1,
            max_session_age_minutes: 60,
            unhealthy_threshold: 3,
        }
    }
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
    #[serde(default)]
    pub connection_timeout: ConnectionTimeoutSettings,
    #[serde(default)]
    pub reconnect: ReconnectSettings,
    #[serde(default)]
    pub heartbeat: HeartbeatSettings,
    #[serde(default)]
    pub pool_health: PoolHealthSettings,
    #[serde(default)]
    pub network_adaptive: NetworkAdaptiveSettings,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatusEvent {
    pub session_id: String,
    pub status: ConnectionStatus,
    pub timestamp: i64,
    pub details: Option<String>,
    pub metrics: Option<ConnectionMetrics>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Authenticating,
    Ready,
    Degraded,
    Reconnecting,
    Disconnected,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionMetrics {
    pub uptime_secs: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub latency_ms: u32,
    pub reconnect_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum NetworkQuality {
    Excellent, // < 50ms
    Good,      // 50-150ms
    Fair,      // 150-300ms
    Poor,      // > 300ms
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdaptiveSettings {
    pub enable_adaptive: bool,             // 是否启用自适应，默认 true
    pub latency_check_interval_secs: u32,  // 延迟检测间隔，默认 30
    pub high_latency_threshold_ms: u32,    // 高延迟阈值，默认 300
    pub low_bandwidth_threshold_kbps: u32, // 低带宽阈值，默认 100
}

impl Default for NetworkAdaptiveSettings {
    fn default() -> Self {
        Self {
            enable_adaptive: true,
            latency_check_interval_secs: 30,
            high_latency_threshold_ms: 300,
            low_bandwidth_threshold_kbps: 100,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub latency_ms: u32,             // 当前延迟
    pub bandwidth_kbps: Option<u32>, // 估算带宽
    pub quality: NetworkQuality,     // 网络质量等级
    pub last_update: i64,            // 最后更新时间戳
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            latency_ms: 0,
            bandwidth_kbps: None,
            quality: NetworkQuality::Unknown,
            last_update: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveParams {
    pub heartbeat_interval_secs: u32,
    pub sftp_buffer_size: usize,
    pub command_timeout_secs: u32,
    pub keepalive_interval_secs: u32,
}

/// 文件操作错误类型（用于前端解析）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FileErrorType {
    Network,
    Permission,
    NotFound,
    Session,
    Timeout,
    Unknown,
}

/// 文件操作结构化错误响应
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileOperationError {
    /// 错误类型
    pub error_type: FileErrorType,
    /// 用户友好的错误消息
    pub message: String,
    /// 是否可重试
    pub retryable: bool,
    /// 原始错误信息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_error: Option<String>,
}

impl FileOperationError {
    /// 根据错误消息自动判断错误类型
    pub fn from_message(msg: String) -> Self {
        let msg_lower = msg.to_lowercase();
        let (error_type, retryable) = if msg_lower.contains("permission denied")
            || msg_lower.contains("access denied")
            || msg_lower.contains("not authorized")
        {
            (FileErrorType::Permission, false)
        } else if msg_lower.contains("not found")
            || msg_lower.contains("no such file")
            || msg_lower.contains("does not exist")
        {
            (FileErrorType::NotFound, false)
        } else if msg_lower.contains("timeout")
            || msg_lower.contains("timed out")
            || msg_lower.contains("time out")
            || msg_lower.contains("wait socket")
        {
            (FileErrorType::Timeout, true)
        } else if msg_lower.contains("connection reset")
            || msg_lower.contains("connection lost")
            || msg_lower.contains("network")
        {
            (FileErrorType::Network, true)
        } else if msg_lower.contains("session") || msg_lower.contains("disconnected") {
            (FileErrorType::Session, true)
        } else {
            (FileErrorType::Unknown, false)
        };

        Self {
            error_type,
            message: msg.clone(),
            retryable,
            original_error: Some(msg),
        }
    }

    /// 创建会话错误
    pub fn session_not_found() -> Self {
        Self {
            error_type: FileErrorType::Session,
            message: "Session not found or disconnected".to_string(),
            retryable: false,
            original_error: None,
        }
    }
}

impl From<String> for FileOperationError {
    fn from(msg: String) -> Self {
        Self::from_message(msg)
    }
}

impl From<&str> for FileOperationError {
    fn from(msg: &str) -> Self {
        Self::from_message(msg.to_string())
    }
}

impl std::fmt::Display for FileOperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FileOperationError {}

/// Server status information for the status bar
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub cpu_usage: Option<f32>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub uptime: Option<u64>,
    pub load_average: Option<String>,
}

/// Disk usage information for a specific path
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskUsage {
    pub path: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f32,
}
