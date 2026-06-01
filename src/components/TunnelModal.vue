<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Pencil, Play, Plus, Square, Trash2 } from 'lucide-vue-next';
import type { AccessEndpoint, HostAsset, Tunnel, TunnelType } from '../types';
import { useTunnelStore } from '../stores/tunnels';
import { useAssetStore } from '../stores/assets';
import { useNotificationStore } from '../stores/notifications';
import { useI18n } from '../composables/useI18n';

type ModalMode = 'list' | 'create' | 'edit';

interface ScenarioCard {
  type: TunnelType;
  titleKey: string;
  descriptionKey: string;
}

const props = defineProps<{ show: boolean; asset: HostAsset | null }>();
defineEmits(['close']);

const tunnelStore = useTunnelStore();
const assetStore = useAssetStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();

const editingId = ref<number | null>(null);
const mode = ref<ModalMode>('list');
const selectedScenario = ref<TunnelType | null>(null);

const scenarioCards: ScenarioCard[] = [
  {
    type: 'local',
    titleKey: 'tunnels.scenarios.localTitle',
    descriptionKey: 'tunnels.scenarios.localDescription',
  },
  {
    type: 'remote',
    titleKey: 'tunnels.scenarios.remoteTitle',
    descriptionKey: 'tunnels.scenarios.remoteDescription',
  },
  {
    type: 'dynamic',
    titleKey: 'tunnels.scenarios.dynamicTitle',
    descriptionKey: 'tunnels.scenarios.dynamicDescription',
  },
];

const defaultForm = (): Tunnel => ({
  name: '',
  assetId: props.asset?.id ?? 0,
  accessEndpointId: 0,
  tunnelType: 'local',
  localHost: '127.0.0.1',
  localPort: undefined,
  remoteHost: '',
  remotePort: undefined,
  remoteBindHost: '127.0.0.1',
});

const form = ref<Tunnel>(defaultForm());

const availableEndpoints = computed(() => {
  if (!props.asset?.id) return [] as AccessEndpoint[];
  return assetStore.accessEndpoints.filter((endpoint) => endpoint.assetId === props.asset?.id);
});

const endpointMap = computed(() => {
  const map = new Map<number, AccessEndpoint>();
  for (const endpoint of assetStore.accessEndpoints) {
    if (endpoint.id != null) {
      map.set(endpoint.id, endpoint);
    }
  }
  return map;
});

const selectedEndpoint = computed(() => endpointMap.value.get(form.value.accessEndpointId) ?? null);
const activeScenario = computed<TunnelType | null>(() =>
  mode.value === 'create' ? selectedScenario.value : form.value.tunnelType
);
const hasSelectedScenario = computed(() => activeScenario.value !== null);
const isLocal = computed(() => activeScenario.value === 'local');
const isRemote = computed(() => activeScenario.value === 'remote');
const isDynamic = computed(() => activeScenario.value === 'dynamic');
const formTitle = computed(() =>
  mode.value === 'edit' ? t('tunnels.editTitle') : t('tunnels.createTitle')
);
const mappingPreview = computed(() =>
  hasSelectedScenario.value ? formatMapping(form.value) : t('tunnels.selectScenarioFirst')
);

watch(
  () => props.show,
  async (val) => {
    if (val && props.asset?.id) {
      await assetStore.loadAssets();
      await tunnelStore.loadTunnels(props.asset.id);
      await tunnelStore.refreshActive();
      resetToList();
    }
  }
);

function formatEndpoint(endpoint: AccessEndpoint | null | undefined): string {
  if (!endpoint) return t('tunnels.endpointUnknown');
  return `${endpoint.name} · ${endpoint.username}@${endpoint.host}:${endpoint.port}`;
}

function resetForm(type: TunnelType = form.value.tunnelType) {
  form.value = {
    ...defaultForm(),
    tunnelType: type,
    assetId: props.asset?.id ?? 0,
  };
  editingId.value = null;
  selectedScenario.value = type;
}

function resetToList() {
  resetForm('local');
  selectedScenario.value = null;
  mode.value = 'list';
}

function startCreate() {
  form.value = {
    ...defaultForm(),
    assetId: props.asset?.id ?? 0,
  };
  editingId.value = null;
  selectedScenario.value = null;
  mode.value = 'create';
}

function applyScenario(type: TunnelType) {
  selectedScenario.value = type;
  form.value.tunnelType = type;

  if (type !== 'local') {
    form.value.remoteHost = '';
  }

  if (type !== 'remote') {
    form.value.remoteBindHost = '127.0.0.1';
  }

  if (type === 'dynamic') {
    form.value.remoteHost = '';
    form.value.remotePort = undefined;
    form.value.remoteBindHost = '127.0.0.1';
  }
}

