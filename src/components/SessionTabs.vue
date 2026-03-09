<script setup lang="ts">
import { useSessionStore } from '../stores/sessions';
import { X, Loader2, Circle } from 'lucide-vue-next';

const sessionStore = useSessionStore();
</script>

<template>
  <div class="flex items-center h-full space-x-1 overflow-x-auto scrollbar-hide">
    <div v-for="session in sessionStore.sessions" :key="session.id"
      class="flex items-center h-full px-3 min-w-[120px] max-w-[200px] bg-bg-tertiary border-r border-border-primary cursor-pointer hover:bg-bg-elevated select-none"
      :class="{ 'bg-bg-elevated border-t-2 border-t-accent': session.id === sessionStore.activeSessionId }"
      @click="sessionStore.setActiveSession(session.id)">

      <!-- Status Icon -->
      <div class="mr-2 flex items-center justify-center">
        <Loader2 v-if="session.status === 'connecting'" class="w-3 h-3 text-yellow-500 animate-spin" />
        <Circle v-else-if="session.status === 'connected'" class="w-3 h-3 text-green-500 fill-current" />
        <Circle v-else class="w-3 h-3 text-red-500 fill-current" />
      </div>

      <span class="text-xs text-text-primary truncate flex-1">{{ session.connectionName }}</span>
      <button @click.stop="sessionStore.closeSession(session.id)"
        class="ml-2 text-text-secondary hover:text-text-primary p-0.5 rounded-full hover:bg-bg-tertiary">
        <X class="w-3 h-3" />
      </button>
    </div>
  </div>
</template>
