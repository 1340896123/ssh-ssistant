export interface Connection {
  id?: number;
  name: string;
  host: string;
  port: number;
  username: string;
  password?: string;
}

export interface FileEntry {
  name: string;
  isDir: boolean;
  size: number;
  mtime: number;
  permissions: number;
}

export interface Session {
  id: string; // UUID from backend
  connectionId: number;
  connectionName: string;
  status: 'connected' | 'disconnected' | 'connecting';
  activeTab: 'terminal' | 'files' | 'ai';
  currentPath: string;
  files: FileEntry[];
}
