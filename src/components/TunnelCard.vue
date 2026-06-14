<script setup lang="ts">
import { Play, Square, Settings2, Pencil, Trash2 } from 'lucide-vue-next';
import type { Tunnel } from '../types';
import { useI18n } from '../composables/useI18n';

withDefaults(
  defineProps<{
    tunnel: Tunnel;
    mapping: string;
    endpointLabel: string;
    assetName?: string;
    errorMessage?: string;
    isActive: boolean;
    /** Whether to show the per-tunnel asset name line (hidden in per-asset modal context). */
    showAssetName?: boolean;
    /** Icon used for the primary manage/edit action. */
    manageVariant?: 'settings' | 'edit';
    /** Prefix for data-testid attributes. */
    testIdPrefix?: string;
  }>(),
  {
    assetName: '',
    errorMessage: '',
    showAssetName: true,
    manageVariant: 'settings',
    testIdPrefix: 'tunnel-card',
  },
);

const emit = defineEmits<{
  (e: 'start', tunnel: Tunnel): void;
  (e: 'stop', tunnel: Tunnel): void;
  (e: 'manage', tunnel: Tunnel): void;
  (e: 'delete', tunnel: Tunnel): void;
}>();

const { t } = useI18n();
</script>

<template>
  <div
    class="rounded border border-border-secondary p-3"
    :data-testid="`${testIdPrefix}-item`"
    :data-tunnel-name="tunnel.name"
  >
    <div class="flex items-center justify-between gap-4">
      <div class="min-w-0">
        <div class="truncate text-sm font-semibold text-text-primary">{{ tunnel.name }}</div>
        <div class="truncate text-xs text-text-secondary">
          {{ t('tunnels.mapping') }}: {{ mapping }}
        </div>
        <div v-if="showAssetName" class="mt-1 text-[11px] text-text-muted">
          {{ assetName || t('tunnels.assetUnknown') }}
        </div>
        <div class="mt-1 text-[11px] text-text-muted">
          {{ t('tunnels.boundEndpoint') }}: {{ endpointLabel }}
        </div>
        <div
          v-if="errorMessage"
          class="mt-1 text-[11px] text-error"
        >
          {{ errorMessage }}
        </div>
      </div>
      <div class="flex shrink-0 items-center space-x-2">
        <span
          v-if="!isActive"
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
          v-if="!isActive"
          :data-testid="`${testIdPrefix}-start`"
          class="p-1 text-success transition-colors hover:text-success/80"
          :title="t('tunnels.start')"
          @click="emit('start', tunnel)"
        >
          <Play class="h-4 w-4" />
        </button>
        <button
          v-else
          :data-testid="`${testIdPrefix}-stop`"
          class="p-1 text-warning transition-colors hover:text-warning/80"
          :title="t('tunnels.stop')"
          @click="emit('stop', tunnel)"
        >
          <Square class="h-4 w-4" />
        </button>

        <button
          :data-testid="`${testIdPrefix}-manage`"
          class="p-1 text-text-muted transition-colors hover:text-info"
          :title="manageVariant === 'edit' ? t('tunnels.tooltipEdit') : t('tunnels.manage')"
          @click="emit('manage', tunnel)"
        >
          <Pencil v-if="manageVariant === 'edit'" class="h-4 w-4" />
          <Settings2 v-else class="h-4 w-4" />
        </button>
        <button
          :data-testid="`${testIdPrefix}-delete`"
          class="p-1 text-text-muted transition-colors hover:text-error"
          :title="t('tunnels.delete')"
          @click="emit('delete', tunnel)"
        >
          <Trash2 class="h-4 w-4" />
        </button>
      </div>
    </div>
  </div>
</template>
