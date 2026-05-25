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

export interface CloudAssetRecord {
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

export interface JobRunArchive {
  id?: number;
  jobRunId: number;
  assetId?: number | null;
  sessionId?: string | null;
  command: string;
  status: string;
  riskLevel: AssetCriticality;
  output?: string | null;
  summary?: string | null;
  archivedAt: number;
  createdAt: number;
  completedAt?: number | null;
  source?: string | null;
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

export interface OpsMatchedAsset {
  assetId: number;
  assetName: string;
  host: string;
  criticality: AssetCriticality;
  environmentName?: string | null;
  healthSummary?: string | null;
  matchReason: string;
}

export interface OpsPlanStep {
  id: string;
  title: string;
  description: string;
  command?: string | null;
  targetAssetId?: number | null;
  targetAssetName?: string | null;
  riskLevel: AssetCriticality;
  requiresConfirmation: boolean;
  runbook?: string | null;
}

export interface OpsConsoleAnswer {
  summary: string;
  matchedAssets: OpsMatchedAsset[];
  statusExplanation?: string | null;
  recommendedChecks: string[];
  planSteps: OpsPlanStep[];
  reviewChecklist: string[];
  sources: string[];
}

export interface JobBatchPreviewTarget {
  assetId: number;
  assetName: string;
  host: string;
  labels: string[];
  environmentName?: string | null;
  riskLevel: AssetCriticality;
  matchReason: string;
}

export interface JobBatchPreview {
  command: string;
  scopeType: string;
  scopeValue?: string | null;
  riskLevel: AssetCriticality;
  targetCount: number;
  targets: JobBatchPreviewTarget[];
  warnings: string[];
  requiresConfirmation: boolean;
  suggestedSessionReuse: number;
}

export interface JobBatchRequest {
  templateId?: number | null;
  commandText: string;
  scopeType: string;
  scopeValue?: string | null;
  targetAssetIds: number[];
  riskLevel?: AssetCriticality | null;
  source?: string | null;
}

export interface JobBatchResultItem {
  assetId: number;
  assetName: string;
  sessionId?: string | null;
  jobRunId?: number | null;
  status: string;
  output?: string | null;
  error?: string | null;
  riskLevel: AssetCriticality;
  usedExistingSession: boolean;
}

export interface JobBatchResult {
  total: number;
  completed: number;
  failed: number;
  startedAt: number;
  completedAt: number;
  items: JobBatchResultItem[];
  warnings: string[];
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

export interface SyncChangeLogEntry {
  id?: number;
  objectType: string;
  objectId: string;
  operation: string;
  objectVersion: number;
  summary: string;
  payloadJson?: string | null;
  syncStatus: string;
  serviceKey?: string | null;
  createdAt: number;
  syncedAt?: number | null;
}

export interface SyncServiceConfig {
  id?: number;
  serviceKey: string;
  displayName: string;
  baseUrl?: string | null;
  authMode: string;
  authToken?: string | null;
  enabled: boolean;
  metadataJson?: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface SyncObjectVersionSummary {
  objectType: string;
  count: number;
  maxVersion: number;
}

export interface SyncObjectVersionEntry {
  objectType: string;
  objectId: string;
  version: number;
  updatedAt: number;
}

export interface SyncOverview {
  state: SyncState;
  pendingChanges: number;
  totalChanges: number;
  lastChangeAt?: number | null;
  services: SyncServiceConfig[];
  recentChanges: SyncChangeLogEntry[];
  protocolVersion: string;
  strategy: string;
  objectVersionSummary: SyncObjectVersionSummary[];
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
export type AccountMode = "personal" | "enterpriseSubAccount" | "local";
export type AISubscriptionPlan =
  | "free"
  | "personal"
  | "team"
  | "enterprise"
  | "custom";
export type AISubscriptionStatus =
  | "inactive"
  | "trialing"
  | "active"
  | "pastDue"
  | "cancelled";

export type AIProviderType = "openai" | "anthropic";

export interface AccountProfile {
  mode: AccountMode;
  userId?: string | null;
  displayName?: string | null;
  email?: string | null;
  enterpriseId?: string | null;
  enterpriseName?: string | null;
  subAccountId?: string | null;
  accessToken?: string | null;
  refreshToken?: string | null;
  expiresAt?: number | null;
  refreshExpiresAt?: number | null;
}

export interface SyncPreferences {
  enabled: boolean;
  endpointUrl?: string | null;
  organizationScope?: string | null;
  syncAssets: boolean;
  syncSettings: boolean;
  lastCloudSyncAt?: number | null;
}

export interface AIEndpointConfig {
  endpointName: string;
  apiUrl: string;
  apiKey: string;
  modelName: string;
  providerType: AIProviderType;
}

export interface AISubscriptionConfig {
  plan: AISubscriptionPlan;
  status: AISubscriptionStatus;
  seats: number;
  billingScope?: "global" | "enterprise" | "personal";
  pricePerSeat?: number;
  currency?: string;
  planDisplayName?: string;
  startedAt?: number | null;
  renewalAt?: number | null;
  allowCustomEndpoint?: boolean;
  useCustomEndpoint: boolean;
  syncToCloud: boolean;
}

export interface SubscriptionInvoiceLineItem {
  id: string;
  invoiceId: string;
  itemType: string;
  description: string;
  quantity: number;
  unitPrice: number;
  amount: number;
  currency: string;
  totalTokens?: number | null;
  createdAt: string;
}

export interface SubscriptionPaymentTransaction {
  id: string;
  invoiceId: string;
  targetType: string;
  targetId: string;
  providerKey: string;
  amount: number;
  currency: string;
  paymentMethod: string;
  status: string;
  externalReference: string;
  note: string;
  checkoutUrl: string;
  expiresAt?: string | null;
  paidAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface SubscriptionPaymentProvider {
  providerKey: string;
  displayName: string;
  providerType: string;
  webhookSecret: string;
  enabled: boolean;
  metadataJson: string;
  checkoutBaseUrl: string;
  webhookMode: string;
  apiBaseUrl: string;
  secretApiKey: string;
  successUrl: string;
  cancelUrl: string;
  updatedAt: string;
}

export interface SubscriptionInvoiceSummary {
  id: string;
  targetType: string;
  targetId: string;
  planCode: string;
  status: "open" | "paid" | "overdue" | "voided";
  seatCount: number;
  unitPrice: number;
  subscriptionAmount: number;
  aiUsageAmount: number;
  totalAmount: number;
  currency: string;
  billingMonth: string;
  dueAt: string;
  createdAt: string;
  updatedAt: string;
  paidAmount: number;
  remainingAmount: number;
  lineItems: SubscriptionInvoiceLineItem[];
  payments: SubscriptionPaymentTransaction[];
}

export interface SubscriptionUsageAccountSummary {
  accountId: string;
  accountMode: string;
  requests: number;
  totalTokens: number;
  estimatedCost: number;
  currency: string;
}

export interface SubscriptionUsageSummary {
  billingMonth: string;
  totalRequests: number;
  managedRequests: number;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  estimatedCost: number;
  currency: string;
  topAccounts: SubscriptionUsageAccountSummary[];
}

export interface ClientSubscriptionSnapshot {
  subscription: AISubscriptionConfig;
  currentInvoice?: SubscriptionInvoiceSummary | null;
  recentInvoices: SubscriptionInvoiceSummary[];
  paymentProviders: SubscriptionPaymentProvider[];
  usage: SubscriptionUsageSummary;
}

export interface LocalWorkspaceAccessHistoryEntry {
  assetId: number;
  connectedAt: number;
  status: ConnectionHistoryStatus;
  reason?: string;
  source: ConnectionHistorySource;
}

export interface LocalWorkspaceSnapshot {
  settings: Settings;
  records: CloudAssetRecord[];
  folders: AssetFolder[];
  environments: Environment[];
  tags: AssetTag[];
  savedViews: SavedAssetView[];
  accessHistory: LocalWorkspaceAccessHistoryEntry[];
  jobTemplates: JobTemplate[];
  jobRuns: JobRun[];
  jobArchives: JobRunArchive[];
  auditEvents: AuditEvent[];
  syncState?: SyncState | null;
  syncObjectVersions: SyncObjectVersionEntry[];
  syncChanges: SyncChangeLogEntry[];
  syncServices: SyncServiceConfig[];
}

export interface PendingCheckoutSession {
  invoiceId: string;
  providerKey: string;
  checkoutUrl?: string | null;
  externalReference?: string | null;
  createdAt: number;
  expiresAt?: number | null;
}

export interface AIConfig {
  apiUrl: string;
  apiKey: string;
  modelName: string;
  providerType: AIProviderType;
  subscription: AISubscriptionConfig;
  customEndpoint: AIEndpointConfig;
  subscriptionSnapshot?: ClientSubscriptionSnapshot | null;
  pendingCheckoutSession?: PendingCheckoutSession | null;
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
  account: AccountProfile;
  sync: SyncPreferences;
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
