import { defineStore } from 'pinia';
import type { Connection, ConnectionGroup, ConnectionHistoryEntry, ConnectionHistorySource } from '../types';
import { useAssetStore } from './assets';
import { sessionService } from '../services';

const FAVORITES_STORAGE_KEY = 'connection-favorites';
const HISTORY_STORAGE_KEY = 'connection-history';
const MAX_HISTORY_ITEMS = 40;

function canUseStorage() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function readFavorites(): number[] {
  if (!canUseStorage()) return [];

  try {
    const raw = window.localStorage.getItem(FAVORITES_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((value): value is number => typeof value === 'number')
      : [];
  } catch {
    return [];
  }
}

function readHistory(): ConnectionHistoryEntry[] {
  if (!canUseStorage()) return [];

  try {
    const raw = window.localStorage.getItem(HISTORY_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);

    return Array.isArray(parsed)
      ? parsed.filter((item): item is ConnectionHistoryEntry => {
        return typeof item?.connectionId === 'number'
          && typeof item?.connectedAt === 'number'
          && typeof item?.status === 'string'
          && typeof item?.source === 'string';
      })
      : [];
  } catch {
    return [];
  }
}

function writeFavorites(favorites: number[]) {
  if (!canUseStorage()) return;
  window.localStorage.setItem(FAVORITES_STORAGE_KEY, JSON.stringify(favorites));
}

function writeHistory(history: ConnectionHistoryEntry[]) {
  if (!canUseStorage()) return;
  window.localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(history));
}

