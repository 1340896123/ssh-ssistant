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
          files: []
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
    setActiveSession(id: string) {
      this.activeSessionId = id;
    },
    setActiveTab(tab: 'terminal' | 'files' | 'ai') {
      const session = this.activeSession;
      if (session) {
        session.activeTab = tab;
      }
    }
  }
});
