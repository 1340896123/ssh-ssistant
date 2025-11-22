import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { Connection } from '../types';

export const useConnectionStore = defineStore('connections', {
  state: () => ({
    connections: [] as Connection[],
  }),
  actions: {
    async loadConnections() {
      try {
        this.connections = await invoke<Connection[]>('get_connections');
        console.log('Loaded connections:', this.connections);
      } catch (e) {
        console.error('Failed to load connections:', e);
      }
    },
    async addConnection(conn: Connection): Promise<boolean> {
      console.log('Adding connection:', conn);
      try {
        await invoke('create_connection', { conn });
        console.log('Connection added, reloading...');
        await this.loadConnections();
        return true;
      } catch (e) {
        console.error('Failed to add connection:', e);
        return false;
      }
    },
    async updateConnection(conn: Connection): Promise<boolean> {
      console.log('Updating connection:', conn);
      try {
        await invoke('update_connection', { conn });
        console.log('Connection updated, reloading...');
        await this.loadConnections();
        return true;
      } catch (e) {
        console.error('Failed to update connection:', e);
        return false;
      }
    },
    async deleteConnection(id: number) {
      await invoke('delete_connection', { id });
      await this.loadConnections();
    }
  }
});
