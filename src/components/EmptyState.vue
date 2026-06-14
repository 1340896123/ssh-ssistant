<script setup lang="ts">
import type { Component } from 'vue';

withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: Component;
    /** When true, renders as a compact inline block (no large icon). */
    compact?: boolean;
  }>(),
  {
    description: '',
    icon: undefined,
    compact: false,
  },
);
</script>

<template>
  <div
    v-if="!compact"
    class="rounded-xl border border-dashed border-border-primary bg-bg-secondary/70 p-5 text-center"
  >
    <div
      v-if="icon"
      class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-bg-tertiary text-accent"
    >
      <component :is="icon" class="h-6 w-6" />
    </div>
    <div class="text-sm font-medium text-text-primary">{{ title }}</div>
    <div v-if="description" class="mt-1 text-xs leading-5 text-text-secondary">
      {{ description }}
    </div>
    <div v-if="$slots.actions" class="mt-4 flex items-center justify-center gap-2">
      <slot name="actions" />
    </div>
  </div>
  <div
    v-else
    class="rounded border border-dashed border-border-primary bg-bg-primary px-3 py-4 text-center text-xs text-text-secondary"
  >
    {{ title }}
    <span v-if="description" class="block mt-1">{{ description }}</span>
  </div>
</template>
