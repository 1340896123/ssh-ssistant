<script setup lang="ts">
import { useConnectionStore } from '../stores/connections';
import { useSessionStore } from '../stores/sessions';
import { onMounted } from 'vue';
import { Trash2, Monitor } from 'lucide-vue-next';

const connectionStore = useConnectionStore();
const sessionStore = useSessionStore();

onMounted(() => {
  connectionStore.loadConnections();
});

function connect(conn: any) {
  sessionStore.createSession(conn);
}
</script>

<template>
  <div class="space-y-1">
    <div v-for="conn in connectionStore.connections" :key="conn.id" 
         class="group flex items-center justify-between p-2 hover:bg-gray-700 rounded cursor-pointer"
         @dblclick="connect(conn)">
      <div class="flex items-center space-x-2 overflow-hidden">
        <Monitor class="w-4 h-4 text-gray-400" />
        <span class="text-sm text-gray-200 truncate" :title="conn.host">{{ conn.name }}</span>
      </div>
      <button @click.stop="connectionStore.deleteConnection(conn.id!)" 
              class="p-1 text-gray-500 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer">
        <Trash2 class="w-4 h-4" />
      </button>
    </div>
  </div>
</template>
