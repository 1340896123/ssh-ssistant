import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { Connection, ConnectionGroup } from '../types';

export const useConnectionStore = defineStore('connections', {
  state: () => ({
    connections: [] as Connection[],
    groups: [] as ConnectionGroup[],
  }),
  getters: {
    treeData: (state) => {
      const buildTree = (parentId: number | null): (ConnectionGroup | Connection)[] => {
        const result: (ConnectionGroup | Connection)[] = [];
        
        // Add groups
        const childGroups = state.groups.filter(g => (g.parentId ?? null) === parentId);
        childGroups.forEach(g => {
          const children = buildTree(g.id!);
          // We create a new object to avoid mutating the state directly with 'children' property if it's not there
          // But here we want to attach children for the UI
          result.push({ ...g, children });
        });

        // Add connections
        const childConns = state.connections.filter(c => (c.groupId ?? null) === parentId);
        childConns.forEach(c => result.push(c));

        return result;
      };
      return buildTree(null);
    }
  },
  actions: {
    async loadConnections() {
      try {
        const [conns, groups] = await Promise.all([
          invoke<Connection[]>('get_connections'),
          invoke<ConnectionGroup[]>('get_groups')
        ]);
        this.connections = conns;
        this.groups = groups;
        console.log('Loaded connections and groups');
      } catch (e) {
        console.error('Failed to load connections/groups:', e);
      }
    },
    async addConnection(conn: Connection): Promise<boolean> {
      console.log('Adding connection:', conn);
      try {
        await invoke('create_connection', { conn });
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
    },
    async addGroup(group: ConnectionGroup): Promise<boolean> {
      try {
        await invoke('create_group', { group });
        await this.loadConnections();
        return true;
      } catch (e) {
        console.error('Failed to add group:', e);
        return false;
      }
    },
    async updateGroup(group: ConnectionGroup): Promise<boolean> {
      try {
        await invoke('update_group', { group });
        await this.loadConnections();
        return true;
      } catch (e) {
        console.error('Failed to update group:', e);
        return false;
      }
    },
    async deleteGroup(id: number) {
      await invoke('delete_group', { id });
      await this.loadConnections();
    },
    async moveItem(type: 'connection' | 'group', id: number, targetGroupId: number | null) {
      if (type === 'connection') {
        const conn = this.connections.find(c => c.id === id);
        if (conn) {
          await this.updateConnection({ ...conn, groupId: targetGroupId });
        }
      } else {
        const group = this.groups.find(g => g.id === id);
        if (group) {
          if (targetGroupId === id) return; // Prevent self-loop
          await this.updateGroup({ ...group, parentId: targetGroupId });
        }
      }
    }
  }
});
