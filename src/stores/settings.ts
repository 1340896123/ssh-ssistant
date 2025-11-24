import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { Settings } from '../types';

export const useSettingsStore = defineStore('settings', {
  state: (): Settings => ({
    theme: 'dark',
    language: 'zh',
    ai: {
      apiUrl: 'https://api.openai.com/v1',
      apiKey: '',
      modelName: 'gpt-3.5-turbo'
    }
  }),
  actions: {
    async loadSettings() {
      try {
        const settings = await invoke<Settings>('get_settings');
        this.$patch(settings);
        this.applyTheme();
      } catch (e) {
        console.error('Failed to load settings', e);
      }
    },
    async saveSettings(settings: Partial<Settings>) {
      this.$patch(settings);
      this.applyTheme();
      try {
        await invoke('save_settings', { settings: this.$state });
      } catch (e) {
        console.error('Failed to save settings', e);
      }
    },
    applyTheme() {
      if (this.theme === 'dark') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    }
  }
});
