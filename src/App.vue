<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
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
import type { Connection } from "./types";
import { Settings } from "lucide-vue-next";

const sessionStore = useSessionStore();
const connectionStore = useConnectionStore();
const settingsStore = useSettingsStore();
const showConnectionModal = ref(false);
const showSettingsModal = ref(false);
const editingConnection = ref<Connection | null>(null);

// Layout state
const fileWidth = ref(30); // percentage
const aiWidth = ref(30);   // percentage
// Terminal width is derived: 100 - fileWidth - aiWidth

const containerRef = ref<HTMLElement | null>(null);
const isResizing = ref<'file' | 'ai' | null>(null);

const activeSession = computed(() => sessionStore.activeSession);

onMounted(() => {
  settingsStore.loadSettings();
  window.addEventListener('mousemove', handleMouseMove);
  window.addEventListener('mouseup', handleMouseUp);
});

onUnmounted(() => {
  window.removeEventListener('mousemove', handleMouseMove);
  window.removeEventListener('mouseup', handleMouseUp);
});

function startResize(target: 'file' | 'ai') {
  isResizing.value = target;
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function handleMouseMove(e: MouseEvent) {
  if (!isResizing.value || !containerRef.value) return;

  const containerRect = containerRef.value.getBoundingClientRect();
  const totalWidth = containerRect.width;

  if (isResizing.value === 'file') {
    // Calculate new file width percentage based on mouse X relative to container
    const newFileWidth = ((e.clientX - containerRect.left) / totalWidth) * 100;
    // Constraints
    if (newFileWidth > 10 && newFileWidth < (100 - aiWidth.value - 10)) {
      fileWidth.value = newFileWidth;
    }
  } else if (isResizing.value === 'ai') {
    // Calculate new AI width. Mouse X is the left edge of AI panel approximately.
    // AI width = 100 - (percent at mouse X)
    const mousePercent = ((e.clientX - containerRect.left) / totalWidth) * 100;
    const newAiWidth = 100 - mousePercent;

    if (newAiWidth > 10 && newAiWidth < (100 - fileWidth.value - 10)) {
      aiWidth.value = newAiWidth;
    }
  }
}

function handleMouseUp() {
  if (isResizing.value) {
    isResizing.value = null;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }
}

function handleSaveConnection(conn: Connection) {
  const action = conn.id
    ? connectionStore.updateConnection(conn)
    : connectionStore.addConnection(conn);

  action.then((success) => {
    if (success) {
      showConnectionModal.value = false;
      editingConnection.value = null;
    } else {
      alert('Failed to save connection. Please check the logs.');
    }
  });
}

function openNewConnectionModal() {
  editingConnection.value = null;
  showConnectionModal.value = true;
}

function openEditConnectionModal(conn: Connection) {
  editingConnection.value = conn;
  showConnectionModal.value = true;
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
        <ConnectionList @edit="openEditConnectionModal" />
      </div>
      <div class="p-4 border-t border-gray-700">
        <button @click="openNewConnectionModal"
          class="w-full bg-blue-600 hover:bg-blue-500 text-white py-2 px-4 rounded cursor-pointer transition-colors">
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
      <div class="flex-1 relative overflow-hidden" v-if="sessionStore.sessions.length > 0" ref="containerRef">
        <div v-for="session in sessionStore.sessions" :key="session.id" v-show="activeSession && session.id === activeSession.id" class="flex-1 absolute inset-0 flex">
          <!-- Files -->
          <div class="overflow-hidden flex flex-col" :style="{ width: fileWidth + '%' }">
            <FileManager :sessionId="session.id" />
          </div>

          <!-- Resizer 1 -->
          <div class="w-1 bg-gray-800 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
            @mousedown.prevent="startResize('file')"></div>

          <!-- Terminal -->
          <div class="overflow-hidden flex flex-col flex-1 border-l border-r border-gray-700"
            :style="{ width: `calc(100% - ${fileWidth}% - ${aiWidth}%)` }">
            <TerminalView :sessionId="session.id" />
          </div>

          <!-- Resizer 2 -->
          <div class="w-1 bg-gray-800 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
            @mousedown.prevent="startResize('ai')"></div>

          <!-- AI -->
          <div class="overflow-hidden flex flex-col" :style="{ width: aiWidth + '%' }">
            <AIAssistant :sessionId="session.id" />
          </div>
        </div>
      </div>
      <div class="flex-1 flex items-center justify-center text-gray-500" v-else>
        Select a connection to start
      </div>
    </main>

    <ConnectionModal :show="showConnectionModal" :connectionToEdit="editingConnection"
      @close="showConnectionModal = false" @save="handleSaveConnection" />
    <SettingsModal :show="showSettingsModal" @close="showSettingsModal = false" />
  </div>
</template>