function formatMapping(tunnel: Tunnel): string {
  const localHost = tunnel.localHost?.trim() || '127.0.0.1';
  if (tunnel.tunnelType === 'local') {
    const remoteHost = tunnel.remoteHost?.trim() || t('tunnels.previewRemoteHost');
    const remotePort = tunnel.remotePort ?? t('tunnels.previewPort');
    const localPort = tunnel.localPort ?? t('tunnels.previewPort');
    return `${localHost}:${localPort} -> ${remoteHost}:${remotePort}`;
  }
  if (tunnel.tunnelType === 'remote') {
    const remoteBindHost = tunnel.remoteBindHost?.trim() || '127.0.0.1';
    const remotePort = tunnel.remotePort ?? t('tunnels.previewPort');
    const localPort = tunnel.localPort ?? t('tunnels.previewPort');
    return `${remoteBindHost}:${remotePort} -> ${localHost}:${localPort}`;
  }
  const localPort = tunnel.localPort ?? t('tunnels.previewPort');
  return `${localHost}:${localPort} (SOCKS)`;
}

function validateForm(): string | null {
  if (!hasSelectedScenario.value) return t('tunnels.tunnelType');
  if (!form.value.name.trim()) return t('tunnels.name');
  if (!form.value.accessEndpointId) return t('tunnels.accessEndpoint');
  if (isLocal.value) {
    if (!form.value.localPort) return t('tunnels.localPort');
    if (!form.value.remoteHost?.trim()) return t('tunnels.remoteHost');
    if (!form.value.remotePort) return t('tunnels.remotePort');
  }
  if (isRemote.value) {
    if (!form.value.localPort) return t('tunnels.localPort');
    if (!form.value.remotePort) return t('tunnels.remotePort');
  }
  if (isDynamic.value && !form.value.localPort) {
    return t('tunnels.localPort');
  }
  return null;
}

async function saveTunnel() {
  if (!props.asset?.id) return;
  const missing = validateForm();
  if (missing) {
    notificationStore.error(`${missing} ${t('tunnels.required') ?? ''}`.trim());
    return;
  }

  const payload: Tunnel = {
    ...form.value,
    tunnelType: activeScenario.value ?? form.value.tunnelType,
    assetId: props.asset.id,
    accessEndpointId: Number(form.value.accessEndpointId),
    localPort: form.value.localPort ? Number(form.value.localPort) : undefined,
    remotePort: form.value.remotePort ? Number(form.value.remotePort) : undefined,
  };

  if (!payload.remoteHost?.trim()) delete payload.remoteHost;
  if (!payload.remoteBindHost?.trim()) delete payload.remoteBindHost;
  if (!payload.localHost?.trim()) delete payload.localHost;
  if (payload.tunnelType !== 'local') delete payload.remoteHost;
  if (payload.tunnelType === 'dynamic') delete payload.remotePort;
  if (payload.tunnelType !== 'remote') delete payload.remoteBindHost;

  try {
    if (editingId.value) {
      await tunnelStore.updateTunnel({ ...payload, id: editingId.value });
    } else {
      await tunnelStore.createTunnel(payload);
    }
    notificationStore.success(t('tunnels.saved'));
    resetToList();
    await tunnelStore.loadTunnels(props.asset.id);
    await tunnelStore.refreshActive();
  } catch (e: any) {
    notificationStore.error(e?.toString() || t('tunnels.saveFailed'));
  }
}

function editTunnel(tunnel: Tunnel) {
  form.value = { ...defaultForm(), ...tunnel };
  editingId.value = tunnel.id ?? null;
  selectedScenario.value = tunnel.tunnelType;
  mode.value = 'edit';
}

async function deleteTunnel(tunnel: Tunnel) {
  if (!props.asset?.id || !tunnel.id) return;
  if (!window.confirm(t('tunnels.deleteConfirm', { name: tunnel.name }))) return;
  try {
    await tunnelStore.deleteTunnel(tunnel.id, props.asset.id);
    notificationStore.success(t('tunnels.deleted'));
  } catch (e: any) {
    notificationStore.error(e?.toString() || t('tunnels.deleteFailed'));
  }
}

async function startTunnel(tunnel: Tunnel) {
  if (!tunnel.id) return;
  try {
    await tunnelStore.startTunnel(tunnel.id);
    notificationStore.success(t('tunnels.started'));
  } catch (e: any) {
    notificationStore.error(e?.toString() || t('tunnels.startFailed'));
  } finally {
    await tunnelStore.refreshActive();
  }
}

