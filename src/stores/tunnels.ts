import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { Tunnel, TunnelStatus } from '../types';

export const useTunnelStore = defineStore('tunnels', {
  state: () => ({
    tunnels: [] as Tunnel[],
    activeTunnelIds: [] as number[],
  }),
  getters: {
    isActive: (state) => (id: number) => state.activeTunnelIds.includes(id),
  },
  actions: {
    async loadTunnels(connectionId?: number) {
      try {
        const tunnels = await invoke<Tunnel[]>('get_tunnels', { connection_id: connectionId });
        this.tunnels = tunnels;
      } catch (e) {
        console.error('Failed to load tunnels:', e);
      }
    },
    async createTunnel(tunnel: Tunnel) {
      await invoke<number>('create_tunnel', { tunnel });
      await this.loadTunnels(tunnel.connectionId);
    },
    async updateTunnel(tunnel: Tunnel) {
      await invoke('update_tunnel', { tunnel });
      await this.loadTunnels(tunnel.connectionId);
    },
    async deleteTunnel(id: number, connectionId: number) {
      await invoke('delete_tunnel', { id });
      await this.loadTunnels(connectionId);
      this.activeTunnelIds = this.activeTunnelIds.filter(activeId => activeId !== id);
    },
    async startTunnel(id: number) {
      const status = await invoke<TunnelStatus>('start_tunnel', { id });
      if (status.active) {
        if (!this.activeTunnelIds.includes(id)) {
          this.activeTunnelIds.push(id);
        }
      }
    },
    async stopTunnel(id: number) {
      await invoke('stop_tunnel', { id });
      this.activeTunnelIds = this.activeTunnelIds.filter(activeId => activeId !== id);
    },
    async refreshActive() {
      const statuses = await invoke<TunnelStatus[]>('get_active_tunnels');
      this.activeTunnelIds = statuses.filter(s => s.active).map(s => s.id);
    }
  }
});
