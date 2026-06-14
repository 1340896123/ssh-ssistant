<script setup lang="ts">
import type { HostAsset } from '../types';

withDefaults(
  defineProps<{
    asset: HostAsset;
    /** Secondary line shown below the asset name (e.g. endpoint label). */
    endpointLabel: string;
    /** Compact variant: removes padding, suitable for dense lists. */
    dense?: boolean;
  }>(),
  {
    dense: false,
  },
);

const emit = defineEmits<{
  (e: 'connect', asset: HostAsset): void;
}>();
</script>

<template>
  <div
    class="rounded-lg border border-border-primary bg-bg-primary"
    :class="dense ? 'px-3 py-2' : 'px-3 py-3'"
  >
    <div class="flex items-start justify-between gap-3">
      <button class="min-w-0 flex-1 text-left" @click="emit('connect', asset)">
        <div class="flex items-center gap-2">
          <slot name="title">
            <span class="truncate text-sm text-text-primary">{{ asset.name }}</span>
          </slot>
          <!-- Badges slot: platform / criticality / source badges -->
          <slot name="badges" />
        </div>
        <div class="mt-1 truncate text-xs text-text-secondary">{{ endpointLabel }}</div>
        <!-- Meta slot: owner / environment / health / reason / time -->
        <div v-if="$slots.meta" class="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-text-secondary">
          <slot name="meta" />
        </div>
      </button>
      <!-- Actions slot: favorite / edit / delete buttons -->
      <div v-if="$slots.actions" class="flex shrink-0 items-center gap-1">
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>
