<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useSshKeyStore } from '../stores/sshKeys';
import { useAssetStore } from '../stores/assets';
import { useSessionStore } from '../stores/sessions';
import { useTransferStore } from '../stores/transfers';
import type { AISubscriptionConfig, Settings } from '../types';
import { useI18n } from '../composables/useI18n';
import { X, Plus, Trash2, Key } from 'lucide-vue-next';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits(['close']);
const store = useSettingsStore();
const assetStore = useAssetStore();
const sessionStore = useSessionStore();
const transferStore = useTransferStore();
const sshKeyStore = useSshKeyStore();
const { t } = useI18n();

const activeTab = ref('general');
const cloudSecret = ref('');
const cloudStatusMessage = ref('');
const isCloudLoggingIn = ref(false);
const isCloudSyncing = ref(false);
const subscriptionSummary = ref(store.activeSubscriptionSummary());
const isCloudManagedSubscription = computed(() => store.isCloudManagedSubscription());
const subscriptionSnapshot = computed(() => store.ai.subscriptionSnapshot);
const isCreatingCheckout = ref<string | null>(null);
const isRefreshingBilling = ref(false);
const selectedCheckoutProvider = ref('manual');

function createClearedSubscription() {
  return {
    plan: 'free',
    planDisplayName: 'Free',
    status: 'inactive',
    seats: 1,
    billingScope: 'global',
    pricePerSeat: 0,
    currency: 'USD',
    startedAt: null,
    renewalAt: null,
    allowCustomEndpoint: true,
    useCustomEndpoint: true,
    syncToCloud: true,
  } satisfies AISubscriptionConfig;
}

function createClearedAiConfig(ai: Settings['ai']) {
  return {
    ...ai,
    apiUrl: 'https://api.openai.com/v1',
    apiKey: '',
    modelName: 'gpt-3.5-turbo',
    providerType: 'openai' as const,
    customEndpoint: {
      endpointName: 'Default Custom Endpoint',
      apiUrl: 'https://api.openai.com/v1',
      apiKey: '',
      modelName: 'gpt-3.5-turbo',
      providerType: 'openai' as const,
    },
    subscription: createClearedSubscription(),
    subscriptionSnapshot: null,
    pendingCheckoutSession: null,
  };
}

function buildAccountFingerprint(account: Settings['account']) {
  return [
    account.mode,
    account.email?.trim() ?? '',
    account.userId?.trim() ?? '',
    account.enterpriseId?.trim() ?? '',
    account.enterpriseName?.trim() ?? '',
    account.subAccountId?.trim() ?? '',
  ].join('|');
}

function normalizeAccountForSave(account: Settings['account']) {
  const currentFingerprint = buildAccountFingerprint(store.account);
  const nextFingerprint = buildAccountFingerprint(account);
  const shouldClearCloudState = account.mode === 'local' || currentFingerprint !== nextFingerprint;
  const nextDisplayName =
    account.mode === 'local'
      ? 'Local Workspace'
      : (account.displayName || account.email || account.userId || account.subAccountId || 'Personal Account');

  return {
    ...account,
    displayName: nextDisplayName,
    email: account.mode === 'personal' ? (account.email || account.userId || '').trim() || null : null,
    userId: account.mode === 'personal' ? (account.userId || account.email || '').trim() || null : null,
    enterpriseId: account.mode === 'enterpriseSubAccount' ? (account.enterpriseId || '').trim() || null : null,
    enterpriseName:
      account.mode === 'enterpriseSubAccount' ? (account.enterpriseName || '').trim() || null : null,
    subAccountId: account.mode === 'enterpriseSubAccount' ? (account.subAccountId || '').trim() || null : null,
    accessToken: shouldClearCloudState ? null : account.accessToken ?? null,
    refreshToken: shouldClearCloudState ? null : account.refreshToken ?? null,
    expiresAt: shouldClearCloudState ? null : account.expiresAt ?? null,
    refreshExpiresAt: shouldClearCloudState ? null : account.refreshExpiresAt ?? null,
  };
}

