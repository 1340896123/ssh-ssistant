import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import { useNotificationStore } from './notifications';
import type { Session, Connection, Workspace } from '../types';

export const useSessionStore = defineStore('sessions', {
  state: () => ({
    sessions: [] as Session[],
    activeSessionId: null as string | null,
  }),
  getters: {
    activeSession: (state) => state.sessions.find(s => s.id === state.activeSessionId),
  },
  actions: {
    async setSessionWorkspace(sessionId: string, path: string) {
      const session = this.sessions.find(s => s.id === sessionId);
      if (!session) return;

      const name = path.split('/').pop() || 'workspace';
      
      // Initialize workspace state
      session.activeWorkspace = {
        path,
        name,
        context: 'Indexing...',
        fileTree: '',
        isIndexed: false,
      };

      try {
        // 1. Generate file tree (Limit depth to 2 to avoid huge output)
        // Try 'tree' first, then fallback to 'find'
        let treeOutput = '';
        try {
            // Check if tree exists
            const hasTree = await invoke<string>('exec_command', { id: sessionId, command: 'which tree' });
            if (hasTree && !hasTree.includes('no tree')) {
                 treeOutput = await invoke<string>('exec_command', { 
                    id: sessionId, 
                    command: `cd '${path}' && tree -L 2 --noreport` 
                });
            } else {
                 throw new Error('no tree');
            }
        } catch {
            // Fallback to find
            try {
                 // Find directories and files, max depth 2, exclude hidden files
                 const findCmd = `cd '${path}' && find . -maxdepth 2 -not -path '*/.*'`;
                 treeOutput = await invoke<string>('exec_command', { id: sessionId, command: findCmd });
            } catch (e) {
                treeOutput = "(Unable to list files: " + e + ")";
            }
        }

        // 2. Read key config files
        // List of common config files to check
        const configFiles = ['package.json', 'Cargo.toml', 'requirements.txt', 'docker-compose.yml', 'README.md', 'nginx.conf'];
        let contextSummary = "Key Configuration Files:\n";

        for (const file of configFiles) {
            try {
                // Check if file exists
                const checkCmd = `cd '${path}' && test -f ${file} && echo "yes"`;
                const exists = await invoke<string>('exec_command', { id: sessionId, command: checkCmd });
                
                if (exists && exists.trim() === 'yes') {
                     const content = await invoke<string>('read_remote_file', { 
                        id: sessionId, 
                        path: `${path.replace(/\/$/, '')}/${file}`, 
                        maxBytes: 2048 // Read first 2KB
                    });
                    contextSummary += `\n--- ${file} ---\n${content.substring(0, 1000)}${content.length > 1000 ? '\n...(truncated)' : ''}\n`;
                }
            } catch {
                // Ignore errors for individual files
            }
        }

        // 3. Check Git Status
        try {
             const gitStatus = await invoke<string>('exec_command', { 
                id: sessionId, 
                command: `cd '${path}' && git status -s | head -n 10` 
            });
            if (gitStatus && !gitStatus.includes('not a git repository')) {
                contextSummary += `\n--- Git Status ---\n${gitStatus}`;
            }
        } catch {}

        // Update workspace
        if (session.activeWorkspace) {
            session.activeWorkspace.fileTree = treeOutput;
            session.activeWorkspace.context = contextSummary;
            session.activeWorkspace.isIndexed = true;
        }

      } catch (e) {
        console.error("Failed to index workspace", e);
        if (session.activeWorkspace) {
            session.activeWorkspace.context = `Failed to index: ${e}`;
        }
      }
    },

    async createSession(conn: Connection) {
      try {
        const id = await invoke<string>('connect', { config: conn });
        const session: Session = {
          id,
          connectionId: conn.id!,
          connectionName: conn.name,
          status: 'connected',
          activeTab: 'terminal',
          currentPath: '.',
          files: [],
          connectedAt: Date.now(),
        };
        this.sessions.push(session);
        this.activeSessionId = id;
      } catch (e) {
        console.error('Failed to connect', e);
        useNotificationStore().error('Failed to connect: ' + e);
      }
    },
    async closeSession(id: string) {
      try {
        await invoke('disconnect', { id });
      } catch (e) {
        console.error(e);
      }
      this.sessions = this.sessions.filter(s => s.id !== id);
      if (this.activeSessionId === id) {
        this.activeSessionId = this.sessions.length > 0 ? this.sessions[0].id : null;
      }
    },
    async disconnectSession(id: string) {
      const index = this.sessions.findIndex(s => s.id === id);
      if (index === -1) return;

      // Optimistic update - replace object to ensure reactivity
      const session = this.sessions[index];
      this.sessions[index] = { ...session, status: 'disconnected' };

      try {
        await invoke('disconnect', { id });
      } catch (e) {
        console.error('Failed to disconnect', e);
      }
    },
    async reconnectSession(id: string) {
      const session = this.sessions.find(s => s.id === id);
      if (!session) return;

      // We need the connection config. 
      // Ideally we should store it in the session or fetch it from connection store.
      // Since we only have connectionId, we need to access connectionStore.
      // But circular dependency might be an issue if we import useConnectionStore here?
      // Let's try to import it inside the action or use a getter if possible.
      // Or just pass the config? No, the UI calls this.
      
      // Dynamic import to avoid circular dependency if any
      const { useConnectionStore } = await import('./connections');
      const connectionStore = useConnectionStore();
      const conn = connectionStore.connections.find(c => c.id === session.connectionId);
      
      if (!conn) {
        useNotificationStore().error('Connection configuration not found!');
        return;
      }

      session.status = 'connecting';
      try {
        // Pass the existing session ID to reuse it
        await invoke('connect', { config: conn, id: session.id });
        session.status = 'connected';
        session.connectedAt = Date.now();
      } catch (e) {
        console.error('Failed to reconnect', e);
        session.status = 'disconnected';
        useNotificationStore().error('Failed to reconnect: ' + e);
      }
    },
    setActiveSession(id: string) {
      this.activeSessionId = id;
    },
    updateSessionStatus(id: string, status: 'connected' | 'disconnected' | 'connecting') {
      console.log('updateSessionStatus called', id, status);
      const index = this.sessions.findIndex(s => s.id === id);
      if (index !== -1) {
        const session = this.sessions[index];
        this.sessions[index] = { ...session, status };
        console.log('Session status updated to', status);
      } else {
        console.warn('Session not found for status update', id);
      }
    },
    setActiveTab(tab: 'terminal' | 'files' | 'ai') {
      const session = this.activeSession;
      if (session) {
        session.activeTab = tab;
      }
    }
  }
});
