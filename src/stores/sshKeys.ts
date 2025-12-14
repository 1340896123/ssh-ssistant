import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { SshKey } from '../types';

export const useSshKeyStore = defineStore('sshKeys', {
  state: () => ({
    keys: [] as SshKey[],
  }),
  actions: {
    async loadKeys() {
      try {
        const keys = await invoke<SshKey[]>('get_ssh_keys');
        this.keys = keys;
      } catch (e) {
        console.error('Failed to load SSH keys:', e);
      }
    },
    async addKey(key: Omit<SshKey, 'id' | 'createdAt'>): Promise<boolean> {
      try {
        await invoke('create_ssh_key', { key: { ...key, id: 0, createdAt: 0 } });
        await this.loadKeys();
        return true;
      } catch (error) {
        console.error('Failed to add SSH key:', error);
        return false;
      }
    },

    async generateKey(name: string, algorithm: string, passphrase?: string) {
      try {
        await invoke('generate_ssh_key', { name, algorithm, passphrase });
        await this.loadKeys();
      } catch (error) {
        console.error('Failed to generate SSH key:', error);
        throw error;
      }
    },

    async installKey(connectionId: number, keyId: number) {
      try {
        await invoke('install_ssh_key', { connectionId, keyId });
      } catch (error) {
        console.error('Failed to install SSH key:', error);
        throw error;
      }
    },
    async deleteKey(id: number) {
      try {
        await invoke('delete_ssh_key', { id });
        await this.loadKeys();
      } catch (e) {
        console.error('Failed to delete SSH key:', e);
      }
    }
  }
});