function formatMoney(value: number, currency = 'USD') {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
    maximumFractionDigits: 2,
  }).format(value || 0);
}

const form = ref({
  theme: store.theme,
  language: store.language,
  account: { ...store.account },
  sync: { ...store.sync },
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
      account: { ...store.account },
      sync: { ...store.sync },
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
    subscriptionSummary.value = store.activeSubscriptionSummary();
    selectedCheckoutProvider.value = store.ai.subscriptionSnapshot?.paymentProviders?.[0]?.providerKey || 'manual';
    sshKeyStore.loadKeys();
    showAddKeyForm.value = false;
    newKey.value = { name: '', content: '', passphrase: '' };
  }
});

async function save() {
  const previousMode = store.account.mode;
  const account = normalizeAccountForSave(form.value.account);
  const currentFingerprint = buildAccountFingerprint(store.account);
  const nextFingerprint = buildAccountFingerprint(account);
  const shouldClearCloudState =
    account.mode !== previousMode ||
    currentFingerprint !== nextFingerprint;

  if (shouldClearCloudState && previousMode === 'local' && account.mode !== 'local') {
    await store.saveCurrentLocalWorkspaceSnapshot().catch(() => undefined);
  }

  if (shouldClearCloudState) {
    await transferStore.cancelAllAndReset().catch(() => undefined);
    await sessionStore.disconnectAllSessions().catch(() => undefined);
    sessionStore.cleanupEventListeners();
    transferStore.clearLocalState();
  }

  const nextSettings: Partial<Settings> = {
    ...form.value,
    account,
    ai: shouldClearCloudState ? createClearedAiConfig(form.value.ai) : form.value.ai,
  };

  await store.saveSettings(nextSettings);

  if (shouldClearCloudState) {
    if (account.mode === 'local') {
      store.clearLoginGatewayRequired();
      const restored = await store.restoreSavedLocalWorkspaceSnapshot().catch(() => false);
      if (!restored) {
        await assetStore.clearWorkspace().catch(() => undefined);
      }
      emit('close');
      window.location.reload();
      return;
    }

    await assetStore.clearWorkspace().catch(() => undefined);
    store.markLoginGatewayRequired();
    emit('close');
    window.location.reload();
    return;
  }

  emit('close');
}

async function loginToCloud() {
  isCloudLoggingIn.value = true;
  cloudStatusMessage.value = '';
  const previousState = JSON.parse(JSON.stringify(store.$state)) as Settings;
  try {
    const previousMode = store.account.mode;
    if (previousMode === 'local' && form.value.account.mode !== 'local') {
      await store.saveCurrentLocalWorkspaceSnapshot().catch(() => undefined);
    }
    await store.saveSettings({
      ...form.value,
      account: {
        ...form.value.account,
        displayName:
          form.value.account.mode === 'local'
            ? 'Local Workspace'
            : form.value.account.mode === 'enterpriseSubAccount'
              ? (
                  form.value.account.displayName ||
                  form.value.account.enterpriseName ||
                  form.value.account.subAccountId ||
                  'Enterprise Sub-Account'
                )
              : (form.value.account.displayName || form.value.account.email || form.value.account.userId || 'Personal Account'),
        email:
          form.value.account.mode === 'personal'
            ? (form.value.account.email || form.value.account.userId || '').trim() || null
            : null,
        userId:
          form.value.account.mode === 'personal'
            ? (form.value.account.userId || form.value.account.email || '').trim() || null
            : null,
        enterpriseId:
          form.value.account.mode === 'enterpriseSubAccount'
            ? (form.value.account.enterpriseId || '').trim() || null
            : null,
        enterpriseName:
          form.value.account.mode === 'enterpriseSubAccount'
            ? (form.value.account.enterpriseName || '').trim() || null
            : null,
        subAccountId:
          form.value.account.mode === 'enterpriseSubAccount'
            ? (form.value.account.subAccountId || '').trim() || null
            : null,
        accessToken: null,
        refreshToken: null,
        expiresAt: null,
        refreshExpiresAt: null,
      },
      ai: createClearedAiConfig(form.value.ai),
    });
    await store.loginToCloud(cloudSecret.value);
    await transferStore.cancelAllAndReset().catch(() => undefined);
    await sessionStore.disconnectAllSessions().catch(() => undefined);
    sessionStore.cleanupEventListeners();
    transferStore.clearLocalState();
    await assetStore.clearWorkspace().catch(() => undefined);
    store.clearLoginGatewayRequired();
    emit('close');
    window.location.reload();
    return;
  } catch (error) {
    await store.saveSettings(previousState).catch(() => undefined);
    cloudStatusMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isCloudLoggingIn.value = false;
  }
}

