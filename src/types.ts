export interface Connection {
  id?: number;
  name: string;
  host: string;
  port: number;
  username: string;
  password?: string;
  // Jump host config
  jumpHost?: string;
  jumpPort?: number;
  jumpUsername?: string;
  jumpPassword?: string;
  groupId?: number | null;
  osType?: string; // Operating system type: "Linux", "Windows", "macOS", optional for backward compatibility
}

export interface ConnectionGroup {
  id?: number;
  name: string;
  parentId?: number | null;
  children?: (ConnectionGroup | Connection)[]; // For UI tree structure
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

export type ColumnKey = "name" | "size" | "date" | "owner";

export interface AIConfig {
  apiUrl: string;
  apiKey: string;
  modelName: string;
}

export type TerminalCursorStyle = "block" | "underline" | "bar";

export interface TerminalAppearanceSettings {
  fontSize: number;
  fontFamily: string;
  cursorStyle: TerminalCursorStyle;
  lineHeight: number;
}

export type FileManagerViewMode = "flat" | "tree";

export interface FileManagerSettings {
  viewMode: FileManagerViewMode;
  sftpBufferSize: number; // SFTP buffer size in KB
}

export interface SshPoolSettings {
  maxBackgroundSessions: number; // 最大后台会话数量
  enableAutoCleanup: boolean; // 是否启用自动清理
  cleanupIntervalMinutes: number; // 清理间隔（分钟）
}

export interface Settings {
  theme: "light" | "dark";
  language: "en" | "zh";
  ai: AIConfig;
  terminalAppearance: TerminalAppearanceSettings;
  fileManager: FileManagerSettings;
  sshPool: SshPoolSettings;
}

export interface Workspace {
  path: string;
  name: string;
  context: string;
  fileTree: string;
  isIndexed: boolean;
}

export interface Session {
  id: string; // UUID from backend
  connectionId: number;
  connectionName: string;
  status: "connected" | "disconnected" | "connecting";
  activeTab: "terminal" | "files" | "ai";
  currentPath: string;
  files: FileEntry[];
  connectedAt: number;
  activeWorkspace?: Workspace;
  os?: string;
}
