<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { Play, Square, Trash2, Pencil } from 'lucide-vue-next';
import type { Connection, Tunnel } from '../types';
import { useTunnelStore } from '../stores/tunnels';
import { useNotificationStore } from '../stores/notifications';
import { useI18n } from '../composables/useI18n';

const props = defineProps<{ show: boolean; connection: Connection | null }>();
const emit = defineEmits(['close']);
const tunnelStore = useTunnelStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();

const editingId = ref<number | null>(null);

const defaultForm = (): Tunnel => ({
  name: '',
  connectionId: props.connection?.id ?? 0,
  tunnelType: 'local',
  localHost: '127.0.0.1',
  localPort: undefined,
  remoteHost: '',
  remotePort: undefined,
  remoteBindHost: '127.0.0.1',
  proxyJump: '',
  proxyCommand: '',
  agentForwarding: false,
});

const form = ref<Tunnel>(defaultForm());

const isLocal = computed(() => form.value.tunnelType === 'local');
const isRemote = computed(() => form.value.tunnelType === 'remote');
const isDynamic = computed(() => form.value.tunnelType === 'dynamic');

watch(
  () => props.show,
  async (val) => {
    if (val && props.connection?.id) {
      await tunnelStore.loadTunnels(props.connection.id);
      await tunnelStore.refreshActive();
      resetForm();
    }
  }
);

function resetForm() {
  form.value = defaultForm();
  editingId.value = null;
}

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

function validateForm(): string | null {
  if (!form.value.name.trim()) return t('tunnels.name');
  if (isLocal.value) {
    if (!form.value.localPort) return t('tunnels.localPort');
    if (!form.value.remoteHost?.trim()) return t('tunnels.remoteHost');
    if (!form.value.remotePort) return t('tunnels.remotePort');
  }
  if (isRemote.value) {
    if (!form.value.localPort) return t('tunnels.localPort');
    if (!form.value.remotePort) return t('tunnels.remotePort');
  }
  if (isDynamic.value) {
    if (!form.value.localPort) return t('tunnels.localPort');
  }
  return null;
}

async function saveTunnel() {
  if (!props.connection?.id) return;
  const missing = validateForm();
  if (missing) {
    notificationStore.error(`${missing} ${t('tunnels.required') ?? ''}`.trim());
    return;
  }

  const payload: Tunnel = {
    ...form.value,
    connectionId: props.connection.id,
    localPort: form.value.localPort ? Number(form.value.localPort) : undefined,
    remotePort: form.value.remotePort ? Number(form.value.remotePort) : undefined,
  };

  if (!payload.proxyJump?.trim()) delete payload.proxyJump;
  if (!payload.proxyCommand?.trim()) delete payload.proxyCommand;
  if (!payload.remoteHost?.trim()) delete payload.remoteHost;
  if (!payload.remoteBindHost?.trim()) delete payload.remoteBindHost;
  if (!payload.localHost?.trim()) delete payload.localHost;

  try {
    if (editingId.value) {
      await tunnelStore.updateTunnel({ ...payload, id: editingId.value });
    } else {
      await tunnelStore.createTunnel(payload);
    }
    resetForm();
  } catch (e: any) {
    notificationStore.error(e?.toString() || 'Failed to save tunnel');
  }
}

function editTunnel(tunnel: Tunnel) {
  form.value = { ...tunnel };
  editingId.value = tunnel.id ?? null;
}

