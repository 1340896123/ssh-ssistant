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
  sshPool: { ...store.sshPool },
  connectionTimeout: { ...store.connectionTimeout },
  reconnect: { ...store.reconnect },
  heartbeat: { ...store.heartbeat },
  poolHealth: { ...store.poolHealth },
  networkAdaptive: { ...store.networkAdaptive }
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
      sshPool: { ...store.sshPool },
      connectionTimeout: { ...store.connectionTimeout },
      reconnect: { ...store.reconnect },
      heartbeat: { ...store.heartbeat },
      poolHealth: { ...store.poolHealth },
      networkAdaptive: { ...store.networkAdaptive }
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
  localStorage.removeItem('appWorkspaceLayout');
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
  if (confirm(t('settings.deleteKeyConfirm'))) {
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
  { id: 'connection', label: 'settings.connection' },
  { id: 'sshPool', label: 'settings.sshPool' },
  { id: 'sshKeys', label: 'settings.sshKeys' },
];

</script>

<template>
  <div v-if="show" class="fixed inset-0 z-modal flex items-center justify-center bg-bg-overlay backdrop-blur-sm">
    <div class="flex max-h-[85vh] w-[700px] min-h-0 min-w-0 flex-col rounded-lg border border-border-primary bg-bg-elevated">
      <div class="flex items-center justify-between p-4 border-b border-border-primary">
        <h2 class="text-lg font-semibold text-text-primary">{{ t('settings.title') }}</h2>
        <button @click="$emit('close')" class="text-text-secondary hover:text-text-primary transition-colors-fast">
          <X class="w-5 h-5" />
        </button>
      </div>

      <div class="flex min-h-0 flex-grow flex-col overflow-hidden">
        <div class="border-b border-border-primary py-2">
          <nav class="flex space-x-2 px-4 overflow-x-auto no-scrollbar" aria-label="Tabs">
            <button v-for="tab in tabs" :key="tab.id" @click="activeTab = tab.id" :class="[
              'px-3 py-2 text-sm font-medium whitespace-nowrap rounded transition-all-fast',
              activeTab === tab.id
                ? 'bg-accent text-text-primary'
                : 'text-text-secondary hover:bg-bg-elevated hover:text-text-primary'
            ]">
              {{ t(tab.label) }}
            </button>
          </nav>
        </div>

        <div class="min-h-0 overflow-y-auto p-6 custom-scrollbar">
          <!-- General Tab -->
          <div v-if="activeTab === 'general'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.appearance') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.theme') }}</label>
                  <select v-model="form.theme"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast">
                    <option value="dark">{{ t('themes.dark') }}</option>
                    <option value="light">{{ t('themes.light') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.language') }}</label>
                  <select v-model="form.language"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast">
                    <option value="en">{{ t('languages.en') }}</option>
                    <option value="zh">{{ t('languages.zh') }}</option>
                  </select>
                </div>
              </div>
            </section>

            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.cacheManagement') }}</h3>
              <div class="space-y-4">
                <div>
                  <p class="text-sm text-text-secondary mb-2">{{ t('settings.clearCacheDesc') }}</p>
                  <button @click="clearCache"
                    class="px-4 py-2 text-sm bg-error hover:bg-error/80 text-text-primary rounded transition-colors-fast">
                    {{ t('settings.clearCache') }}
                  </button>
                </div>
              </div>
            </section>
          </div>

          <!-- AI Tab -->
          <div v-if="activeTab === 'ai'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.aiAssistant') }}</h3>
              <p class="text-sm text-text-secondary mb-4">{{ t('settings.aiAssistantDesc') }}</p>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.providerType') }}</label>
                  <select v-model="form.ai.providerType"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast">
                    <option value="openai">{{ t('aiProviders.openai') }}</option>
                    <option value="anthropic">{{ t('aiProviders.anthropic') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.apiUrl') }}</label>
                  <input v-model="form.ai.apiUrl" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                    placeholder="https://api.openai.com/v1" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.apiKey') }}</label>
                  <input v-model="form.ai.apiKey" type="password"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                    placeholder="sk-..." />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.modelName') }}</label>
                  <input v-model="form.ai.modelName" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                    placeholder="gpt-3.5-turbo" />
                </div>
              </div>
            </section>
          </div>

          <!-- Terminal Tab -->
          <div v-if="activeTab === 'terminal'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.terminalAppearance') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.terminalFontSize')
                  }}</label>
                  <input v-model.number="form.terminalAppearance.fontSize" type="number" min="8" max="32"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.terminalFontFamily')
                  }}</label>
                  <input v-model="form.terminalAppearance.fontFamily" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.terminalCursorStyle')
                  }}</label>
                  <select v-model="form.terminalAppearance.cursorStyle"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast">
                    <option value="block">{{ t('terminal.cursor.block') }}</option>
                    <option value="underline">{{ t('terminal.cursor.underline') }}</option>
                    <option value="bar">{{ t('terminal.cursor.bar') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.terminalLineHeight')
                  }}</label>
                  <input v-model.number="form.terminalAppearance.lineHeight" type="number" step="0.1" min="0.8" max="2"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
              </div>
            </section>
          </div>

          <!-- File Manager Tab -->
          <div v-if="activeTab === 'fileManager'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.fileManagement') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.fileManagerViewMode')
                  }}</label>
                  <select v-model="form.fileManager.viewMode"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast">
                    <option value="flat">{{ t('fileManager.viewMode.flat') }}</option>
                    <option value="tree">{{ t('fileManager.viewMode.tree') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.sftpBufferSize') }}</label>
                  <input v-model.number="form.fileManager.sftpBufferSize" type="number" min="64" max="1024" step="64"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.sftpBufferSizeDesc') }}</p>
                </div>
              </div>
            </section>
          </div>

          <!-- Connection Tab -->
          <div v-if="activeTab === 'connection'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.connectionTimeout') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.connectionTimeoutSecs') }}</label>
                  <input v-model.number="form.connectionTimeout.connectionTimeoutSecs" type="number" min="5" max="120"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.connectionTimeoutSecsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.jumpHostTimeoutSecs') }}</label>
                  <input v-model.number="form.connectionTimeout.jumpHostTimeoutSecs" type="number" min="10" max="120"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.jumpHostTimeoutSecsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.localForwardTimeoutSecs') }}</label>
                  <input v-model.number="form.connectionTimeout.localForwardTimeoutSecs" type="number" min="5" max="60"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.localForwardTimeoutSecsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.commandTimeoutSecs') }}</label>
                  <input v-model.number="form.connectionTimeout.commandTimeoutSecs" type="number" min="10" max="300"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.commandTimeoutSecsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.sftpOperationTimeoutSecs') }}</label>
                  <input v-model.number="form.connectionTimeout.sftpOperationTimeoutSecs" type="number" min="30" max="600"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.sftpOperationTimeoutSecsDesc') }}</p>
                </div>
              </div>
            </section>

            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.reconnectTitle') }}</h3>
              <div class="space-y-4">
                <div class="flex items-center">
                  <input v-model="form.reconnect.enableAutoReconnect" type="checkbox"
                    class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                  <span class="ml-2 text-sm text-secondary">{{ t('settings.reconnectEnabled') }}</span>
                </div>
                <p class="text-xs text-text-secondary">
                  {{ t('settings.reconnectHint') }}
                </p>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.reconnectMaxAttempts') }}</label>
                  <input v-model.number="form.reconnect.maxReconnectAttempts" type="number" min="1" max="10"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.reconnectMaxAttemptsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.reconnectInitialDelay') }}</label>
                  <input v-model.number="form.reconnect.initialDelayMs" type="number" min="500" max="5000" step="100"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.reconnectInitialDelayDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.reconnectMaxDelay') }}</label>
                  <input v-model.number="form.reconnect.maxDelayMs" type="number" min="5000" max="60000" step="1000"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.reconnectMaxDelayDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.reconnectBackoffMultiplier') }}</label>
                  <input v-model.number="form.reconnect.backoffMultiplier" type="number" min="1.5" max="3.0" step="0.1"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">
                    {{ t('settings.reconnectBackoffMultiplierDesc') }}
                  </p>
                </div>
              </div>
            </section>

            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.heartbeatTitle') }}</h3>
              <div class="space-y-4">
                <p class="text-xs text-text-secondary">
                  {{ t('settings.heartbeatHint') }}
                </p>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.heartbeatTcpInterval') }}</label>
                  <input v-model.number="form.heartbeat.tcpKeepaliveIntervalSecs" type="number" min="30" max="300"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.heartbeatTcpIntervalDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.heartbeatSshInterval') }}</label>
                  <input v-model.number="form.heartbeat.sshKeepaliveIntervalSecs" type="number" min="5" max="60"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.heartbeatSshIntervalDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.heartbeatAppInterval') }}</label>
                  <input v-model.number="form.heartbeat.appHeartbeatIntervalSecs" type="number" min="10" max="120"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.heartbeatAppIntervalDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.heartbeatTimeout') }}</label>
                  <input v-model.number="form.heartbeat.heartbeatTimeoutSecs" type="number" min="2" max="30"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.heartbeatTimeoutDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.heartbeatFailedBeforeAction') }}</label>
                  <input v-model.number="form.heartbeat.failedHeartbeatsBeforeAction" type="number" min="1" max="10"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">
                    {{ t('settings.heartbeatFailedBeforeActionDesc') }}
                  </p>
                </div>
              </div>
            </section>

            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.poolHealthTitle') }}</h3>
              <div class="space-y-4">
                <p class="text-xs text-text-secondary">
                  {{ t('settings.poolHealthHint') }}
                </p>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.poolHealthInterval') }}</label>
                  <input v-model.number="form.poolHealth.healthCheckIntervalSecs" type="number" min="30" max="300"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.poolHealthIntervalDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.poolHealthWarmupCount') }}</label>
                  <input v-model.number="form.poolHealth.sessionWarmupCount" type="number" min="0" max="5"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.poolHealthWarmupCountDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.poolHealthMaxSessionAge') }}</label>
                  <input v-model.number="form.poolHealth.maxSessionAgeMinutes" type="number" min="10" max="480"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.poolHealthMaxSessionAgeDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.poolHealthUnhealthyThreshold') }}</label>
                  <input v-model.number="form.poolHealth.unhealthyThreshold" type="number" min="1" max="10"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">
                    {{ t('settings.poolHealthUnhealthyThresholdDesc') }}
                  </p>
                </div>
              </div>
            </section>

            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.networkAdaptiveTitle') }}</h3>
              <div class="space-y-4">
                <p class="text-xs text-text-secondary">
                  {{ t('settings.networkAdaptiveHint') }}
                </p>
                <div class="flex items-center">
                  <input v-model="form.networkAdaptive.enableAdaptive" type="checkbox"
                    class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                  <span class="ml-2 text-sm text-secondary">{{ t('settings.networkAdaptiveEnabled') }}</span>
                </div>
                <p class="text-xs text-text-secondary">
                  {{ t('settings.networkAdaptiveProfileHint') }}
                </p>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.networkAdaptiveLatencyInterval') }}</label>
                  <input v-model.number="form.networkAdaptive.latencyCheckIntervalSecs" type="number" min="10" max="120"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.networkAdaptiveLatencyIntervalDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.networkAdaptiveHighLatencyThreshold') }}</label>
                  <input v-model.number="form.networkAdaptive.highLatencyThresholdMs" type="number" min="100" max="1000"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.networkAdaptiveHighLatencyThresholdDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.networkAdaptiveLowBandwidthThreshold') }}</label>
                  <input v-model.number="form.networkAdaptive.lowBandwidthThresholdKbps" type="number" min="10" max="500"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.networkAdaptiveLowBandwidthThresholdDesc') }}</p>
                </div>
              </div>
            </section>
          </div>

          <!-- SSH Pool Tab -->
          <div v-if="activeTab === 'sshPool'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.sshPool') }}</h3>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.maxBackgroundSessions')
                  }}</label>
                  <input v-model.number="form.sshPool.maxBackgroundSessions" type="number" min="1" max="10"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.maxBackgroundSessionsDesc') }}</p>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.enableAutoCleanup')
                  }}</label>
                  <div class="flex items-center">
                    <input v-model="form.sshPool.enableAutoCleanup" type="checkbox"
                      class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                    <span class="ml-2 text-sm text-secondary">{{ t('settings.enableAutoCleanupDesc') }}</span>
                  </div>
                </div>
                <div v-if="form.sshPool.enableAutoCleanup">
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.cleanupIntervalMinutes')
                  }}</label>
                  <input v-model.number="form.sshPool.cleanupIntervalMinutes" type="number" min="1" max="60"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                  <p class="text-xs text-text-secondary mt-1">{{ t('settings.cleanupIntervalMinutesDesc') }}</p>
                </div>
              </div>
            </section>
          </div>

          <!-- SSH Keys Tab -->
          <div v-if="activeTab === 'sshKeys'" class="space-y-6">
            <div class="flex justify-between items-center mb-4">
              <h3 class="text-lg font-semibold text-primary">{{ t('settings.sshKeys') }}</h3>
              <button @click="showAddKeyForm = true"
                class="flex items-center gap-2 px-3 py-1.5 bg-accent hover:bg-accent/80 text-text-primary rounded text-sm">
                <Plus class="w-4 h-4" /> {{ t('settings.addKey') }}
              </button>
            </div>

            <div v-if="showAddKeyForm" class="bg-bg-elevated p-4 rounded mb-6 border border-border-primary">
              <div class="flex gap-4 border-b border-border-primary mb-4 pb-2">
                <button @click="keyInputMode = 'import'" :class="[
                  'text-sm font-medium pb-1 transition-colors-fast',
                  keyInputMode === 'import' ? 'text-accent border-b-2 border-accent' : 'text-text-secondary hover:text-text-primary'
                ]">{{ t('settings.importExistingKey') }}</button>
                <button @click="keyInputMode = 'generate'" :class="[
                  'text-sm font-medium pb-1 transition-colors-fast',
                  keyInputMode === 'generate' ? 'text-accent border-b-2 border-accent' : 'text-text-secondary hover:text-text-primary'
                ]">{{ t('settings.generateNewKey') }}</button>
              </div>

              <!-- Import Mode -->
              <div v-if="keyInputMode === 'import'" class="space-y-3">
                <div>
                  <label class="block text-xs uppercase text-text-secondary mb-1">{{ t('settings.keyName') }}</label>
                  <input v-model="newKey.name"
                    class="w-full p-2 bg-bg-tertiary border border-border-primary rounded text-text-primary focus:border-accent outline-none transition-all-fast"
                    :placeholder="t('settings.keyNamePlaceholder')" />
                </div>
                <div>
                  <label class="block text-xs uppercase text-text-secondary mb-1">{{ t('settings.privateKeyContent') }}</label>
                  <textarea v-model="newKey.content" rows="4"
                    class="w-full p-2 bg-bg-tertiary border border-border-primary rounded text-text-primary focus:border-accent outline-none transition-all-fast font-mono text-xs"
                    placeholder="-----BEGIN OPENSSH PRIVATE KEY-----..." />
                </div>
                <div>
                  <label class="block text-xs uppercase text-text-secondary mb-1">{{ t('settings.passphraseOptional') }}</label>
                  <input v-model="newKey.passphrase" type="password"
                    class="w-full p-2 bg-bg-tertiary border border-border-primary rounded text-text-primary focus:border-accent outline-none transition-all-fast"
                    :placeholder="t('settings.keyPassphrasePlaceholder')" />
                </div>
                <div class="flex justify-end gap-2 mt-2">
                  <button @click="showAddKeyForm = false"
                    class="px-3 py-1.5 text-sm text-text-secondary hover:text-text-primary">{{ t('settings.cancel') }}</button>
                  <button @click="addKey"
                    class="px-3 py-1.5 text-sm bg-success hover:bg-success/80 text-text-primary rounded">{{ t('settings.importKey') }}</button>
                </div>
              </div>

              <!-- Generate Mode -->
              <div v-if="keyInputMode === 'generate'" class="space-y-3">
                <div>
                  <label class="block text-xs uppercase text-text-secondary mb-1">{{ t('settings.keyName') }}</label>
                  <input v-model="genKey.name"
                    class="w-full p-2 bg-bg-tertiary border border-border-primary rounded text-text-primary focus:border-accent outline-none transition-all-fast"
                    placeholder="id_ed25519" />
                </div>
                <div>
                  <label class="block text-xs uppercase text-text-secondary mb-1">{{ t('settings.algorithm') }}</label>
                  <select v-model="genKey.algorithm"
                    class="w-full p-2 bg-bg-tertiary border border-border-primary rounded text-text-primary focus:border-accent outline-none transition-all-fast">
                    <option value="ed25519">{{ t('settings.algorithmEd25519') }}</option>
                    <option value="rsa">{{ t('settings.algorithmRsa') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-xs uppercase text-text-secondary mb-1">{{ t('settings.passphraseOptional') }}</label>
                  <input v-model="genKey.passphrase" type="password"
                    class="w-full p-2 bg-bg-tertiary border border-border-primary rounded text-text-primary focus:border-accent outline-none transition-all-fast"
                    :placeholder="t('settings.securePassphrasePlaceholder')" />
                </div>
                <div class="flex justify-end gap-2 mt-2">
                  <button @click="showAddKeyForm = false"
                    class="px-3 py-1.5 text-sm text-text-secondary hover:text-text-primary">{{ t('settings.cancel') }}</button>
                  <button @click="generateKey" :disabled="isGenerating"
                    class="px-3 py-1.5 text-sm bg-accent hover:bg-accent/80 text-text-primary rounded disabled:opacity-50 flex items-center gap-2">
                    <div v-if="isGenerating"
                      class="w-3 h-3 border-2 border-bg-primary border-t-transparent rounded-full animate-spin"></div>
                    {{ t('settings.generateAndSave') }}
                  </button>
                </div>
              </div>
            </div>

            <div class="space-y-2">
              <div v-if="sshKeyStore.keys.length === 0" class="text-text-secondary text-center py-8">
                {{ t('settings.noSshKeys') }}
              </div>
              <div v-else v-for="key in sshKeyStore.keys" :key="key.id"
                class="flex items-center justify-between p-3 bg-bg-elevated/50 rounded border border-border-primary hover:border-accent transition-all-fast">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded bg-bg-tertiary flex items-center justify-center text-accent border border-border-primary">
                    <Key class="w-4 h-4" />
                  </div>
                  <div>
                    <div class="font-medium text-text-primary">{{ key.name }}</div>
                    <div class="text-xs text-text-secondary">{{ t('settings.createdAt') }}: {{ formatDate(key.createdAt) }}</div>
                  </div>
                </div>
                <button @click="deleteKey(key.id)"
                  class="p-2 text-text-secondary hover:text-error hover:bg-bg-tertiary rounded transition-colors-fast">
                  <Trash2 class="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>

        </div>
      </div>

      <div class="p-4 border-t border-border-primary flex justify-end space-x-3">
        <button @click="$emit('close')"
          class="px-4 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-bg-elevated rounded">{{ t('settings.cancel')
          }}</button>
        <button @click="save" class="px-4 py-2 text-sm bg-accent hover:bg-accent/80 text-text-primary rounded">{{
          t('settings.saveChanges') }}</button>
      </div>
    </div>
  </div>
</template>
