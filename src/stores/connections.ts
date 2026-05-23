import { defineStore } from 'pinia';
import type { Connection, ConnectionGroup, ConnectionHistorySource } from '../types';
import { useAssetStore } from './assets';
import { sessionService } from '../services';

export const useConnectionStore = defineStore('connections', {
  state: () => ({
    connections: [] as Connection[],
    groups: [] as ConnectionGroup[],
    favorites: [] as number[],
    history: [] as {
      connectionId: number;
      connectedAt: number;
      status: 'success' | 'failed';
      reason?: string;
      source: ConnectionHistorySource;
    }[],
  }),
  getters: {
    treeData: (state) => {
      const buildTree = (parentId: number | null): (ConnectionGroup | Connection)[] => {
        const result: (ConnectionGroup | Connection)[] = [];

        const childGroups = state.groups.filter((g) => (g.parentId ?? null) === parentId);
        childGroups.forEach((g) => {
          const children = buildTree(g.id!);
          result.push({ ...g, children });
        });

        const childConns = state.connections.filter((c) => (c.groupId ?? null) === parentId);
        childConns.forEach((c) => result.push(c));

        return result;
      };
      return buildTree(null);
    },
    favoriteConnections: (state) =>
      state.connections.filter(
        (conn) => conn.id !== undefined && state.favorites.includes(conn.id),
      ),
    historyEntries: (state) =>
      state.history
        .filter((entry) => state.connections.some((conn) => conn.id === entry.connectionId))
        .sort((a, b) => b.connectedAt - a.connectedAt),
  },
  actions: {
    async loadConnections() {
      const assetStore = useAssetStore();
      try {
        await assetStore.loadAssets();
        this.connections = assetStore.assets.map((asset) => {
          const endpoint = asset.id
            ? assetStore.defaultAccessEndpointForAsset(asset.id)
            : null;
          const credentialRef = endpoint
            ? assetStore.credentialRefById(endpoint.credentialRefId ?? null)
            : null;
          return {
            id: asset.id,
            name: asset.name,
            host: endpoint?.host ?? asset.host,
            port: endpoint?.port ?? asset.port,
            username: endpoint?.username ?? credentialRef?.username ?? 'root',
            password: undefined,
            authType: endpoint?.authType ?? 'password',
            sshKeyId: endpoint?.sshKeyId ?? credentialRef?.sshKeyId ?? null,
            jumpHost: endpoint?.jumpHost ?? undefined,
            jumpPort: endpoint?.jumpPort ?? undefined,
            jumpUsername: endpoint?.jumpUsername ?? undefined,
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
          };
        });
        this.groups = assetStore.folders.map((folder) => ({
          ...folder,
          parentId: folder.parentId ?? null,
        }));
        this.favorites = assetStore.favorites;
        this.history = assetStore.historyEntries;
      } catch (e) {
        console.error('Failed to load connections/groups:', e);
      }
    },
    async addConnection(conn: Connection): Promise<boolean> {
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
          owner: conn.owner ?? '',
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
          owner: conn.owner ?? '',
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
      await useAssetStore().deleteAsset(id);
      await this.loadConnections();
    },
    async addGroup(group: ConnectionGroup): Promise<boolean> {
      try {
        await useAssetStore().addFolder({
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
        await useAssetStore().updateFolder({
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
      await useAssetStore().deleteFolder(id);
      await this.loadConnections();
    },
    async moveItem(type: 'connection' | 'group', id: number, targetGroupId: number | null) {
      if (type === 'connection') {
        const conn = this.connections.find((c) => c.id === id);
        if (conn) {
          await this.updateConnection({ ...conn, groupId: targetGroupId });
        }
      } else {
        const group = this.groups.find((g) => g.id === id);
        if (group) {
          if (targetGroupId === id) return;
          await this.updateGroup({ ...group, parentId: targetGroupId });
        }
      }
    },
    async testConnection(conn: Connection): Promise<boolean> {
      try {
        await sessionService.testConnection(conn);
        return true;
      } catch (e) {
        console.error('Connection test failed:', e);
        throw e;
      }
    },
    toggleFavorite(connectionId: number) {
      void useAssetStore().toggleFavorite(connectionId);
      this.favorites = useAssetStore().favorites;
    },
    isFavorite(connectionId: number) {
      return useAssetStore().isFavorite(connectionId);
    },
    addSuccessfulConnection(connectionId: number, source: ConnectionHistorySource = 'tree') {
      useAssetStore().addSuccessfulConnection(connectionId, source);
      this.history = useAssetStore().historyEntries;
    },
    addFailedConnection(
      connectionId: number,
      reason?: string,
      source: ConnectionHistorySource = 'tree',
    ) {
      useAssetStore().addFailedConnection(connectionId, reason, source);
      this.history = useAssetStore().historyEntries;
    },
  },
});
