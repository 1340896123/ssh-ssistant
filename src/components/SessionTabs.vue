<script setup lang="ts">
import { useSessionStore } from '../stores/sessions';
import { X } from 'lucide-vue-next';

const sessionStore = useSessionStore();
</script>

<template>
  <div class="flex items-center h-full space-x-1 overflow-x-auto scrollbar-hide">
    <div v-for="session in sessionStore.sessions" :key="session.id"
         class="flex items-center h-full px-3 min-w-[120px] max-w-[200px] bg-gray-800 border-r border-gray-700 cursor-pointer hover:bg-gray-700 select-none"
         :class="{ '!bg-gray-900 border-t-2 border-t-blue-500': session.id === sessionStore.activeSessionId }"
         @click="sessionStore.setActiveSession(session.id)">
      <span class="text-xs text-gray-300 truncate flex-1">{{ session.connectionName }}</span>
      <button @click.stop="sessionStore.closeSession(session.id)" class="ml-2 text-gray-500 hover:text-white p-0.5 rounded-full hover:bg-gray-600">
        <X class="w-3 h-3" />
      </button>
    </div>
  </div>
</template>
