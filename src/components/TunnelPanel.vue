<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { RefreshCw } from 'lucide-vue-next';
import { useTunnelStore } from '../stores/tunnels';
import { useAssetStore } from '../stores/assets';
import { useNotificationStore } from '../stores/notifications';
import { useI18n } from '../composables/useI18n';
import TunnelCard from './TunnelCard.vue';
import EmptyState from './EmptyState.vue';
import type { AccessEndpoint, HostAsset, Tunnel } from '../types';

const emit = defineEmits<{
  (e: 'manage', asset: HostAsset): void;
}>();

const tunnelStore = useTunnelStore();
const assetStore = useAssetStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();

const selectedAssetId = ref<number | 'all'>('all');
const isLoading = ref(false);

const assetMap = computed(() => {
  const map = new Map<number, HostAsset>();
  for (const asset of assetStore.assets) {
    if (asset.id != null) map.set(asset.id, asset);
  }
  return map;
});

const endpointMap = computed(() => {
  const map = new Map<number, AccessEndpoint>();
  for (const endpoint of assetStore.accessEndpoints) {
    if (endpoint.id != null) map.set(endpoint.id, endpoint);
  }
  return map;
});

const selectedAsset = computed(() => {
  if (selectedAssetId.value === 'all') return null;
  return assetMap.value.get(selectedAssetId.value) || null;
});

async function loadData() {
  isLoading.value = true;
  try {
    if (selectedAssetId.value === 'all') {
      await tunnelStore.loadTunnels();
    } else {
      await tunnelStore.loadTunnels(selectedAssetId.value);
    }
    await tunnelStore.refreshActive();
  } catch (e) {
    console.error('Failed to load tunnels:', e);
  } finally {
    isLoading.value = false;
  }
}

onMounted(async () => {
  await assetStore.loadAssets();
  await loadData();
});

watch(() => selectedAssetId.value, async () => {
  await loadData();
});

function formatMapping(tunnel: Tunnel): string {
  const localHost = tunnel.localHost || '127.0.0.1';
  if (tunnel.tunnelType === 'local') {
    return `${localHost}:${tunnel.localPort} -> ${tunnel.remoteHost}:${tunnel.remotePort}`;
  }
  if (tunnel.tunnelType === 'remote') {
    const remoteBindHost = tunnel.remoteBindHost || '127.0.0.1';
    return `${remoteBindHost}:${tunnel.remotePort} -> ${localHost}:${tunnel.localPort}`;
  }
  return `${localHost}:${tunnel.localPort} (SOCKS)`;
}

function formatEndpoint(endpoint: AccessEndpoint | null | undefined): string {
  if (!endpoint) return t('tunnels.endpointUnknown');
  return `${endpoint.name} · ${endpoint.username}@${endpoint.host}:${endpoint.port}`;
}

async function startTunnel(tunnel: Tunnel) {
  if (!tunnel.id) return;
  try {
    await tunnelStore.startTunnel(tunnel.id);
  } catch (e: any) {
    notificationStore.error(e?.toString() || 'Failed to start tunnel');
  } finally {
    await tunnelStore.refreshActive();
  }
}

async function stopTunnel(tunnel: Tunnel) {
  if (!tunnel.id) return;
  try {
    await tunnelStore.stopTunnel(tunnel.id);
  } catch (e: any) {
    notificationStore.error(e?.toString() || 'Failed to stop tunnel');
  } finally {
    await tunnelStore.refreshActive();
  }
}

function openManage(tunnel?: Tunnel) {
  if (tunnel) {
    const asset = assetMap.value.get(tunnel.assetId);
    if (!asset) {
      notificationStore.error(t('tunnels.assetMissing') || 'Asset not found');
      return;
    }
    emit('manage', asset);
    return;
  }

  if (!selectedAsset.value) {
    notificationStore.error(t('tunnels.selectAsset'));
    return;
  }
  emit('manage', selectedAsset.value);
}

async function deleteTunnel(tunnel: Tunnel) {
  if (!tunnel.id) return;
  const asset = assetMap.value.get(tunnel.assetId);
  if (!asset?.id) {
    notificationStore.error(t('tunnels.assetMissing') || 'Asset not found');
    return;
  }
  if (!window.confirm(t('tunnels.deleteConfirm', { name: tunnel.name }))) return;
  try {
    await tunnelStore.deleteTunnel(tunnel.id, asset.id);
  } catch (e: any) {
    notificationStore.error(e?.toString() || 'Failed to delete tunnel');
  }
}
</script>

<template>
  <div class="space-y-3" data-testid="tunnel-panel-root">
    <div class="flex items-center justify-between">
      <div class="text-sm font-semibold text-text-primary">{{ t('tunnels.title') }}</div>
      <button @click="loadData" class="p-1.5 rounded hover:bg-bg-tertiary text-text-muted hover:text-text-primary"
        :title="t('tunnels.refresh')">
        <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
      </button>
    </div>

    <div class="grid grid-cols-2 gap-2">
      <div>
        <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.asset') }}</label>
        <select v-model="selectedAssetId" data-testid="tunnel-panel-asset-select"
          class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none">
          <option value="all">{{ t('tunnels.allAssets') }}</option>
          <option v-for="asset in assetStore.assets" :key="asset.id" :value="asset.id">
            {{ asset.name }}
          </option>
        </select>
      </div>
      <div class="flex items-end">
        <button @click="openManage()" data-testid="tunnel-panel-manage-selected" class="w-full px-3 py-2 bg-accent text-white rounded text-sm hover:bg-accent/80">
          {{ t('tunnels.manageSelectedAsset') }}
        </button>
      </div>
    </div>

    <div class="space-y-2">
      <TunnelCard
        v-for="tunnel in tunnelStore.tunnels"
        :key="tunnel.id"
        :tunnel="tunnel"
        :mapping="formatMapping(tunnel)"
        :endpoint-label="formatEndpoint(endpointMap.get(tunnel.accessEndpointId))"
        :asset-name="assetMap.get(tunnel.assetId)?.name || ''"
        :error-message="(tunnel.id && tunnelStore.errorMessages[tunnel.id]) || ''"
        :is-active="tunnelStore.isActive(tunnel.id || 0)"
        :show-asset-name="true"
        manage-variant="settings"
        test-id-prefix="tunnel-panel"
        @start="startTunnel"
        @stop="stopTunnel"
        @manage="openManage"
        @delete="deleteTunnel"
      />

      <EmptyState
        v-if="tunnelStore.tunnels.length === 0"
        :title="t('tunnels.none')"
        :description="t('tunnels.emptyState')"
        compact
      />
    </div>
  </div>
</template>