export const useConnectionStore = defineStore('connections', {
  state: () => ({
    connections: [] as Connection[],
    groups: [] as ConnectionGroup[],
    favorites: readFavorites() as number[],
    history: readHistory() as ConnectionHistoryEntry[],
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
    },
    favoriteConnections: (state) => state.connections.filter(conn => conn.id !== undefined && state.favorites.includes(conn.id)),
    historyEntries: (state) => state.history
      .filter(entry => state.connections.some(conn => conn.id === entry.connectionId))
      .sort((a, b) => b.connectedAt - a.connectedAt),
  },
  actions: {
    async loadConnections() {
      const assetStore = useAssetStore();
      try {
        await assetStore.loadAssets();
        this.connections = assetStore.assets.map((asset) => ({
          id: asset.id,
          name: asset.name,
          host: asset.host,
          port: asset.port,
          username: asset.owner ?? 'root',
          password: undefined,
          authType: 'password',
          sshKeyId: null,
          jumpHost: undefined,
          jumpPort: undefined,
          jumpUsername: undefined,
          jumpPassword: undefined,
          groupId: asset.folderId ?? asset.groupId ?? null,
          osType: asset.platform ?? 'Linux',
          keyContent: null,
          keyPassphrase: null,
          platform: asset.platform ?? 'Linux',
          folderId: asset.folderId ?? asset.groupId ?? null,
          envId: asset.envId ?? null,
          labels: asset.labels ?? [],
          owner: asset.owner,
          criticality: asset.criticality,
          defaultWorkspacePath: asset.defaultWorkspacePath,
          accessEndpointId: asset.accessEndpointId ?? null,
          bastionChainId: asset.bastionChainId ?? null,
          healthSummary: asset.healthSummary ?? null,
          lastAccessedAt: asset.lastAccessedAt ?? null,
          isFavorite: asset.isFavorite ?? false,
        }));
        this.groups = assetStore.folders.map((folder) => ({
          ...folder,
          parentId: folder.parentId ?? null,
        }));
        this.favorites = this.favorites.filter(id => this.connections.some(conn => conn.id === id));
        this.history = this.history.filter(entry => this.connections.some(conn => conn.id === entry.connectionId));
        writeFavorites(this.favorites);
        writeHistory(this.history);
        console.log('Loaded connections and groups');
      } catch (e) {
        console.error('Failed to load connections/groups:', e);
      }
    },
    async addConnection(conn: Connection): Promise<boolean> {
      console.log('Adding connection:', conn);
      try {
        const assetStore = useAssetStore();
        await assetStore.addAsset({
          id: conn.id,
          name: conn.name,
          host: conn.host,
          port: conn.port,
          folderId: conn.groupId ?? conn.folderId ?? null,
          groupId: conn.groupId ?? conn.folderId ?? null,
          platform: conn.platform ?? conn.osType ?? 'Linux',
          envId: conn.envId ?? null,
          labels: conn.labels ?? [],
          owner: conn.owner ?? conn.username,
          criticality: conn.criticality ?? 'medium',
          defaultWorkspacePath: conn.defaultWorkspacePath,
          accessEndpointId: conn.accessEndpointId ?? null,
          bastionChainId: conn.bastionChainId ?? null,
          healthSummary: conn.healthSummary ?? null,
          lastAccessedAt: conn.lastAccessedAt ?? null,
          isFavorite: conn.isFavorite ?? false,
        });
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
        const assetStore = useAssetStore();
        await assetStore.updateAsset({
          id: conn.id,
          name: conn.name,
          host: conn.host,
          port: conn.port,
          folderId: conn.groupId ?? conn.folderId ?? null,
          groupId: conn.groupId ?? conn.folderId ?? null,
          platform: conn.platform ?? conn.osType ?? 'Linux',
          envId: conn.envId ?? null,
          labels: conn.labels ?? [],
          owner: conn.owner ?? conn.username,
          criticality: conn.criticality ?? 'medium',
          defaultWorkspacePath: conn.defaultWorkspacePath,
          accessEndpointId: conn.accessEndpointId ?? null,
          bastionChainId: conn.bastionChainId ?? null,
          healthSummary: conn.healthSummary ?? null,
          lastAccessedAt: conn.lastAccessedAt ?? null,
          isFavorite: conn.isFavorite ?? false,
        });
        await this.loadConnections();
        return true;
      } catch (e) {
        console.error('Failed to update connection:', e);
        return false;
      }
    },
    async deleteConnection(id: number) {
      const assetStore = useAssetStore();
      await assetStore.deleteAsset(id);
      this.favorites = this.favorites.filter(favoriteId => favoriteId !== id);
      this.history = this.history.filter(entry => entry.connectionId !== id);
      writeFavorites(this.favorites);
      writeHistory(this.history);
      await this.loadConnections();
    },
    async addGroup(group: ConnectionGroup): Promise<boolean> {
      try {
        const assetStore = useAssetStore();
        await assetStore.addFolder({
          ...group,
          parentId: group.parentId ?? null,
        });
        await this.loadConnections();
        return true;
      } catch (e) {
        console.error('Failed to add group:', e);
        return false;
      }
    },
    async updateGroup(group: ConnectionGroup): Promise<boolean> {
      try {
        const assetStore = useAssetStore();
        await assetStore.updateFolder({
          ...group,
          parentId: group.parentId ?? null,
        });
        await this.loadConnections();
        return true;
      } catch (e) {
        console.error('Failed to update group:', e);
        return false;
      }
    },
    async deleteGroup(id: number) {
      const assetStore = useAssetStore();
      await assetStore.deleteFolder(id);
      await this.loadConnections();
    },
    async moveItem(type: 'connection' | 'group', id: number, targetGroupId: number | null) {
      console.log('Store: moveItem called:', type, id, targetGroupId);
      if (type === 'connection') {
        const conn = this.connections.find(c => c.id === id);
        if (conn) {
          console.log('Store: Moving connection', conn.name, 'to group', targetGroupId);
          await this.updateConnection({ ...conn, groupId: targetGroupId });
        } else {
          console.error('Store: Connection not found:', id);
        }
      } else {
        const group = this.groups.find(g => g.id === id);
        if (group) {
          if (targetGroupId === id) return; // Prevent self-loop
          console.log('Store: Moving group', group.name, 'to parent', targetGroupId);
          await this.updateGroup({ ...group, parentId: targetGroupId });
        } else {
          console.error('Store: Group not found:', id);
        }
      }
    },
    async testConnection(conn: Connection): Promise<boolean> {
      try {
        await sessionService.testConnection(conn);
        return true;
      } catch (e) {
        console.error('Connection test failed:', e);
        throw e; // Re-throw to let the UI handle the error message
      }
    },
    toggleFavorite(connectionId: number) {
      const assetStore = useAssetStore();
      void assetStore.toggleFavorite(connectionId);
      if (this.favorites.includes(connectionId)) {
        this.favorites = this.favorites.filter(id => id !== connectionId);
      } else {
        this.favorites = [connectionId, ...this.favorites].slice(0, 8);
      }
      writeFavorites(this.favorites);
    },
    isFavorite(connectionId: number) {
      return useAssetStore().isFavorite(connectionId);
    },
    recordHistory(entry: ConnectionHistoryEntry) {
      this.history = [entry, ...this.history]
        .sort((a, b) => b.connectedAt - a.connectedAt)
        .slice(0, MAX_HISTORY_ITEMS);

      writeHistory(this.history);
    },
    addSuccessfulConnection(connectionId: number, source: ConnectionHistorySource = 'tree') {
      useAssetStore().addSuccessfulConnection(connectionId, source);
    },
    addFailedConnection(connectionId: number, reason?: string, source: ConnectionHistorySource = 'tree') {
      useAssetStore().addFailedConnection(connectionId, reason, source);
    }
  }
});
