<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import ConnectionList from "./components/ConnectionList.vue";
import ConnectionModal from "./components/ConnectionModal.vue";
import SessionTabs from "./components/SessionTabs.vue";
import TerminalView from "./components/TerminalView.vue";
import FileManager from "./components/FileManager.vue";
import AIAssistant from "./components/AIAssistant.vue";
import SettingsModal from "./components/SettingsModal.vue";
import { useSessionStore } from "./stores/sessions";
import { useConnectionStore } from "./stores/connections";
import { useSettingsStore } from "./stores/settings";
import { Settings } from "lucide-vue-next";

const sessionStore = useSessionStore();
const connectionStore = useConnectionStore();
const settingsStore = useSettingsStore();
const showConnectionModal = ref(false);
const showSettingsModal = ref(false);

const activeSession = computed(() => sessionStore.activeSession);

onMounted(() => {
  settingsStore.applyTheme();
});

function handleSaveConnection(conn: any) {
  connectionStore.addConnection(conn).then((success) => {
    if (success) {
      showConnectionModal.value = false;
    } else {
      alert('Failed to save connection. Please check the logs.');
    }
  });
}
</script>

<template>
  <div class="h-screen w-screen bg-gray-900 text-white flex overflow-hidden font-sans">
    <!-- Sidebar -->
    <aside class="w-64 bg-gray-800 border-r border-gray-700 flex flex-col flex-shrink-0">
      <div class="p-4 border-b border-gray-700 flex justify-between items-center">
        <h1 class="text-lg font-bold">SSH Assistant</h1>
        <button @click="showSettingsModal = true" class="text-gray-400 hover:text-white" title="Settings">
            <Settings class="w-5 h-5" />
        </button>
      </div>
      <div class="flex-1 overflow-y-auto p-2">
        <ConnectionList />
      </div>
      <div class="p-4 border-t border-gray-700">
        <button @click="showConnectionModal = true" class="w-full bg-blue-600 hover:bg-blue-500 text-white py-2 px-4 rounded cursor-pointer transition-colors">
          New Connection
        </button>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col bg-gray-900 min-w-0">
      <!-- Tabs -->
      <div class="h-10 bg-gray-800 border-b border-gray-700 flex flex-shrink-0">
        <SessionTabs />
      </div>
      
      <!-- Viewport -->
      <div class="flex-1 relative overflow-hidden flex flex-col" v-if="activeSession">
         <!-- Toolbar for session -->
         <div class="h-8 bg-gray-800 border-b border-gray-700 flex items-center px-2 space-x-4 flex-shrink-0">
            <button @click="sessionStore.setActiveTab('terminal')" 
                    :class="{'text-blue-400 border-b-2 border-blue-400': activeSession.activeTab === 'terminal', 'text-gray-400': activeSession.activeTab !== 'terminal'}" 
                    class="text-sm hover:text-white px-2 h-full">Terminal</button>
            <button @click="sessionStore.setActiveTab('files')" 
                    :class="{'text-blue-400 border-b-2 border-blue-400': activeSession.activeTab === 'files', 'text-gray-400': activeSession.activeTab !== 'files'}" 
                    class="text-sm hover:text-white px-2 h-full">Files</button>
            <button @click="sessionStore.setActiveTab('ai')" 
                    :class="{'text-blue-400 border-b-2 border-blue-400': activeSession.activeTab === 'ai', 'text-gray-400': activeSession.activeTab !== 'ai'}" 
                    class="text-sm hover:text-white px-2 h-full">AI Assistant</button>
         </div>
         
         <div class="flex-1 overflow-hidden relative">
             <TerminalView v-if="activeSession.activeTab === 'terminal'" :sessionId="activeSession.id" />
             <FileManager v-if="activeSession.activeTab === 'files'" :sessionId="activeSession.id" />
             <AIAssistant v-if="activeSession.activeTab === 'ai'" />
         </div>
      </div>
      <div class="flex-1 flex items-center justify-center text-gray-500" v-else>
        Select a connection to start
      </div>
    </main>
    
    <ConnectionModal :show="showConnectionModal" @close="showConnectionModal = false" @save="handleSaveConnection" />
    <SettingsModal :show="showSettingsModal" @close="showSettingsModal = false" />
  </div>
</template>