async function stopTunnel(tunnel: Tunnel) {
  if (!tunnel.id) return;
  try {
    await tunnelStore.stopTunnel(tunnel.id);
    notificationStore.info(t('tunnels.stopped'));
  } catch (e: any) {
    notificationStore.error(e?.toString() || t('tunnels.stopFailed'));
  } finally {
    await tunnelStore.refreshActive();
  }
}
</script>

<template>
  <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center bg-bg-overlay">
    <div class="max-h-[90vh] w-[780px] overflow-y-auto rounded border border-border-primary bg-bg-elevated p-6 text-text-primary">
      <div class="mb-4 flex items-center justify-between">
        <div>
          <h2 class="text-xl font-bold text-text-primary">
            {{ t('tunnels.title') }}
          </h2>
          <div v-if="asset" class="mt-1 text-xs text-text-muted">
            {{ asset.name }}
          </div>
        </div>
        <button @click="$emit('close')" class="text-text-muted hover:text-text-primary">✕</button>
      </div>

      <div v-if="mode === 'list'" class="space-y-4">
        <div class="rounded border border-border-secondary bg-bg-primary p-4">
          <div class="flex items-start justify-between gap-4">
            <div>
              <div class="text-sm font-semibold text-text-primary">
                {{ t('tunnels.createEntryTitle') }}
              </div>
              <div class="mt-1 text-xs text-text-secondary">
                {{ t('tunnels.createEntryDescription') }}
              </div>
            </div>
            <button
            @click="startCreate()"
              class="inline-flex items-center gap-2 rounded bg-accent px-3 py-2 text-sm text-white hover:bg-accent/80"
            >
              <Plus class="h-4 w-4" />
              <span>{{ t('tunnels.new') }}</span>
            </button>
          </div>
        </div>

        <div class="space-y-2">
          <div
            v-for="tunnel in tunnelStore.tunnels"
            :key="tunnel.id"
            class="rounded border border-border-secondary p-3"
          >
            <div class="flex items-center justify-between gap-4">
              <div class="min-w-0">
                <div class="text-sm font-semibold text-text-primary">{{ tunnel.name }}</div>
                <div class="text-xs text-text-secondary">
                  {{ t('tunnels.mapping') }}: {{ formatMapping(tunnel) }}
                </div>
                <div class="mt-1 text-[11px] text-text-muted">
                  {{ t('tunnels.boundEndpoint') }}: {{ formatEndpoint(endpointMap.get(tunnel.accessEndpointId)) }}
                </div>
                <div
                  v-if="tunnel.id && tunnelStore.errorMessages[tunnel.id]"
                  class="mt-1 text-[11px] text-error"
                >
                  {{ tunnelStore.errorMessages[tunnel.id] }}
                </div>
              </div>
              <div class="flex items-center space-x-2">
                <span
                  v-if="!tunnelStore.isActive(tunnel.id || 0)"
                  class="rounded bg-bg-tertiary px-2 py-1 text-xs text-text-muted"
                >
                  {{ t('tunnels.inactive') }}
                </span>
                <span
                  v-else
                  class="rounded bg-success/20 px-2 py-1 text-xs text-success"
                >
                  {{ t('tunnels.active') }}
                </span>

                <button
                  v-if="!tunnelStore.isActive(tunnel.id || 0)"
                  @click="startTunnel(tunnel)"
                  class="p-1 text-success hover:text-success/80"
                  :title="t('tunnels.start')"
                >
                  <Play class="h-4 w-4" />
                </button>
                <button
                  v-else
                  @click="stopTunnel(tunnel)"
                  class="p-1 text-warning hover:text-warning/80"
                  :title="t('tunnels.stop')"
                >
                  <Square class="h-4 w-4" />
                </button>

                <button
                  @click="editTunnel(tunnel)"
                  class="p-1 text-text-muted hover:text-info"
                  :title="t('tunnels.tooltipEdit')"
                >
                  <Pencil class="h-4 w-4" />
                </button>
                <button
                  @click="deleteTunnel(tunnel)"
                  class="p-1 text-text-muted hover:text-error"
                  :title="t('tunnels.tooltipDelete')"
                >
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>

          <div
            v-if="tunnelStore.tunnels.length === 0"
            class="rounded border border-dashed border-border-secondary p-4 text-xs text-text-muted"
          >
            {{ t('tunnels.emptyState') }}
          </div>
        </div>
      </div>

      <div v-else class="space-y-4">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-sm font-semibold text-text-primary">{{ formTitle }}</div>
            <div class="mt-1 text-xs text-text-secondary">
              {{ t('tunnels.createHint') }}
            </div>
          </div>
          <button
            @click="resetToList()"
            class="text-xs text-text-muted hover:text-text-primary"
          >
            {{ t('tunnels.backToList') }}
          </button>
        </div>

        <div class="grid grid-cols-3 gap-3">
          <button
            v-for="scenario in scenarioCards"
            :key="scenario.type"
            @click="applyScenario(scenario.type)"
            class="rounded-lg border p-4 text-left transition-colors"
            :class="
              activeScenario === scenario.type
                ? 'border-accent bg-accent/10 text-text-primary'
                : 'border-border-secondary bg-bg-primary text-text-secondary hover:border-border-primary hover:text-text-primary'
            "
          >
            <div class="text-sm font-semibold">
              {{ t(scenario.titleKey) }}
            </div>
            <div class="mt-2 text-xs leading-5">
              {{ t(scenario.descriptionKey) }}
            </div>
          </button>
        </div>

        <div
          v-if="!hasSelectedScenario"
          class="rounded border border-dashed border-border-secondary bg-bg-primary p-4 text-sm text-text-secondary"
        >
          {{ t('tunnels.selectScenarioFirst') }}
        </div>

        <div class="rounded border border-border-secondary p-4">
          <fieldset :disabled="!hasSelectedScenario" class="disabled:opacity-60">
          <div class="grid grid-cols-2 gap-3">
            <div class="col-span-2">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.asset') }}</label>
              <div class="rounded border border-border-secondary bg-bg-primary px-3 py-2 text-sm text-text-primary">
                {{ asset?.name ?? '-' }}
              </div>
            </div>

            <div class="col-span-2">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.accessEndpoint') }}</label>
              <select
                v-model.number="form.accessEndpointId"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              >
                <option :value="0" disabled>{{ t('tunnels.selectEndpoint') }}</option>
                <option v-for="endpoint in availableEndpoints" :key="endpoint.id" :value="endpoint.id">
                  {{ formatEndpoint(endpoint) }}
                </option>
              </select>
            </div>

            <div class="col-span-2 rounded border border-border-secondary bg-bg-primary p-3">
              <div class="text-[11px] uppercase tracking-wide text-text-muted">
                {{ t('tunnels.connectionSourceTitle') }}
              </div>
              <div class="mt-1 text-sm text-text-secondary">
                {{ t('tunnels.connectionSourceDescription') }}
              </div>
              <div class="mt-2 text-xs text-text-muted">
                {{ formatEndpoint(selectedEndpoint) }}
              </div>
            </div>

            <div class="col-span-2">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.name') }}</label>
              <input
                v-model="form.name"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('tunnels.placeholderName')"
              />
            </div>

            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.localHost') }}</label>
              <input
                v-model="form.localHost"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                placeholder="127.0.0.1"
              />
            </div>

            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.localPort') }}</label>
              <input
                v-model.number="form.localPort"
                type="number"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="isDynamic ? '1080' : '8080'"
              />
            </div>

            <template v-if="isLocal">
              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.remoteHost') }}</label>
                <input
                  v-model="form.remoteHost"
                  class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                  placeholder="10.0.0.12"
                />
              </div>
              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.remotePort') }}</label>
                <input
                  v-model.number="form.remotePort"
                  type="number"
                  class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                  placeholder="80"
                />
              </div>
            </template>

            <template v-if="isRemote">
              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.remoteBindHost') }}</label>
                <input
                  v-model="form.remoteBindHost"
                  class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                  placeholder="127.0.0.1"
                />
              </div>
              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('tunnels.remotePort') }}</label>
                <input
                  v-model.number="form.remotePort"
                  type="number"
                  class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                  placeholder="10022"
                />
              </div>
            </template>
          </div>
          </fieldset>

          <div class="mt-4 rounded border border-border-secondary bg-bg-primary p-3">
            <div class="text-[11px] uppercase tracking-wide text-text-muted">
              {{ t('tunnels.mappingPreview') }}
            </div>
            <div class="mt-1 text-sm font-medium text-text-primary">
              {{ mappingPreview }}
            </div>
          </div>

          <div class="mt-4 flex justify-end gap-2">
            <button
              @click="resetToList()"
              class="rounded border border-border-primary px-4 py-2 text-sm text-text-primary hover:bg-bg-primary"
            >
              {{ t('tunnels.cancel') }}
            </button>
            <button
              @click="saveTunnel"
              :disabled="!hasSelectedScenario"
              class="rounded bg-accent px-4 py-2 text-sm text-white hover:bg-accent/80 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {{ t('tunnels.save') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