async function syncSettingsNow() {
  isCloudSyncing.value = true;
  cloudStatusMessage.value = '';
  try {
    await store.saveSettings(form.value);
    const response = await store.syncSettingsToCloud();
    await store.loadClientSubscriptionSnapshot().catch(() => null);
    if (store.sync.enabled) {
      await assetStore.syncAssetsToCloud(
        store.sync.endpointUrl || 'http://localhost:5047',
        store.account.mode,
        store.account.userId || store.account.subAccountId || 'local-workspace',
        store.account.accessToken || '',
        () =>
          store.logoutFromCloud({
            preserveIdentity: true,
          }),
      );
      await assetStore.pullAssetsFromCloud(
        store.sync.endpointUrl || 'http://localhost:5047',
        store.account.mode,
        store.account.userId || store.account.subAccountId || 'local-workspace',
        store.account.accessToken || '',
      );
    }
    form.value.account = { ...store.account };
    form.value.sync = { ...store.sync };
    form.value.ai = { ...store.ai };
    subscriptionSummary.value = store.activeSubscriptionSummary();
    cloudStatusMessage.value = response ? `Synced at ${new Date(response.syncedAt).toLocaleString()}` : 'Cloud sync disabled';
  } catch (error) {
    cloudStatusMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isCloudSyncing.value = false;
  }
}

async function openInvoiceCheckout(invoiceId: string) {
  isCreatingCheckout.value = invoiceId;
  cloudStatusMessage.value = '';
  try {
    const transaction = await store.createClientCheckoutSession(invoiceId, selectedCheckoutProvider.value || 'manual');
    if (transaction.checkoutUrl) {
      window.open(transaction.checkoutUrl, '_blank', 'noopener,noreferrer');
      cloudStatusMessage.value = 'Payment link opened';
    } else {
      cloudStatusMessage.value = 'Payment link created';
    }
    let refreshCount = 0;
    const refreshLoop = async () => {
      refreshCount += 1;
      await store.loadClientSubscriptionSnapshot().catch(() => null);
      form.value.ai = { ...store.ai };
      subscriptionSummary.value = store.activeSubscriptionSummary();
      const invoice = store.ai.subscriptionSnapshot?.recentInvoices?.find((item) => item.id === invoiceId);
      if (!invoice) {
        return;
      }

      if (invoice.status === 'paid') {
        cloudStatusMessage.value = 'Payment status updated: paid';
        return;
      }

      if (refreshCount < 3) {
        window.setTimeout(() => {
          void refreshLoop();
        }, 2000);
      }
    };
    window.setTimeout(() => {
      void refreshLoop();
    }, 1500);
  } catch (error) {
    cloudStatusMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isCreatingCheckout.value = null;
  }
}

