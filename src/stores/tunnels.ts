import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { Tunnel, TunnelStatus } from '../types';

export const useTunnelStore = defineStore('tunnels', {
  state: () => ({
    tunnels: [] as Tunnel[],
    activeTunnelIds: [] as number[],
    errorMessages: {} as Record<number, string>,
  }),
  getters: {
    isActive: (state) => (id: number) => state.activeTunnelIds.includes(id),
  },
  actions: {
    async loadTunnels(assetId?: number) {
      try {
        const tunnels = await invoke<Tunnel[]>('get_tunnels', { asset_id: assetId });
        this.tunnels = tunnels;
        const validIds = new Set(tunnels.map((tunnel) => tunnel.id).filter((id): id is number => typeof id === 'number'));
        this.errorMessages = Object.fromEntries(
          Object.entries(this.errorMessages).filter(([id]) => validIds.has(Number(id)))
        );
      } catch (e) {
        console.error('Failed to load tunnels:', e);
      }
    },
    async createTunnel(tunnel: Tunnel) {
      await invoke<number>('create_tunnel', { tunnel });
      await this.loadTunnels(tunnel.assetId);
    },
    async updateTunnel(tunnel: Tunnel) {
      await invoke('update_tunnel', { tunnel });
      await this.loadTunnels(tunnel.assetId);
    },
    async deleteTunnel(id: number, assetId: number) {
      await invoke('delete_tunnel', { id });
      await this.loadTunnels(assetId);
      this.activeTunnelIds = this.activeTunnelIds.filter(activeId => activeId !== id);
      delete this.errorMessages[id];
    },
    async startTunnel(id: number) {
      try {
        const status = await invoke<TunnelStatus>('start_tunnel', { id });
        delete this.errorMessages[id];
        if (status.active) {
          if (!this.activeTunnelIds.includes(id)) {
            this.activeTunnelIds.push(id);
          }
        }
      } catch (e) {
        this.errorMessages[id] = e instanceof Error ? e.message : String(e);
        throw e;
      }
    },
    async stopTunnel(id: number) {
      await invoke('stop_tunnel', { id });
      this.activeTunnelIds = this.activeTunnelIds.filter(activeId => activeId !== id);
      delete this.errorMessages[id];
    },
    async refreshActive() {
      const statuses = await invoke<TunnelStatus[]>('get_active_tunnels');
      this.activeTunnelIds = statuses.filter(s => s.active).map(s => s.id);
    }
  }
});
