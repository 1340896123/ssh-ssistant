import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { AiEndpointRecord } from '../types';

export const useAiEndpointsStore = defineStore('aiEndpoints', {
  state: () => ({
    endpoints: [] as AiEndpointRecord[],
    loaded: false,
  }),
  getters: {
    defaultEndpoint: (state): AiEndpointRecord | undefined =>
      state.endpoints.find((endpoint) => endpoint.isDefault),
  },
  actions: {
    async loadEndpoints(force = false) {
      if (this.loaded && !force) return;
      try {
        this.endpoints = await invoke<AiEndpointRecord[]>('get_ai_endpoints');
        this.loaded = true;
      } catch (e) {
        console.error('Failed to load AI endpoints:', e);
      }
    },
    async createEndpoint(endpoint: AiEndpointRecord) {
      const id = await invoke<number>('create_ai_endpoint', { endpoint });
      await this.loadEndpoints(true);
      return id;
    },
    async updateEndpoint(endpoint: AiEndpointRecord) {
      await invoke('update_ai_endpoint', { endpoint });
      await this.loadEndpoints(true);
    },
    async deleteEndpoint(id: number) {
      await invoke('delete_ai_endpoint', { id });
      await this.loadEndpoints(true);
    },
    async setDefault(id: number) {
      await invoke('set_default_ai_endpoint', { id });
      await this.loadEndpoints(true);
    },
  },
});
