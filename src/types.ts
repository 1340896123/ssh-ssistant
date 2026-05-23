export interface SshKey {
  id: number;
  name: string;
  content: string;
  passphrase?: string;
  createdAt: number;
}

export type HostPlatform = "Linux" | "Windows" | "macOS";
export type AssetCriticality = "low" | "medium" | "high" | "critical";
export type AccessAuthType = "password" | "key";
export type CredentialKind = "password" | "sshKey" | "token";

export interface HostAsset {
  id?: number;
  name: string;
  host: string;
  port: number;
  platform?: HostPlatform;
  folderId?: number | null;
  envId?: number | null;
  labels?: string[];
  owner?: string;
  criticality?: AssetCriticality;
  defaultWorkspacePath?: string;
  accessEndpointId?: number | null;
  bastionChainId?: string | null;
  healthSummary?: string | null;
  lastAccessedAt?: number | null;
  isFavorite?: boolean;
  groupId?: number | null;
}

export interface Connection {
  id?: number;
  name: string;
  host: string;
  port: number;
  username: string;
  password?: string;
  authType?: AccessAuthType;
  sshKeyId?: number | null;
  jumpHost?: string;
  jumpPort?: number;
  jumpUsername?: string;
  jumpPassword?: string;
  groupId?: number | null;
  osType?: HostPlatform;
  keyContent?: string | null;
  keyPassphrase?: string | null;
  platform?: HostPlatform;
  folderId?: number | null;
  envId?: number | null;
  labels?: string[];
  owner?: string;
  criticality?: AssetCriticality;
  defaultWorkspacePath?: string;
  accessEndpointId?: number | null;
  bastionChainId?: string | null;
  healthSummary?: string | null;
  lastAccessedAt?: number | null;
  isFavorite?: boolean;
}

export type ConnectionHistoryStatus = "success" | "failed";

export type ConnectionHistorySource = "quick" | "history" | "tree" | "search";

export interface ConnectionHistoryEntry {
  connectionId: number;
  connectedAt: number;
  status: ConnectionHistoryStatus;
  reason?: string;
  source: ConnectionHistorySource;
}

export interface AssetAccessHistoryEntry {
  assetId: number;
  connectedAt: number;
  status: ConnectionHistoryStatus;
  reason?: string;
  source: ConnectionHistorySource;
}

export interface AssetFolder {
  id?: number;
  name: string;
  parentId?: number | null;
  color?: string | null;
  children?: (AssetFolder | HostAsset)[];
}

export type ConnectionGroup = AssetFolder;

export interface Environment {
  id?: number;
  name: string;
  slug: string;
  color?: string | null;
  description?: string | null;
}

export interface AssetTag {
  id?: number;
  name: string;
  color?: string | null;
}

export interface AccessEndpoint {
  id?: number;
  assetId: number;
  name: string;
  host: string;
  port: number;
  username: string;
  authType?: AccessAuthType;
  credentialRefId?: number | null;
  sshKeyId?: number | null;
  jumpHost?: string | null;
  jumpPort?: number | null;
  jumpUsername?: string | null;
  jumpPassword?: string | null;
}

