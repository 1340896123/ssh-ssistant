import { defineStore } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import type { ClientSubscriptionSnapshot, Settings } from '../types';
import { setI18nLanguage } from '../i18n';
import { cloudService } from '../services';

const DEFAULT_CHECKOUT_RETURN_URL = 'sshstar://billing/success';
const DEFAULT_CHECKOUT_CANCEL_URL = 'sshstar://billing/cancel';

export const useSettingsStore = defineStore('settings', {
  state: (): Settings => ({
    theme: 'dark',
    language: 'zh',
    account: {
      mode: 'local',
      userId: null,
      displayName: 'Local Workspace',
      email: null,
      enterpriseId: null,
      enterpriseName: null,
      subAccountId: null,
      accessToken: null,
      refreshToken: null,
      expiresAt: null,
    },
    sync: {
      enabled: false,
      endpointUrl: '',
      organizationScope: '',
      syncAssets: true,
      syncSettings: true,
      lastCloudSyncAt: null,
    },
    ai: {
      apiUrl: 'https://api.openai.com/v1',
      apiKey: '',
      modelName: 'gpt-3.5-turbo',
      providerType: 'openai',
      subscription: {
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
      },
      customEndpoint: {
        endpointName: 'Default Custom Endpoint',
        apiUrl: 'https://api.openai.com/v1',
        apiKey: '',
        modelName: 'gpt-3.5-turbo',
        providerType: 'openai',
      },
      subscriptionSnapshot: null as ClientSubscriptionSnapshot | null,
      pendingCheckoutSession: null,
    },
    terminalAppearance: {
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      cursorStyle: 'block',
      lineHeight: 1.0
    },
    fileManager: {
      viewMode: 'flat',
      layout: 'bottom',
      sftpBufferSize: 512
    },
    sshPool: {
      maxBackgroundSessions: 6,
      enableAutoCleanup: true,
      cleanupIntervalMinutes: 5
    },
    connectionTimeout: {
      connectionTimeoutSecs: 15,
      jumpHostTimeoutSecs: 30,
      localForwardTimeoutSecs: 10,
      commandTimeoutSecs: 30,
      sftpOperationTimeoutSecs: 60
    },
    reconnect: {
      maxReconnectAttempts: 5,
      initialDelayMs: 1000,
      maxDelayMs: 30000,
      backoffMultiplier: 2.0,
      enableAutoReconnect: true
    },
    heartbeat: {
      tcpKeepaliveIntervalSecs: 60,
      sshKeepaliveIntervalSecs: 15,
      appHeartbeatIntervalSecs: 30,
      heartbeatTimeoutSecs: 5,
      failedHeartbeatsBeforeAction: 3
    },
    poolHealth: {
      healthCheckIntervalSecs: 60,
      sessionWarmupCount: 1,
      maxSessionAgeMinutes: 60,
      unhealthyThreshold: 3
    },
    networkAdaptive: {
      enableAdaptive: true,
      latencyCheckIntervalSecs: 30,
      highLatencyThresholdMs: 300,
      lowBandwidthThresholdKbps: 100
    }
  }),
  actions: {
    async loadSettings() {
      try {
        const settings = await invoke<Settings>('get_settings');
        this.$patch(settings);
        this.applyTheme();
        await this.applyLanguage();
      } catch (e) {
        console.error('Failed to load settings', e);
      }
    },
    async saveSettings(settings: Partial<Settings>) {
      this.$patch(settings);
      this.applyTheme();
      if (settings.language) {
        await this.applyLanguage();
      }
      try {
        await invoke('save_settings', { settings: this.$state });
      } catch (e) {
        console.error('Failed to save settings', e);
      }
    },
    async loginToCloud(secret = '') {
      const identifier =
        this.account.email?.trim() ||
        this.account.userId?.trim() ||
        this.account.subAccountId?.trim() ||
        'local-workspace';

      const response = await cloudService.login(this.sync.endpointUrl, {
        mode: this.account.mode,
        identifier,
        secret,
      });

      this.account = {
        ...this.account,
        mode: response.mode as Settings['account']['mode'],
        userId: response.accountKey || this.account.userId,
        displayName: response.displayName || this.account.displayName,
        email: response.email || this.account.email,
        enterpriseId: response.enterpriseId || this.account.enterpriseId,
        enterpriseName: response.enterpriseName || this.account.enterpriseName,
        subAccountId: response.subAccountId || this.account.subAccountId,
        accessToken: response.accessToken,
        refreshToken: response.refreshToken,
        expiresAt: Date.parse(response.expiresAt),
        refreshExpiresAt: Date.parse(response.refreshExpiresAt),
      };

      this.sync = {
        ...this.sync,
        enabled: response.mode !== 'local',
        endpointUrl: this.sync.endpointUrl || response.syncEndpointUrl || this.sync.endpointUrl,
      };

      this.ai = {
        ...this.ai,
        subscription: response.aiSubscription ?? this.ai.subscription,
        subscriptionSnapshot: response.subscriptionSnapshot ?? this.ai.subscriptionSnapshot,
        customEndpoint: response.endpointSync
          ? {
              endpointName: response.endpointSync.endpointName,
              apiUrl: response.endpointSync.baseUrl,
              apiKey: this.ai.customEndpoint.apiKey,
              modelName: response.endpointSync.modelName,
              providerType: response.endpointSync.provider as Settings['ai']['customEndpoint']['providerType'],
            }
          : this.ai.customEndpoint,
      };

      if (this.account.accessToken && !this.ai.subscription.useCustomEndpoint) {
        await this.loadManagedAiRuntime().catch(() => undefined);
      }

      await this.saveSettings({});
      return response;
    },
    async syncSettingsToCloud() {
      if (!this.sync.enabled) return null;
      let response;
      try {
        response = await cloudService.syncSettings(this.sync.endpointUrl, {
          mode: this.account.mode,
          accountKey: this.account.userId || this.account.subAccountId || 'local-workspace',
          displayName: this.account.displayName || '',
          email: this.account.email || '',
          enterpriseId: this.account.enterpriseId || '',
          enterpriseName: this.account.enterpriseName || '',
          subAccountId: this.account.subAccountId || '',
          accessToken: this.account.accessToken || '',
          syncEndpointUrl: this.sync.endpointUrl || '',
          organizationScope: this.sync.organizationScope || '',
          syncAssets: this.sync.syncAssets,
          syncSettings: this.sync.syncSettings,
          settingsJson: JSON.stringify(this.$state),
        });
      } catch (error) {
        if (String(error).includes('401')) {
          await this.logoutFromCloud();
        }
        throw error;
      }

      this.sync.lastCloudSyncAt = Date.parse(response.syncedAt);
      this.ai.subscription = response.aiSubscription ?? this.ai.subscription;
      this.ai.subscriptionSnapshot = response.subscriptionSnapshot ?? this.ai.subscriptionSnapshot;
      if (response.endpointSync) {
        this.ai.customEndpoint = {
          ...this.ai.customEndpoint,
          endpointName: response.endpointSync.endpointName,
          apiUrl: response.endpointSync.baseUrl,
          modelName: response.endpointSync.modelName,
          providerType: response.endpointSync.provider as Settings['ai']['customEndpoint']['providerType'],
        };
      }

      await invoke('save_settings', { settings: this.$state });
      return response;
    },
    async pullCloudState() {
      if (!this.sync.enabled) return null;
      const accountKey =
        this.account.userId ||
        this.account.subAccountId ||
        'local-workspace';

      let response;
      try {
        response = await cloudService.pull(
          this.sync.endpointUrl,
          this.account.mode,
          accountKey,
          this.account.accessToken || '',
        );
      } catch (error) {
        if (String(error).includes('401')) {
          await this.logoutFromCloud();
        }
        throw error;
      }

      if (response.settingsJson) {
        try {
          const parsed = JSON.parse(response.settingsJson) as Partial<Settings>;
          this.$patch(parsed);
        } catch (error) {
          console.error('Failed to parse cloud settings payload', error);
        }
      }

      this.sync.lastCloudSyncAt = Date.parse(response.syncedAt);
      if (response.endpointSync) {
        this.ai.customEndpoint = {
          ...this.ai.customEndpoint,
          endpointName: response.endpointSync.endpointName,
          apiUrl: response.endpointSync.baseUrl,
          modelName: response.endpointSync.modelName,
          providerType: response.endpointSync.provider as Settings['ai']['customEndpoint']['providerType'],
        };
      }
      if (response.aiSubscription) {
        this.ai.subscription = response.aiSubscription;
      }
      if (response.subscriptionSnapshot) {
        this.ai.subscriptionSnapshot = response.subscriptionSnapshot;
      }

      if (this.account.accessToken && !this.ai.subscription.useCustomEndpoint) {
        await this.loadManagedAiRuntime().catch(() => undefined);
      }

      await invoke('save_settings', { settings: this.$state });
      return response;
    },
    async refreshCloudSession() {
      if (!this.account.refreshToken) {
        await this.logoutFromCloud();
        return null;
      }

      const response = await cloudService.refresh(
        this.sync.endpointUrl,
        this.account.refreshToken,
      );

      this.account = {
        ...this.account,
        mode: response.mode as Settings['account']['mode'],
        userId: response.accountKey || this.account.userId,
        displayName: response.displayName || this.account.displayName,
        email: response.email || this.account.email,
        enterpriseId: response.enterpriseId || this.account.enterpriseId,
        enterpriseName: response.enterpriseName || this.account.enterpriseName,
        subAccountId: response.subAccountId || this.account.subAccountId,
        accessToken: response.accessToken,
        refreshToken: response.refreshToken,
        expiresAt: Date.parse(response.expiresAt),
        refreshExpiresAt: Date.parse(response.refreshExpiresAt),
      };

      if (this.account.accessToken && !this.ai.subscription.useCustomEndpoint) {
        await this.loadManagedAiRuntime().catch(() => undefined);
      }

      await invoke('save_settings', { settings: this.$state });
      return response;
    },
    async logoutFromCloud() {
      this.account = {
        mode: 'local',
        userId: null,
        displayName: 'Local Workspace',
        email: null,
        enterpriseId: null,
        enterpriseName: null,
        subAccountId: null,
        accessToken: null,
        refreshToken: null,
        expiresAt: null,
        refreshExpiresAt: null,
      };
      this.sync = {
        ...this.sync,
        enabled: false,
        organizationScope: '',
        lastCloudSyncAt: null,
      };
      this.ai.subscriptionSnapshot = null;
      await invoke('save_settings', { settings: this.$state });
    },
    isCloudSessionExpired() {
      return Boolean(this.account.expiresAt && this.account.expiresAt <= Date.now());
    },
    isCloudRefreshExpired() {
      return Boolean(
        this.account.refreshExpiresAt &&
          this.account.refreshExpiresAt <= Date.now(),
      );
    },
    async loadManagedAiRuntime() {
      if (!this.account.accessToken) {
        return null;
      }

      const runtime = await cloudService.getClientAiRuntime(
        this.sync.endpointUrl,
        this.account.accessToken,
      );

      if (!runtime.enabled) {
        return runtime;
      }

      this.ai = {
        ...this.ai,
        apiUrl: runtime.baseUrl,
        apiKey: runtime.apiKey,
        modelName: runtime.modelName,
        providerType: runtime.provider,
      };

      await invoke('save_settings', { settings: this.$state });
      return runtime;
    },
    async loadClientSubscriptionSnapshot() {
      if (!this.account.accessToken) {
        this.ai.subscriptionSnapshot = null;
        return null;
      }

      const snapshot = await cloudService.getClientSubscriptionSnapshot(
        this.sync.endpointUrl,
        this.account.accessToken,
      );
      this.ai.subscriptionSnapshot = snapshot;
      if (snapshot?.subscription) {
        this.ai.subscription = {
          ...this.ai.subscription,
          ...snapshot.subscription,
        };
      }
      await invoke('save_settings', { settings: this.$state });
      return snapshot;
    },
    async createClientCheckoutSession(invoiceId: string, providerKey = 'manual') {
      if (!this.account.accessToken) {
        throw new Error('Cloud session is not available.');
      }

      const transaction = await cloudService.createClientCheckoutSession(
        this.sync.endpointUrl,
        this.account.accessToken,
        {
          invoiceId,
          providerKey,
          returnUrl: DEFAULT_CHECKOUT_RETURN_URL,
          cancelUrl: DEFAULT_CHECKOUT_CANCEL_URL,
        },
      );

      this.ai.pendingCheckoutSession = {
        invoiceId,
        providerKey,
        checkoutUrl: transaction.checkoutUrl,
        externalReference: transaction.externalReference,
        createdAt: Date.now(),
        expiresAt: transaction.expiresAt ? Date.parse(transaction.expiresAt) : null,
      };
      await this.loadClientSubscriptionSnapshot().catch(() => null);
      await invoke('save_settings', { settings: this.$state });
      return transaction;
    },
    async reconcilePendingCheckoutSession() {
      const pending = this.ai.pendingCheckoutSession;
      if (!pending || !this.account.accessToken) {
        return null;
      }

      await this.loadClientSubscriptionSnapshot().catch(() => null);
      const invoice = this.ai.subscriptionSnapshot?.recentInvoices?.find(
        (item) => item.id === pending.invoiceId,
      );

      if (!invoice) {
        this.ai.pendingCheckoutSession = null;
        await invoke('save_settings', { settings: this.$state });
        return null;
      }

      const settled = invoice.status === 'paid' || invoice.remainingAmount <= 0;
      const expired =
        pending.expiresAt !== null &&
        pending.expiresAt !== undefined &&
        pending.expiresAt <= Date.now();

      if (settled || expired || invoice.status === 'overdue') {
        this.ai.pendingCheckoutSession = null;
        await invoke('save_settings', { settings: this.$state });
      }

      return {
        invoice,
        settled,
        expired,
      };
    },
    isCloudManagedSubscription() {
      return this.account.mode !== 'local' && this.sync.enabled;
    },
    activeSubscriptionSummary() {
      const subscription = this.ai.subscription;
      const planLabel = subscription.planDisplayName || subscription.plan;
      const renewal = subscription.renewalAt
        ? new Date(subscription.renewalAt).toLocaleDateString()
        : null;
      return {
        label: planLabel,
        billing: `${subscription.currency || 'USD'} ${subscription.pricePerSeat ?? 0}/seat`,
        scope: subscription.billingScope || 'global',
        renewal,
        canUseCustomEndpoint: subscription.allowCustomEndpoint ?? true,
        usingCustomEndpoint: subscription.useCustomEndpoint,
      };
    },
    applyTheme() {
      if (this.theme === 'dark') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    },
    async applyLanguage() {
      await setI18nLanguage(this.language);
    }
  }
});
