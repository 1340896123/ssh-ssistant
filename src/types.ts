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

export interface AIConfig {
  apiUrl: string;
  apiKey: string;
  modelName: string;
}

export type TerminalCursorStyle = 'block' | 'underline' | 'bar';

export interface TerminalAppearanceSettings {
  fontSize: number;
  fontFamily: string;
  cursorStyle: TerminalCursorStyle;
  lineHeight: number;
}

export interface Settings {
  theme: 'light' | 'dark';
  language: 'en' | 'zh';
  ai: AIConfig;
  terminalAppearance: TerminalAppearanceSettings;
}

export interface Session {
  id: string; // UUID from backend
  connectionId: number;
  connectionName: string;
  status: 'connected' | 'disconnected' | 'connecting';
  activeTab: 'terminal' | 'files' | 'ai';
  currentPath: string;
  files: FileEntry[];
  connectedAt: number;
}
