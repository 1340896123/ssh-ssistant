<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { RefreshCw, Play, Square, Settings2, Trash2 } from 'lucide-vue-next';
import { useTunnelStore } from '../stores/tunnels';
import { useAssetStore } from '../stores/assets';
import { useNotificationStore } from '../stores/notifications';
import { useI18n } from '../composables/useI18n';
import type { HostAsset, Tunnel } from '../types';

const emit = defineEmits<{
  (e: 'manage', asset: HostAsset): void;
}>();

const tunnelStore = useTunnelStore();
const assetStore = useAssetStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();

const selectedConnectionId = ref<number | 'all'>('all');
const isLoading = ref(false);

const assetMap = computed(() => {
  const map = new Map<number, HostAsset>();
  for (const asset of assetStore.assets) {
    if (asset.id != null) map.set(asset.id, asset);
  }
  return map;
});

const selectedAsset = computed(() => {
  if (selectedConnectionId.value === 'all') return null;
  return assetMap.value.get(selectedConnectionId.value) || null;
});

async function loadData() {
  isLoading.value = true;
  try {
    if (selectedConnectionId.value === 'all') {
      await tunnelStore.loadTunnels();
    } else {
      await tunnelStore.loadTunnels(selectedConnectionId.value);
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

watch(() => selectedConnectionId.value, async () => {
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
    const asset = assetMap.value.get(tunnel.connectionId);
    if (!asset) {
      notificationStore.error(t('tunnels.connectionMissing') || 'Connection not found');
      return;
    }
    emit('manage', asset);
    return;
  }

  if (!selectedAsset.value) {
    notificationStore.error(t('tunnels.selectConnection'));
    return;
  }
  emit('manage', selectedAsset.value);
}

async function deleteTunnel(tunnel: Tunnel) {
  if (!tunnel.id) return;
  const asset = assetMap.value.get(tunnel.connectionId);
  if (!asset?.id) {
    notificationStore.error(t('tunnels.connectionMissing') || 'Connection not found');
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
  <div class="space-y-3">
    <div class="flex items-center justify-between">
      <div class="text-sm font-semibold text-text-primary">{{ t('tunnels.title') }}</div>
      <button @click="loadData" class="p-1.5 rounded hover:bg-bg-tertiary text-text-muted hover:text-text-primary"
        :title="t('tunnels.refresh')">
        <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
      </button>
    </div>

    <div class="grid grid-cols-2 gap-2">
      <div>
        <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.connection') }}</label>
        <select v-model="selectedConnectionId"
          class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none">
          <option value="all">{{ t('tunnels.allConnections') }}</option>
          <option v-for="asset in assetStore.assets" :key="asset.id" :value="asset.id">
            {{ asset.name }}
          </option>
        </select>
      </div>
      <div class="flex items-end">
        <button @click="openManage()" class="w-full px-3 py-2 bg-accent text-white rounded text-sm hover:bg-accent/80">
          {{ t('tunnels.new') }}
        </button>
      </div>
    </div>

    <div class="space-y-2">
      <div v-for="tunnel in tunnelStore.tunnels" :key="tunnel.id" class="border border-border-secondary rounded p-3">
        <div class="flex items-center justify-between">
          <div class="min-w-0">
            <div class="text-sm font-semibold text-text-primary truncate">{{ tunnel.name }}</div>
            <div class="text-xs text-text-secondary truncate">
              {{ t('tunnels.mapping') }}: {{ formatMapping(tunnel) }}
            </div>
            <div class="text-[11px] text-text-muted mt-1">
              {{ assetMap.get(tunnel.connectionId)?.name || 'Unknown' }}
            </div>
          </div>
          <div class="flex items-center space-x-2">
            <span class="text-xs px-2 py-1 rounded bg-bg-tertiary text-text-muted" v-if="!tunnelStore.isActive(tunnel.id || 0)">
              {{ t('tunnels.inactive') }}
            </span>
            <span class="text-xs px-2 py-1 rounded bg-success/20 text-success" v-else>
              {{ t('tunnels.active') }}
            </span>

            <button v-if="!tunnelStore.isActive(tunnel.id || 0)" @click="startTunnel(tunnel)"
              class="p-1 text-success hover:text-success/80" :title="t('tunnels.start')">
              <Play class="w-4 h-4" />
            </button>
            <button v-else @click="stopTunnel(tunnel)"
              class="p-1 text-warning hover:text-warning/80" :title="t('tunnels.stop')">
              <Square class="w-4 h-4" />
            </button>

            <button @click="openManage(tunnel)" class="p-1 text-text-muted hover:text-info" :title="t('tunnels.manage')">
              <Settings2 class="w-4 h-4" />
            </button>
            <button @click="deleteTunnel(tunnel)" class="p-1 text-text-muted hover:text-error" :title="t('tunnels.delete')">
              <Trash2 class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      <div v-if="tunnelStore.tunnels.length === 0" class="text-xs text-text-muted">
        {{ t('tunnels.none') }}
      </div>
    </div>
  </div>
</template>
