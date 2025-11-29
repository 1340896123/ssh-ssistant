import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { Session, Connection } from '../types';

export const useSessionStore = defineStore('sessions', {
  state: () => ({
    sessions: [] as Session[],
    activeSessionId: null as string | null,
  }),
  getters: {
    activeSession: (state) => state.sessions.find(s => s.id === state.activeSessionId),
  },
  actions: {
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
        alert('Failed to connect: ' + e);
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
        alert('Connection configuration not found!');
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
        alert('Failed to reconnect: ' + e);
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