async function deleteTunnel(tunnel: Tunnel) {
  if (!props.connection?.id || !tunnel.id) return;
  if (!window.confirm(t('tunnels.deleteConfirm', { name: tunnel.name }))) return;
  try {
    await tunnelStore.deleteTunnel(tunnel.id, props.connection.id);
  } catch (e: any) {
    notificationStore.error(e?.toString() || 'Failed to delete tunnel');
  }
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
</script>

<template>
  <div v-if="show" class="fixed inset-0 bg-bg-overlay flex items-center justify-center z-50">
    <div class="bg-bg-elevated p-6 rounded w-[720px] text-text-primary max-h-[90vh] overflow-y-auto border border-border-primary">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-xl font-bold text-text-primary">
          {{ t('tunnels.title') }}
          <span class="text-xs text-text-muted ml-2" v-if="connection">{{ connection.name }}</span>
        </h2>
        <button @click="$emit('close')" class="text-text-muted hover:text-text-primary">✕</button>
      </div>

      <div class="space-y-4">
        <div class="border border-border-secondary rounded p-4">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-text-primary">{{ t('tunnels.new') }}</h3>
            <button @click="resetForm" class="text-xs text-text-muted hover:text-text-primary">Reset</button>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div class="col-span-2">
              <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.name') }}</label>
              <input v-model="form.name"
                class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                placeholder="My Tunnel" />
            </div>

            <div>
              <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.type') }}</label>
              <select v-model="form.tunnelType" class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none">
                <option value="local">{{ t('tunnels.local') }}</option>
                <option value="remote">{{ t('tunnels.remote') }}</option>
                <option value="dynamic">{{ t('tunnels.dynamic') }}</option>
              </select>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3 mt-3">
            <div>
              <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.localHost') }}</label>
              <input v-model="form.localHost"
                class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                placeholder="127.0.0.1" />
            </div>
            <div>
              <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.localPort') }}</label>
              <input v-model.number="form.localPort" type="number"
                class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                placeholder="8080" />
            </div>

            <template v-if="isLocal">
              <div>
                <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.remoteHost') }}</label>
                <input v-model="form.remoteHost"
                  class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                  placeholder="127.0.0.1" />
              </div>
              <div>
                <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.remotePort') }}</label>
                <input v-model.number="form.remotePort" type="number"
                  class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                  placeholder="80" />
              </div>
            </template>

            <template v-if="isRemote">
              <div>
                <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.remoteBindHost') }}</label>
                <input v-model="form.remoteBindHost"
                  class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                  placeholder="127.0.0.1" />
              </div>
              <div>
                <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.remotePort') }}</label>
                <input v-model.number="form.remotePort" type="number"
                  class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                  placeholder="10022" />
              </div>
            </template>
          </div>

          <div class="grid grid-cols-2 gap-3 mt-3">
            <div>
              <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.proxyJump') }}</label>
              <input v-model="form.proxyJump"
                class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                placeholder="user@jump:22" />
            </div>
            <div>
              <label class="block text-xs text-text-secondary uppercase mb-1">{{ t('tunnels.proxyCommand') }}</label>
              <input v-model="form.proxyCommand"
                class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                placeholder="ssh -W %h:%p bastion" />
            </div>
          </div>

          <div class="flex items-center mt-3">
            <input id="agentForwarding" v-model="form.agentForwarding" type="checkbox"
              class="text-accent focus:ring-accent bg-bg-tertiary border-border-primary" />
            <label for="agentForwarding" class="ml-2 text-sm text-text-secondary">{{ t('tunnels.agentForwarding') }}</label>
          </div>

          <div class="flex justify-end space-x-2 mt-4">
            <button @click="saveTunnel"
              class="px-4 py-2 bg-accent text-white rounded hover:bg-accent/80 text-sm">
              {{ t('tunnels.save') }}
            </button>
          </div>
        </div>

        <div class="space-y-2">
          <div v-for="tunnel in tunnelStore.tunnels" :key="tunnel.id" class="border border-border-secondary rounded p-3">
            <div class="flex items-center justify-between">
              <div>
                <div class="text-sm font-semibold text-text-primary">{{ tunnel.name }}</div>
                <div class="text-xs text-text-secondary">
                  {{ t('tunnels.mapping') }}: {{ formatMapping(tunnel) }}
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

                <button @click="editTunnel(tunnel)" class="p-1 text-text-muted hover:text-info" title="Edit">
                  <Pencil class="w-4 h-4" />
                </button>
                <button @click="deleteTunnel(tunnel)" class="p-1 text-text-muted hover:text-error" title="Delete">
                  <Trash2 class="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>

          <div v-if="tunnelStore.tunnels.length === 0" class="text-xs text-text-muted">
            {{ t('tunnels.new') }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