export interface CredentialRef {
  id?: number;
  name: string;
  credentialKind: CredentialKind;
  username?: string | null;
  secret?: string | null;
  sshKeyId?: number | null;
  assetId?: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface AssetUpsertPayload {
  asset: HostAsset;
  defaultAccessEndpoint: AccessEndpoint;
  defaultCredentialRef?: CredentialRef | null;
}

export interface AssetSessionConnectResult {
  sessionId: string;
  assetId: number;
  assetName: string;
  createdAt: number;
  envId?: number | null;
  accessEndpointId?: number | null;
  credentialRefId?: number | null;
  bastionChainId?: string | null;
  riskLevel: AssetCriticality;
  healthSummary?: string | null;
  osInfo: string;
}

export interface SavedAssetView {
  id?: number;
  name: string;
  queryJson: string;
  createdAt: number;
  updatedAt: number;
}

export interface JobTemplate {
  id?: number;
  name: string;
  command: string;
  scopeType: string;
  scopeValue?: string | null;
  riskLevel: AssetCriticality;
  requiresConfirmation: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface JobRun {
  id?: number;
  assetId?: number | null;
  sessionId?: string | null;
  templateId?: number | null;
  command: string;
  status: string;
  output?: string | null;
  riskLevel: AssetCriticality;
  initiatedBy?: string | null;
  source?: string | null;
  createdAt: number;
  completedAt?: number | null;
}

export interface AuditEvent {
  id?: number;
  eventType: string;
  assetId?: number | null;
  sessionId?: string | null;
  jobRunId?: number | null;
  title: string;
  detail?: string | null;
  severity: "info" | "warning" | "error";
  metadataJson?: string | null;
  createdAt: number;
}

export interface SyncState {
  id?: number;
  stateKey: string;
  status: string;
  version: number;
  endpointUrl?: string | null;
  lastSyncedAt?: number | null;
  lastError?: string | null;
  metadataJson?: string | null;
  updatedAt: number;
}

export type TunnelType = "local" | "remote" | "dynamic";

export interface Tunnel {
  id?: number;
  connectionId: number;
  name: string;
  tunnelType: TunnelType;
  localHost?: string;
  localPort?: number;
  remoteHost?: string;
  remotePort?: number;
  remoteBindHost?: string;
  proxyJump?: string;
  proxyCommand?: string;
  agentForwarding?: boolean;
  createdAt?: number;
}

export interface TunnelStatus {
  id: number;
  active: boolean;
  pid?: number;
}

export interface FileEntry {
  name: string;
  isDir: boolean;
  size: number;
  mtime: number;
  permissions: number;
  uid: number;
  owner: string;
}

export interface FilePageResponse {
  entries: FileEntry[];
  nextCursor: number | null;
  hasMore: boolean;
}

export type ColumnKey = "name" | "size" | "date" | "owner";

export type AIProviderType = "openai" | "anthropic";

export interface AIConfig {
  apiUrl: string;
  apiKey: string;
  modelName: string;
  providerType: AIProviderType;
}

export type TerminalCursorStyle = "block" | "underline" | "bar";

export interface TerminalAppearanceSettings {
  fontSize: number;
  fontFamily: string;
  cursorStyle: TerminalCursorStyle;
  lineHeight: number;
}

export type FileManagerViewMode = "flat" | "tree";
export type FileManagerLayout = "left" | "bottom";

export interface FileManagerSettings {
  viewMode: FileManagerViewMode;
  layout: FileManagerLayout;
  sftpBufferSize: number;
}

export interface SshPoolSettings {
  maxBackgroundSessions: number;
  enableAutoCleanup: boolean;
  cleanupIntervalMinutes: number;
}

export interface ConnectionTimeoutSettings {
  connectionTimeoutSecs: number;
  jumpHostTimeoutSecs: number;
  localForwardTimeoutSecs: number;
  commandTimeoutSecs: number;
  sftpOperationTimeoutSecs: number;
}

export interface ReconnectSettings {
  maxReconnectAttempts: number;
  initialDelayMs: number;
  maxDelayMs: number;
  backoffMultiplier: number;
  enableAutoReconnect: boolean;
}

export interface HeartbeatSettings {
  tcpKeepaliveIntervalSecs: number;
  sshKeepaliveIntervalSecs: number;
  appHeartbeatIntervalSecs: number;
  heartbeatTimeoutSecs: number;
  failedHeartbeatsBeforeAction: number;
}

export interface PoolHealthSettings {
  healthCheckIntervalSecs: number;
  sessionWarmupCount: number;
  maxSessionAgeMinutes: number;
  unhealthyThreshold: number;
}

export type NetworkQuality = "Excellent" | "Good" | "Fair" | "Poor" | "Unknown";

export interface NetworkAdaptiveSettings {
  enableAdaptive: boolean;
  latencyCheckIntervalSecs: number;
  highLatencyThresholdMs: number;
  lowBandwidthThresholdKbps: number;
}

export interface NetworkStatus {
  latencyMs: number;
  bandwidthKbps?: number;
  quality: NetworkQuality;
  lastUpdate: number;
}

export interface AdaptiveParams {
  heartbeatIntervalSecs: number;
  sftpBufferSize: number;
  commandTimeoutSecs: number;
  keepaliveIntervalSecs: number;
}

export interface Settings {
  theme: "light" | "dark";
  language: "en" | "zh";
  ai: AIConfig;
  terminalAppearance: TerminalAppearanceSettings;
  fileManager: FileManagerSettings;
  sshPool: SshPoolSettings;
  connectionTimeout: ConnectionTimeoutSettings;
  reconnect: ReconnectSettings;
  heartbeat: HeartbeatSettings;
  poolHealth: PoolHealthSettings;
  networkAdaptive: NetworkAdaptiveSettings;
}

export interface Workspace {
  path: string;
  name: string;
  context: string;
  fileTree: string;
  isIndexed: boolean;
}

export interface OpsSession {
  id: string;
  assetId: number;
  assetName: string;
  createdAt: number;
  accessEndpointId?: number | null;
  credentialRefId?: number | null;
  bastionChainId?: string | null;
  currentPath: string;
  riskLevel: AssetCriticality;
  healthSummary?: string | null;
  lastJobRunId?: number | null;
  status: "connected" | "disconnected" | "connecting";
  activeTab: "terminal" | "files" | "ai";
  files: FileEntry[];
  connectedAt: number;
  activeWorkspace?: Workspace;
  os?: string;
  envId?: number | null;
}

export type Session = OpsSession;

export type ConnectionStatus =
  | "connecting"
  | "connected"
  | "authenticating"
  | "ready"
  | "degraded"
  | "reconnecting"
  | "disconnected"
  | "error";

export interface ConnectionMetrics {
  uptimeSecs: number;
  bytesSent: number;
  bytesReceived: number;
  latencyMs: number;
  reconnectCount: number;
  lastError?: string;
}

export interface ConnectionStatusEvent {
  sessionId: string;
  status: ConnectionStatus;
  timestamp: number;
  details?: string;
  metrics?: ConnectionMetrics;
}

export interface ReconnectEvent {
  sessionId: string;
  attempt: number;
  maxAttempts: number;
  delayMs: number;
}
