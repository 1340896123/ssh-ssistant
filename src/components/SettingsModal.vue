<script setup lang="ts">
import { ref, watch } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useI18n } from '../composables/useI18n';
import { X } from 'lucide-vue-next';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits(['close']);
const store = useSettingsStore();
const { t } = useI18n();

const activeTab = ref('general');

const form = ref({
  theme: store.theme,
  language: store.language,
  ai: { ...store.ai },
  terminalAppearance: { ...store.terminalAppearance },
  fileManager: { ...store.fileManager },
  sshPool: { ...store.sshPool }
});

watch(() => props.show, (val) => {
  if (val) {
    activeTab.value = 'general';
    form.value = {
      theme: store.theme,
      language: store.language,
      ai: { ...store.ai },
      terminalAppearance: { ...store.terminalAppearance },
      fileManager: { ...store.fileManager },
      sshPool: { ...store.sshPool }
    };
  }
});

function save() {
  store.saveSettings(form.value);
  emit('close');
}

function clearCache() {
  localStorage.removeItem('sidebarWidth');
  // 重置侧边栏宽度到默认值
  const defaultWidth = 256;
  localStorage.setItem('sidebarWidth', defaultWidth.toString());
  // 触发页面刷新或重新加载以应用更改
  window.location.reload();
}

const tabs = [
  { id: 'general', label: 'settings.general' },
  { id: 'ai', label: 'settings.aiAssistant' },
  { id: 'terminal', label: 'settings.terminalAppearance' },
  { id: 'fileManager', label: 'settings.fileManagement' },
  { id: 'sshPool', label: 'settings.sshPool' },
];

</script>

<template>
  <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="bg-gray-800 rounded-lg shadow-xl w-[600px] border border-gray-700 flex flex-col max-h-[80vh]">
      <div class="flex items-center justify-between p-4 border-b border-gray-700">
        <h2 class="text-lg font-semibold text-white">{{ t('settings.title') }}</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-white">
          <X class="w-5 h-5" />
        </button>
      </div>

      <div class="flex-grow flex flex-col">
        <div class="border-b border-gray-700">
          <nav class="flex space-x-4 px-4" aria-label="Tabs">
            <button
              v-for="tab in tabs"
              :key="tab.id"
              @click="activeTab = tab.id"
              :class="[
                'px-3 py-3 text-sm font-medium',
                activeTab === tab.id
                  ? 'border-b-2 border-blue-500 text-white'
                  : 'border-b-2 border-transparent text-gray-400 hover:border-gray-500 hover:text-gray-300'
              ]"
            >
              {{ t(tab.label) }}
            </button>
          </nav>
        </div>

        <div class="p-6 overflow-y-auto">
          <!-- General Tab -->
          <div v-if="activeTab === 'general'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-white mb-4">{{ t('settings.appearance') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.theme') }}</label>
                  <select v-model="form.theme" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="dark">{{ t('themes.dark') }}</option>
                    <option value="light">{{ t('themes.light') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.language') }}</label>
                  <select v-model="form.language" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="en">{{ t('languages.en') }}</option>
                    <option value="zh">{{ t('languages.zh') }}</option>
                  </select>
                </div>
              </div>
            </section>
            
            <section>
              <h3 class="text-lg font-semibold text-white mb-4">{{ t('settings.cacheManagement') }}</h3>
              <div class="space-y-4">
                <div>
                  <p class="text-sm text-gray-400 mb-2">{{ t('settings.clearCacheDesc') }}</p>
                  <button @click="clearCache" class="px-4 py-2 text-sm bg-red-600 hover:bg-red-500 text-white rounded transition-colors">
                    {{ t('settings.clearCache') }}
                  </button>
                </div>
              </div>
            </section>
          </div>

          <!-- AI Tab -->
          <div v-if="activeTab === 'ai'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-white mb-4">{{ t('settings.aiAssistant') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.apiUrl') }}</label>
                  <input v-model="form.ai.apiUrl" type="text" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" placeholder="https://api.openai.com/v1" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.apiKey') }}</label>
                  <input v-model="form.ai.apiKey" type="password" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" placeholder="sk-..." />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.modelName') }}</label>
                  <input v-model="form.ai.modelName" type="text" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" placeholder="gpt-3.5-turbo" />
                </div>
              </div>
            </section>
          </div>

          <!-- Terminal Tab -->
          <div v-if="activeTab === 'terminal'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-white mb-4">{{ t('settings.terminalAppearance') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalFontSize') }}</label>
                  <input v-model.number="form.terminalAppearance.fontSize" type="number" min="8" max="32" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalFontFamily') }}</label>
                  <input v-model="form.terminalAppearance.fontFamily" type="text" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalCursorStyle') }}</label>
                  <select v-model="form.terminalAppearance.cursorStyle" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="block">{{ t('terminal.cursor.block') }}</option>
                    <option value="underline">{{ t('terminal.cursor.underline') }}</option>
                    <option value="bar">{{ t('terminal.cursor.bar') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalLineHeight') }}</label>
                  <input v-model.number="form.terminalAppearance.lineHeight" type="number" step="0.1" min="0.8" max="2" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                </div>
              </div>
            </section>
          </div>
          
          <!-- File Manager Tab -->
          <div v-if="activeTab === 'fileManager'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-white mb-4">{{ t('settings.fileManagement') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.fileManagerViewMode') }}</label>
                  <select v-model="form.fileManager.viewMode" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="flat">{{ t('fileManager.viewMode.flat') }}</option>
                    <option value="tree">{{ t('fileManager.viewMode.tree') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">SFTP Buffer Size (KB)</label>
                  <input v-model.number="form.fileManager.sftpBufferSize" type="number" min="64" max="1024" step="64" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                  <p class="text-xs text-gray-400 mt-1">Buffer size for SFTP file transfers (64KB-1024KB, step 64KB)</p>
                </div>
              </div>
            </section>
          </div>
          
          <!-- SSH Pool Tab -->
          <div v-if="activeTab === 'sshPool'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-white mb-4">{{ t('settings.sshPool') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.maxBackgroundSessions') }}</label>
                  <input v-model.number="form.sshPool.maxBackgroundSessions" type="number" min="1" max="10" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                  <p class="text-xs text-gray-400 mt-1">{{ t('settings.maxBackgroundSessionsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.enableAutoCleanup') }}</label>
                  <div class="flex items-center">
                    <input v-model="form.sshPool.enableAutoCleanup" type="checkbox" class="bg-gray-700 border-gray-600 rounded text-blue-600 focus:ring-blue-500 focus:ring-offset-gray-800 focus:ring-offset-0" />
                    <span class="ml-2 text-sm text-gray-300">{{ t('settings.enableAutoCleanupDesc') }}</span>
                  </div>
                </div>
                <div v-if="form.sshPool.enableAutoCleanup">
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.cleanupIntervalMinutes') }}</label>
                  <input v-model.number="form.sshPool.cleanupIntervalMinutes" type="number" min="1" max="60" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                  <p class="text-xs text-gray-400 mt-1">{{ t('settings.cleanupIntervalMinutesDesc') }}</p>
                </div>
              </div>
            </section>
          </div>

        </div>
      </div>

      <div class="p-4 border-t border-gray-700 flex justify-end space-x-3">
        <button @click="$emit('close')" class="px-4 py-2 text-sm text-gray-300 hover:text-white hover:bg-gray-700 rounded">{{ t('settings.cancel') }}</button>
        <button @click="save" class="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded">{{ t('settings.saveChanges') }}</button>
      </div>
    </div>
  </div>
</template>
