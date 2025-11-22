import { defineStore } from 'pinia';
import type { Settings } from '../types';

const STORAGE_KEY = 'ssh_app_settings';

export const useSettingsStore = defineStore('settings', {
  state: (): Settings => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      try {
        return JSON.parse(saved);
      } catch (e) {
        console.error('Failed to parse settings', e);
      }
    }
    return {
      theme: 'dark',
      language: 'zh',
      ai: {
        apiUrl: 'https://api.openai.com/v1',
        apiKey: '',
        modelName: 'gpt-3.5-turbo'
      }
    };
  },
  actions: {
    saveSettings(settings: Partial<Settings>) {
      this.$patch(settings);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.$state));
      this.applyTheme();
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
