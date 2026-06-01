<script setup lang="ts">
defineProps<{
  title: string
  description: string
}>()
</script>

<template>
  <div class="admin-section-head">
    <div>
      <h2 class="admin-section-title">{{ title }}</h2>
      <p class="admin-section-desc">{{ description }}</p>
    </div>
    <slot />
  </div>
</template>
