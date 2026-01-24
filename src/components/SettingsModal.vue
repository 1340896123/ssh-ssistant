<script setup lang="ts">
import { ref, watch } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useSshKeyStore } from '../stores/sshKeys';
import { useI18n } from '../composables/useI18n';
import { X, Plus, Trash2, Key } from 'lucide-vue-next';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits(['close']);
const store = useSettingsStore();
const sshKeyStore = useSshKeyStore();
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

// SSH Key Management State
const showAddKeyForm = ref(false);
const newKey = ref({
  name: '',
  content: '',
  passphrase: ''
});

const keyInputMode = ref<'import' | 'generate'>('import');
const isGenerating = ref(false);
const genKey = ref({
  name: '',
  algorithm: 'ed25519',
  passphrase: ''
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
    sshKeyStore.loadKeys();
    showAddKeyForm.value = false;
    newKey.value = { name: '', content: '', passphrase: '' };
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

async function addKey() {
  if (!newKey.value.name || !newKey.value.content) return;
  const success = await sshKeyStore.addKey({
    name: newKey.value.name,
    content: newKey.value.content,
    passphrase: newKey.value.passphrase || undefined
  });
  if (success) {
    showAddKeyForm.value = false;
    newKey.value = { name: '', content: '', passphrase: '' };
  }
}

async function generateKey() {
  if (!genKey.value.name) return;
  isGenerating.value = true;
  try {
    await sshKeyStore.generateKey(
      genKey.value.name,
      genKey.value.algorithm,
      genKey.value.passphrase || undefined
    );
    showAddKeyForm.value = false;
    genKey.value = { name: '', algorithm: 'ed25519', passphrase: '' };
  } finally {
    isGenerating.value = false;
  }
}

async function deleteKey(id: number) {
  if (confirm('Are you sure you want to delete this SSH key?')) {
    await sshKeyStore.deleteKey(id);
  }
}

function formatDate(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

const tabs = [
  { id: 'general', label: 'settings.general' },
  { id: 'ai', label: 'settings.aiAssistant' },
  { id: 'terminal', label: 'settings.terminalAppearance' },
  { id: 'fileManager', label: 'settings.fileManagement' },
  { id: 'sshPool', label: 'settings.sshPool' },
  { id: 'sshKeys', label: 'SSH Keys' }, // TODO: Add i18n
];

</script>

<template>
  <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="bg-gray-800 rounded-lg shadow-xl w-[700px] border border-gray-700 flex flex-col max-h-[85vh]">
      <div class="flex items-center justify-between p-4 border-b border-gray-700">
        <h2 class="text-lg font-semibold text-white">{{ t('settings.title') }}</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-white">
          <X class="w-5 h-5" />
        </button>
      </div>

      <div class="flex-grow flex flex-col overflow-hidden">
        <div class="border-b border-gray-700 py-2">
          <nav class="flex space-x-2 px-4 overflow-x-auto no-scrollbar" aria-label="Tabs">
            <button v-for="tab in tabs" :key="tab.id" @click="activeTab = tab.id" :class="[
              'px-3 py-2 text-sm font-medium whitespace-nowrap rounded transition-colors',
              activeTab === tab.id
                ? 'bg-blue-600 text-white'
                : 'text-gray-400 hover:bg-gray-700 hover:text-white'
            ]">
              {{ activeTab === 'sshKeys' && tab.id === 'sshKeys' ? 'SSH Keys' : t(tab.label) }}
            </button>
          </nav>
        </div>

        <div class="p-6 overflow-y-auto custom-scrollbar">
          <!-- General Tab -->
          <div v-if="activeTab === 'general'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-white mb-4">{{ t('settings.appearance') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.theme') }}</label>
                  <select v-model="form.theme"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="dark">{{ t('themes.dark') }}</option>
                    <option value="light">{{ t('themes.light') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.language') }}</label>
                  <select v-model="form.language"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
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
                  <button @click="clearCache"
                    class="px-4 py-2 text-sm bg-red-600 hover:bg-red-500 text-white rounded transition-colors">
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
                  <input v-model="form.ai.apiUrl" type="text"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none"
                    placeholder="https://api.openai.com/v1" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.apiKey') }}</label>
                  <input v-model="form.ai.apiKey" type="password"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none"
                    placeholder="sk-..." />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.modelName') }}</label>
                  <input v-model="form.ai.modelName" type="text"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none"
                    placeholder="gpt-3.5-turbo" />
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
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalFontSize')
                  }}</label>
                  <input v-model.number="form.terminalAppearance.fontSize" type="number" min="8" max="32"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalFontFamily')
                  }}</label>
                  <input v-model="form.terminalAppearance.fontFamily" type="text"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalCursorStyle')
                  }}</label>
                  <select v-model="form.terminalAppearance.cursorStyle"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="block">{{ t('terminal.cursor.block') }}</option>
                    <option value="underline">{{ t('terminal.cursor.underline') }}</option>
                    <option value="bar">{{ t('terminal.cursor.bar') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.terminalLineHeight')
                  }}</label>
                  <input v-model.number="form.terminalAppearance.lineHeight" type="number" step="0.1" min="0.8" max="2"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
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
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.fileManagerViewMode')
                  }}</label>
                  <select v-model="form.fileManager.viewMode"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="flat">{{ t('fileManager.viewMode.flat') }}</option>
                    <option value="tree">{{ t('fileManager.viewMode.tree') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">Layout Position</label>
                  <select v-model="form.fileManager.layout"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                    <option value="bottom">Bottom (Below Terminal)</option>
                    <option value="left">Left (Side by Side)</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">SFTP Buffer Size (KB)</label>
                  <input v-model.number="form.fileManager.sftpBufferSize" type="number" min="64" max="1024" step="64"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
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
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.maxBackgroundSessions')
                  }}</label>
                  <input v-model.number="form.sshPool.maxBackgroundSessions" type="number" min="1" max="10"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                  <p class="text-xs text-gray-400 mt-1">{{ t('settings.maxBackgroundSessionsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.enableAutoCleanup')
                  }}</label>
                  <div class="flex items-center">
                    <input v-model="form.sshPool.enableAutoCleanup" type="checkbox"
                      class="bg-gray-700 border-gray-600 rounded text-blue-600 focus:ring-blue-500 focus:ring-offset-gray-800 focus:ring-offset-0" />
                    <span class="ml-2 text-sm text-gray-300">{{ t('settings.enableAutoCleanupDesc') }}</span>
                  </div>
                </div>
                <div v-if="form.sshPool.enableAutoCleanup">
                  <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.cleanupIntervalMinutes')
                  }}</label>
                  <input v-model.number="form.sshPool.cleanupIntervalMinutes" type="number" min="1" max="60"
                    class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" />
                  <p class="text-xs text-gray-400 mt-1">{{ t('settings.cleanupIntervalMinutesDesc') }}</p>
                </div>
              </div>
            </section>
          </div>

          <!-- SSH Keys Tab -->
          <div v-if="activeTab === 'sshKeys'" class="space-y-6">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-lg font-semibold text-white">SSH Keys</h3>
              <button @click="showAddKeyForm = true"
                class="flex items-center gap-2 px-3 py-1.5 bg-blue-600 hover:bg-blue-500 text-white rounded text-sm">
                <Plus class="w-4 h-4" /> Add Key
              </button>
            </div>

            <div v-if="showAddKeyForm" class="bg-gray-700 p-4 rounded mb-6 border border-gray-600">
              <div class="flex gap-4 border-b border-gray-600 mb-4 pb-2">
                <button @click="keyInputMode = 'import'" :class="[
                  'text-sm font-medium pb-1 transition-colors',
                  keyInputMode === 'import' ? 'text-blue-400 border-b-2 border-blue-400' : 'text-gray-400 hover:text-white'
                ]">Import Existing Key</button>
                <button @click="keyInputMode = 'generate'" :class="[
                  'text-sm font-medium pb-1 transition-colors',
                  keyInputMode === 'generate' ? 'text-blue-400 border-b-2 border-blue-400' : 'text-gray-400 hover:text-white'
                ]">Generate New Key</button>
              </div>

              <!-- Import Mode -->
              <div v-if="keyInputMode === 'import'" class="space-y-3">
                <div>
                  <label class="block text-xs uppercase text-gray-400 mb-1">Key Name</label>
                  <input v-model="newKey.name"
                    class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-white focus:border-blue-500 outline-none"
                    placeholder="My Private Key" />
                </div>
                <div>
                  <label class="block text-xs uppercase text-gray-400 mb-1">Private Key Content</label>
                  <textarea v-model="newKey.content" rows="4"
                    class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-white focus:border-blue-500 outline-none font-mono text-xs"
                    placeholder="-----BEGIN OPENSSH PRIVATE KEY-----..." />
                </div>
                <div>
                  <label class="block text-xs uppercase text-gray-400 mb-1">Passphrase (Optional)</label>
                  <input v-model="newKey.passphrase" type="password"
                    class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-white focus:border-blue-500 outline-none"
                    placeholder="Key Passphrase" />
                </div>
                <div class="flex justify-end gap-2 mt-2">
                  <button @click="showAddKeyForm = false"
                    class="px-3 py-1.5 text-sm text-gray-300 hover:text-white">Cancel</button>
                  <button @click="addKey"
                    class="px-3 py-1.5 text-sm bg-green-600 hover:bg-green-500 text-white rounded">Import Key</button>
                </div>
              </div>

              <!-- Generate Mode -->
              <div v-if="keyInputMode === 'generate'" class="space-y-3">
                <div>
                  <label class="block text-xs uppercase text-gray-400 mb-1">Key Name</label>
                  <input v-model="genKey.name"
                    class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-white focus:border-blue-500 outline-none"
                    placeholder="id_ed25519" />
                </div>
                <div>
                  <label class="block text-xs uppercase text-gray-400 mb-1">Algorithm</label>
                  <select v-model="genKey.algorithm"
                    class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-white focus:border-blue-500 outline-none">
                    <option value="ed25519">Ed25519 (Recommended)</option>
                    <option value="rsa">RSA (3072-bit)</option>
                  </select>
                </div>
                <div>
                  <label class="block text-xs uppercase text-gray-400 mb-1">Passphrase (Optional)</label>
                  <input v-model="genKey.passphrase" type="password"
                    class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-white focus:border-blue-500 outline-none"
                    placeholder="Secure Passphrase" />
                </div>
                <div class="flex justify-end gap-2 mt-2">
                  <button @click="showAddKeyForm = false"
                    class="px-3 py-1.5 text-sm text-gray-300 hover:text-white">Cancel</button>
                  <button @click="generateKey" :disabled="isGenerating"
                    class="px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded disabled:opacity-50 flex items-center gap-2">
                    <div v-if="isGenerating"
                      class="w-3 h-3 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                    Generate & Save
                  </button>
                </div>
              </div>
            </div>

            <div class="space-y-2">
              <div v-if="sshKeyStore.keys.length === 0" class="text-gray-400 text-center py-8">
                No SSH keys found. Add one to get started.
              </div>
              <div v-else v-for="key in sshKeyStore.keys" :key="key.id"
                class="flex items-center justify-between p-3 bg-gray-700/50 rounded border border-gray-700 hover:border-gray-600">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded bg-gray-600 flex items-center justify-center text-gray-300">
                    <Key class="w-4 h-4" />
                  </div>
                  <div>
                    <div class="font-medium text-white">{{ key.name }}</div>
                    <div class="text-xs text-gray-400">Created: {{ formatDate(key.createdAt) }}</div>
                  </div>
                </div>
                <button @click="deleteKey(key.id)"
                  class="p-2 text-gray-400 hover:text-red-400 hover:bg-gray-600 rounded">
                  <Trash2 class="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>

        </div>
      </div>

      <div class="p-4 border-t border-gray-700 flex justify-end space-x-3">
        <button @click="$emit('close')"
          class="px-4 py-2 text-sm text-gray-300 hover:text-white hover:bg-gray-700 rounded">{{ t('settings.cancel')
          }}</button>
        <button @click="save" class="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded">{{
          t('settings.saveChanges') }}</button>
      </div>
    </div>
  </div>
</template>