async function refreshBillingSnapshot() {
  isRefreshingBilling.value = true;
  cloudStatusMessage.value = '';
  try {
    await store.loadClientSubscriptionSnapshot();
    form.value.ai = { ...store.ai };
    subscriptionSummary.value = store.activeSubscriptionSummary();
    cloudStatusMessage.value = 'Billing status refreshed';
  } catch (error) {
    cloudStatusMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isRefreshingBilling.value = false;
  }
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
  { id: 'account', label: 'settings.accountAndSync' },
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
                    :disabled="isCloudManagedSubscription && !form.ai.subscription.useCustomEndpoint"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast disabled:cursor-not-allowed disabled:opacity-60">
                    <option value="openai">{{ t('aiProviders.openai') }}</option>
                    <option value="anthropic">{{ t('aiProviders.anthropic') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.apiUrl') }}</label>
                  <input v-model="form.ai.apiUrl" type="text"
                    :disabled="isCloudManagedSubscription && !form.ai.subscription.useCustomEndpoint"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast disabled:cursor-not-allowed disabled:opacity-60"
                    placeholder="https://api.openai.com/v1" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.apiKey') }}</label>
                  <input v-model="form.ai.apiKey" type="password"
                    :disabled="isCloudManagedSubscription && !form.ai.subscription.useCustomEndpoint"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast disabled:cursor-not-allowed disabled:opacity-60"
                    placeholder="sk-..." />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.modelName') }}</label>
                  <input v-model="form.ai.modelName" type="text"
                    :disabled="isCloudManagedSubscription && !form.ai.subscription.useCustomEndpoint"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast disabled:cursor-not-allowed disabled:opacity-60"
                    placeholder="gpt-3.5-turbo" />
                </div>
                <div class="rounded border border-border-primary bg-bg-tertiary/50 p-4 space-y-4">
                  <div>
                    <h4 class="text-sm font-semibold text-text-primary">{{ t('settings.aiSubscriptionTitle') }}</h4>
                    <p class="mt-1 text-xs text-text-secondary">{{ t('settings.aiSubscriptionDesc') }}</p>
                  </div>
                  <div class="flex flex-wrap gap-2 text-xs text-text-secondary">
                    <span class="rounded-full border border-border-primary bg-bg-secondary px-3 py-1">{{ subscriptionSummary.label }}</span>
                    <span class="rounded-full border border-border-primary bg-bg-secondary px-3 py-1">{{ subscriptionSummary.scope }}</span>
                    <span class="rounded-full border border-border-primary bg-bg-secondary px-3 py-1">{{ subscriptionSummary.billing }}</span>
                    <span v-if="subscriptionSummary.renewal" class="rounded-full border border-border-primary bg-bg-secondary px-3 py-1">renew {{ subscriptionSummary.renewal }}</span>
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.subscriptionPlan') }}</label>
                    <select v-model="form.ai.subscription.plan"
                      :disabled="isCloudManagedSubscription"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast disabled:cursor-not-allowed disabled:opacity-60">
                      <option value="free">{{ t('settings.subscriptionPlans.free') }}</option>
                      <option value="personal">{{ t('settings.subscriptionPlans.personal') }}</option>
                      <option value="team">{{ t('settings.subscriptionPlans.team') }}</option>
                      <option value="enterprise">{{ t('settings.subscriptionPlans.enterprise') }}</option>
                      <option value="custom">{{ t('settings.subscriptionPlans.custom') }}</option>
                    </select>
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.subscriptionStatus') }}</label>
                    <select v-model="form.ai.subscription.status"
                      :disabled="isCloudManagedSubscription"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast disabled:cursor-not-allowed disabled:opacity-60">
                      <option value="inactive">{{ t('settings.subscriptionStatuses.inactive') }}</option>
                      <option value="trialing">{{ t('settings.subscriptionStatuses.trialing') }}</option>
                      <option value="active">{{ t('settings.subscriptionStatuses.active') }}</option>
                      <option value="pastDue">{{ t('settings.subscriptionStatuses.pastDue') }}</option>
                      <option value="cancelled">{{ t('settings.subscriptionStatuses.cancelled') }}</option>
                    </select>
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.subscriptionSeats') }}</label>
                    <input v-model.number="form.ai.subscription.seats" type="number" min="1" max="9999"
                      :disabled="isCloudManagedSubscription"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast disabled:cursor-not-allowed disabled:opacity-60" />
                  </div>
                  <div v-if="isCloudManagedSubscription" class="rounded border border-accent/30 bg-accent/10 px-3 py-2 text-xs text-text-secondary">
                    {{ t('settings.subscriptionManagedNotice') }}
                  </div>
                  <label class="flex items-center gap-2 text-sm text-text-secondary">
                    <input v-model="form.ai.subscription.useCustomEndpoint" type="checkbox"
                      class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                    <span>{{ t('settings.useCustomEndpoint') }}</span>
                  </label>
                  <label class="flex items-center gap-2 text-sm text-text-secondary">
                    <input v-model="form.ai.subscription.syncToCloud" type="checkbox"
                      :disabled="isCloudManagedSubscription"
                      class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                    <span>{{ t('settings.syncCustomEndpointToCloud') }}</span>
                  </label>
                  <div v-if="subscriptionSnapshot" class="rounded border border-border-primary bg-bg-secondary/60 p-4 space-y-3">
                    <div>
                      <h5 class="text-sm font-semibold text-text-primary">{{ t('settings.currentInvoiceTitle') }}</h5>
                      <p class="mt-1 text-xs text-text-secondary">{{ t('settings.currentInvoiceDesc') }}</p>
                    </div>
                    <div class="flex justify-end">
                      <button
                        class="rounded border border-border-primary px-3 py-1 text-xs text-text-primary transition-all-fast hover:bg-bg-secondary disabled:cursor-not-allowed disabled:opacity-60"
                        :disabled="isRefreshingBilling"
                        @click="refreshBillingSnapshot"
                      >
                        {{ isRefreshingBilling ? t('settings.refreshingBilling') : t('settings.refreshBilling') }}
                      </button>
                    </div>
                    <div v-if="subscriptionSnapshot.currentInvoice" class="grid gap-2 text-xs text-text-secondary md:grid-cols-2">
                      <div>{{ t('settings.invoiceStatus') }}: {{ subscriptionSnapshot.currentInvoice.status }}</div>
                      <div>{{ t('settings.invoiceMonth') }}: {{ subscriptionSnapshot.currentInvoice.billingMonth }}</div>
                      <div>{{ t('settings.invoiceTotal') }}: {{ formatMoney(subscriptionSnapshot.currentInvoice.totalAmount, subscriptionSnapshot.currentInvoice.currency) }}</div>
                      <div>{{ t('settings.invoiceRemaining') }}: {{ formatMoney(subscriptionSnapshot.currentInvoice.remainingAmount, subscriptionSnapshot.currentInvoice.currency) }}</div>
                    </div>
                    <p v-else class="text-xs text-text-secondary">{{ t('settings.noCurrentInvoice') }}</p>
                    <div class="grid gap-2 text-xs text-text-secondary md:grid-cols-2">
                      <div>{{ t('settings.usageRequests') }}: {{ subscriptionSnapshot.usage.totalRequests }}</div>
                      <div>{{ t('settings.usageManagedRequests') }}: {{ subscriptionSnapshot.usage.managedRequests }}</div>
                      <div>{{ t('settings.usageTokens') }}: {{ subscriptionSnapshot.usage.totalTokens }}</div>
                      <div>{{ t('settings.usageEstimatedCost') }}: {{ formatMoney(subscriptionSnapshot.usage.estimatedCost, subscriptionSnapshot.usage.currency) }}</div>
                    </div>
                    <div v-if="subscriptionSnapshot.paymentProviders?.length" class="space-y-2">
                      <label class="block text-xs font-semibold text-text-primary">{{ t('settings.paymentProvider') }}</label>
                      <select
                        v-model="selectedCheckoutProvider"
                        class="w-full rounded border border-border-primary bg-bg-tertiary/60 px-3 py-2 text-xs text-text-primary outline-none transition-all-fast"
                      >
                        <option
                          v-for="provider in subscriptionSnapshot.paymentProviders"
                          :key="provider.providerKey"
                          :value="provider.providerKey"
                        >
                          {{ provider.displayName }} ({{ provider.providerType }})
                        </option>
                      </select>
                    </div>
                    <div v-if="subscriptionSnapshot.recentInvoices?.length" class="space-y-2">
                      <p class="text-xs font-semibold text-text-primary">{{ t('settings.recentInvoicesTitle') }}</p>
                      <div
                        v-for="invoice in subscriptionSnapshot.recentInvoices"
                        :key="invoice.id"
                        class="rounded border border-border-primary bg-bg-tertiary/60 px-3 py-3 text-xs text-text-secondary"
                      >
                        <div class="flex flex-wrap items-center justify-between gap-2">
                          <div>
                            <div class="font-medium text-text-primary">{{ invoice.billingMonth }} · {{ invoice.status }}</div>
                            <div class="mt-1">{{ formatMoney(invoice.totalAmount, invoice.currency) }} · {{ t('settings.invoiceRemaining') }} {{ formatMoney(invoice.remainingAmount, invoice.currency) }}</div>
                            <div v-if="invoice.payments?.length" class="mt-2 space-y-1">
                              <div
                                v-for="payment in invoice.payments"
                                :key="payment.id"
                                class="text-[11px] text-text-secondary"
                              >
                                {{ payment.paymentMethod }} · {{ payment.status }} · {{ formatMoney(payment.amount, payment.currency) }}
                              </div>
                            </div>
                          </div>
                          <button
                            v-if="invoice.remainingAmount > 0"
                            class="rounded border border-border-primary px-3 py-1 text-xs text-text-primary transition-all-fast hover:bg-bg-secondary disabled:cursor-not-allowed disabled:opacity-60"
                            :disabled="isCreatingCheckout === invoice.id"
                            @click="openInvoiceCheckout(invoice.id)"
                          >
                            {{ isCreatingCheckout === invoice.id ? t('settings.openingCheckout') : t('settings.payInvoice') }}
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
                <div class="rounded border border-border-primary bg-bg-tertiary/50 p-4 space-y-4">
                  <div>
                    <h4 class="text-sm font-semibold text-text-primary">{{ t('settings.customEndpointTitle') }}</h4>
                    <p class="mt-1 text-xs text-text-secondary">{{ t('settings.customEndpointDesc') }}</p>
                  </div>
                  <div v-if="!form.ai.subscription.useCustomEndpoint" class="rounded border border-warning/40 bg-warning/10 px-3 py-2 text-xs text-text-secondary">
                    {{ t('settings.customEndpointLocked') }}
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.endpointName') }}</label>
                    <input v-model="form.ai.customEndpoint.endpointName" type="text"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                      :placeholder="t('settings.endpointNamePlaceholder')" :disabled="!form.ai.subscription.useCustomEndpoint" />
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.customApiUrl') }}</label>
                    <input v-model="form.ai.customEndpoint.apiUrl" type="text"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                      placeholder="https://api.openai.com/v1" :disabled="!form.ai.subscription.useCustomEndpoint" />
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.customApiKey') }}</label>
                    <input v-model="form.ai.customEndpoint.apiKey" type="password"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                      placeholder="sk-..." :disabled="!form.ai.subscription.useCustomEndpoint" />
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.customModelName') }}</label>
                    <input v-model="form.ai.customEndpoint.modelName" type="text"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                      placeholder="gpt-4o-mini" :disabled="!form.ai.subscription.useCustomEndpoint" />
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.customProviderType') }}</label>
                    <select v-model="form.ai.customEndpoint.providerType"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                      :disabled="!form.ai.subscription.useCustomEndpoint">
                      <option value="openai">{{ t('aiProviders.openai') }}</option>
                      <option value="anthropic">{{ t('aiProviders.anthropic') }}</option>
                    </select>
                  </div>
                </div>
              </div>
            </section>
          </div>

          <div v-if="activeTab === 'account'" class="space-y-6">
            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.accountModeTitle') }}</h3>
              <p class="text-sm text-text-secondary mb-4">{{ t('settings.accountModeDesc') }}</p>
              <div class="space-y-4">
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.accountMode') }}</label>
                  <select v-model="form.account.mode"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast">
                    <option value="local">{{ t('settings.accountModes.local') }}</option>
                    <option value="personal">{{ t('settings.accountModes.personal') }}</option>
                    <option value="enterpriseSubAccount">{{ t('settings.accountModes.enterpriseSubAccount') }}</option>
                  </select>
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.displayName') }}</label>
                  <input v-model="form.account.displayName" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.accountEmail') }}</label>
                  <input v-model="form.account.email" type="email"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div v-if="form.account.mode !== 'local'">
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.accountId') }}</label>
                  <input v-model="form.account.userId" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div v-if="form.account.mode === 'enterpriseSubAccount'">
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.enterpriseId') }}</label>
                  <input v-model="form.account.enterpriseId" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div v-if="form.account.mode === 'enterpriseSubAccount'">
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.enterpriseName') }}</label>
                  <input v-model="form.account.enterpriseName" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div v-if="form.account.mode === 'enterpriseSubAccount'">
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.subAccountId') }}</label>
                  <input v-model="form.account.subAccountId" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
                <div v-if="form.account.mode !== 'local'">
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.cloudAccessToken') }}</label>
                  <input v-model="form.account.accessToken" type="password"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast" />
                </div>
              </div>
            </section>

            <section>
              <h3 class="text-lg font-semibold text-text-primary mb-4">{{ t('settings.cloudSyncTitle') }}</h3>
              <p class="text-sm text-text-secondary mb-4">{{ t('settings.cloudSyncDesc') }}</p>
              <div class="space-y-4">
                <label class="flex items-center gap-2 text-sm text-text-secondary">
                  <input v-model="form.sync.enabled" type="checkbox"
                    class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                  <span>{{ t('settings.cloudSyncEnabled') }}</span>
                </label>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.cloudSyncEndpoint') }}</label>
                  <input v-model="form.sync.endpointUrl" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                    placeholder="https://sync.example.com/api" />
                </div>
                <div>
                  <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.organizationScope') }}</label>
                  <input v-model="form.sync.organizationScope" type="text"
                    class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                    :placeholder="t('settings.organizationScopePlaceholder')" />
                </div>
                <label class="flex items-center gap-2 text-sm text-text-secondary">
                  <input v-model="form.sync.syncAssets" type="checkbox"
                    class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                  <span>{{ t('settings.syncAssets') }}</span>
                </label>
                <label class="flex items-center gap-2 text-sm text-text-secondary">
                  <input v-model="form.sync.syncSettings" type="checkbox"
                    class="bg-bg-secondary border-border-primary rounded text-text-primary focus:ring-accent focus:ring-offset-bg-secondary focus:ring-offset-0" />
                  <span>{{ t('settings.syncSettingsLabel') }}</span>
                </label>
                <div v-if="form.account.mode !== 'local'" class="space-y-3 rounded border border-border-primary bg-bg-tertiary/50 p-4">
                  <div>
                    <label class="block text-sm font-medium text-secondary mb-1">{{ t('settings.cloudAccessToken') }}</label>
                    <input v-model="cloudSecret" type="password"
                      class="w-full bg-bg-secondary border border-border-primary rounded px-3 py-2 text-text-primary focus:border-accent outline-none transition-all-fast"
                      placeholder="temporary login secret" />
                  </div>
                  <div class="flex flex-wrap gap-3">
                    <button
                      class="px-4 py-2 text-sm bg-accent hover:bg-accent/80 text-text-primary rounded disabled:opacity-50"
                      :disabled="isCloudLoggingIn"
                      @click="loginToCloud"
                    >
                      {{ isCloudLoggingIn ? 'Connecting...' : 'Cloud Login' }}
                    </button>
                    <button
                      class="px-4 py-2 text-sm bg-success hover:bg-success/80 text-text-primary rounded disabled:opacity-50"
                      :disabled="isCloudSyncing"
                      @click="syncSettingsNow"
                    >
                      {{ isCloudSyncing ? 'Syncing...' : 'Sync Settings Now' }}
                    </button>
                  </div>
                  <p v-if="cloudStatusMessage" class="text-xs text-text-secondary">{{ cloudStatusMessage }}</p>
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
