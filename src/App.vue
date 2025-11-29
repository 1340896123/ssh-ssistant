<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onBeforeUpdate } from "vue";
import { invoke } from "@tauri-apps/api/core";
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
import { useI18n } from "./composables/useI18n";
import type { Connection } from "./types";
import { Settings } from "lucide-vue-next";

const sessionStore = useSessionStore();
const connectionStore = useConnectionStore();
const settingsStore = useSettingsStore();
const { t } = useI18n();
const showConnectionModal = ref(false);
const showSettingsModal = ref(false);
const editingConnection = ref<Connection | null>(null);

// AI Context Refs
const terminalViewRefs = ref<any[]>([]);
const terminalContext = ref('');

onBeforeUpdate(() => {
  terminalViewRefs.value = [];
});

function getActiveTerminalView() {
  if (!activeSession.value) return null;
  const activeIndex = sessionStore.sessions.findIndex(s => s.id === activeSession.value?.id);
  if (activeIndex !== -1 && terminalViewRefs.value[activeIndex]) {
    return terminalViewRefs.value[activeIndex];
  }
  return null;
}

function updateTerminalContext() {
  const activeTerminal = getActiveTerminalView();
  if (activeTerminal && typeof activeTerminal.getContent === 'function') {
    terminalContext.value = activeTerminal.getContent();
  } else {
    terminalContext.value = '';
  }
}

// Layout state
const fileWidth = ref(30); // percentage
const aiWidth = ref(30);   // percentage
// Terminal width is derived: 100 - fileWidth - aiWidth

// Sidebar width state - initialize from cache immediately
const getInitialSidebarWidth = () => {
  const cachedWidth = typeof localStorage !== 'undefined' ? localStorage.getItem('sidebarWidth') : null;
  return cachedWidth ? parseInt(cachedWidth, 10) : 256;
};
const sidebarWidth = ref(getInitialSidebarWidth());

const containerRef = ref<HTMLElement | null>(null);
const isResizing = ref<'file' | 'ai' | 'sidebar' | null>(null);

const activeSession = computed(() => sessionStore.activeSession);

// Session status bar state
const now = ref(Date.now());
interface DiskInfo {
  size: string;
  used: string;
  avail: string;
  percent: string;
}

interface SessionStats {
  uptime: string;
  disk: DiskInfo | null;
  ip: string;
}

const sessionStatus = ref<Record<string, SessionStats>>({});
let statusTimer: number | null = null;
let clockTimer: number | null = null;

function formatDuration(totalSeconds: number): string {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0 || hours > 0) parts.push(`${minutes}m`);
  parts.push(`${seconds}s`);
  return parts.join(" ");
}

const activeSessionDuration = computed(() => {
  if (!activeSession.value || !activeSession.value.connectedAt) return '';
  const diffMs = now.value - activeSession.value.connectedAt;
  if (diffMs <= 0) return '0s';
  const diffSeconds = Math.floor(diffMs / 1000);
  return formatDuration(diffSeconds);
});

async function refreshActiveSessionStatus() {
  if (!activeSession.value) return;
  const id = activeSession.value.id;
  try {
    // Command to get Uptime, Disk (Size|Used|Avail|Use%), and IP
    // We use awk to format disk output: Size|Used|Avail|Use%
    const command = `
      echo "UPTIME_START"; 
      (uptime -p 2>/dev/null || uptime 2>/dev/null); 
      echo "UPTIME_END";
      echo "DISK_START"; 
      df -h / 2>/dev/null | tail -1 | awk '{print $2 "|" $3 "|" $4 "|" $5}'; 
      echo "DISK_END";
      echo "IP_START"; 
      (hostname -I 2>/dev/null || echo 'n/a');
      echo "IP_END";
    `.replace(/\n/g, ' ');

    const result = await invoke<string>('exec_command', { id, command });
    
    // Parse result
    const uptimeMatch = result.match(/UPTIME_START\s*([\s\S]*?)\s*UPTIME_END/);
    const diskMatch = result.match(/DISK_START\s*([\s\S]*?)\s*DISK_END/);
    const ipMatch = result.match(/IP_START\s*([\s\S]*?)\s*IP_END/);

    const uptime = uptimeMatch ? uptimeMatch[1].trim() : 'N/A';
    const diskRaw = diskMatch ? diskMatch[1].trim() : '';
    const ip = ipMatch ? ipMatch[1].trim().split(' ')[0] : 'N/A'; // Take first IP

    let disk: DiskInfo | null = null;
    if (diskRaw) {
      const parts = diskRaw.split('|');
      if (parts.length >= 4) {
        disk = {
          size: parts[0],
          used: parts[1],
          avail: parts[2],
          percent: parts[3]
        };
      }
    }

    sessionStatus.value = {
      ...sessionStatus.value,
      [id]: {
        uptime,
        disk,
        ip
      },
    };
  } catch (e) {
    console.error(e);
    // Keep previous status or set to null/error state if needed
  }
}

onMounted(() => {
  settingsStore.loadSettings();
  window.addEventListener('mousemove', handleMouseMove);
  window.addEventListener('mouseup', handleMouseUp);

  clockTimer = window.setInterval(() => {
    now.value = Date.now();
  }, 1000);

  statusTimer = window.setInterval(() => {
    refreshActiveSessionStatus();
  }, 5000);
});

onUnmounted(() => {
  window.removeEventListener('mousemove', handleMouseMove);
  window.removeEventListener('mouseup', handleMouseUp);

  if (clockTimer !== null) {
    clearInterval(clockTimer);
    clockTimer = null;
  }

  if (statusTimer !== null) {
    clearInterval(statusTimer);
    statusTimer = null;
  }
});

function startResize(target: 'file' | 'ai' | 'sidebar') {
  isResizing.value = target;
  document.body.style.cursor = 'col-resize';
  document.body.style.userSelect = 'none';
}

function handleMouseMove(e: MouseEvent) {
  if (!isResizing.value) return;

  if (isResizing.value === 'sidebar') {
    // Calculate new sidebar width in pixels relative to the viewport
    const windowRect = document.body.getBoundingClientRect();
    const newSidebarWidth = e.clientX - windowRect.left;
    // Constraints: min 10% of screen width, max 50% of screen width
    const screenWidth = window.innerWidth;
    const minWidth = screenWidth * 0.1; // 10% of screen width
    const maxWidth = screenWidth * 0.5; // 50% of screen width
    if (newSidebarWidth >= minWidth && newSidebarWidth <= maxWidth) {
      sidebarWidth.value = newSidebarWidth;
      // Save to localStorage
      localStorage.setItem('sidebarWidth', newSidebarWidth.toString());
    }
    return;
  }

  // For file and ai resizers, we need containerRef
  if (!containerRef.value) return;
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
    <aside class="bg-gray-800 border-r border-gray-700 flex flex-col flex-shrink-0" :style="{ width: sidebarWidth + 'px' }">
      <div class="p-4 border-b border-gray-700 flex justify-between items-center">
        <h1 class="text-lg font-bold">{{ t('app.title') }}</h1>
        <button @click="showSettingsModal = true" class="text-gray-400 hover:text-white" :title="t('app.settings')">
          <Settings class="w-5 h-5" />
        </button>
      </div>
      <div class="flex-1 overflow-y-auto p-2">
        <ConnectionList @edit="openEditConnectionModal" />
      </div>
      <div class="p-4 border-t border-gray-700">
        <button @click="openNewConnectionModal"
          class="w-full bg-blue-600 hover:bg-blue-500 text-white py-2 px-4 rounded cursor-pointer transition-colors">
          {{ t('app.newConnection') }}
        </button>
      </div>
    </aside>

    <!-- Sidebar Resizer -->
    <div
      class="w-1 bg-gray-600 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
      @mousedown.prevent="startResize('sidebar')"
    ></div>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col bg-gray-900 min-w-0">
      <!-- Tabs -->
      <div class="h-10 bg-gray-800 border-b border-gray-700 flex flex-shrink-0">
        <SessionTabs />
      </div>

      <!-- Viewport -->
      <div class="flex-1 relative overflow-hidden" v-if="sessionStore.sessions.length > 0" ref="containerRef">
        <div
          v-for="(session, index) in sessionStore.sessions"
          :key="session.id"
          v-show="activeSession && session.id === activeSession.id"
          class="flex-1 absolute inset-0 flex flex-col"
        >
          <div class="flex-1 flex overflow-hidden">
            <!-- Files -->
            <div class="overflow-hidden flex flex-col" :style="{ width: fileWidth + '%' }">
              <FileManager :sessionId="session.id" />
            </div>

            <!-- Resizer 1 -->
            <div
              class="w-1 bg-gray-800 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
              @mousedown.prevent="startResize('file')"
            ></div>

            <!-- Terminal -->
            <div
              class="overflow-hidden flex flex-col flex-1 border-l border-r border-gray-700"
              :style="{ width: `calc(100% - ${fileWidth}% - ${aiWidth}%)` }"
            >
              <TerminalView
                :ref="(el: any) => { if (el) terminalViewRefs[index] = el }"
                :sessionId="session.id"
              />
            </div>

            <!-- Resizer 2 -->
            <div
              class="w-1 bg-gray-800 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
              @mousedown.prevent="startResize('ai')"
            ></div>

            <!-- AI -->
            <div class="overflow-hidden flex flex-col" :style="{ width: aiWidth + '%' }">
              <AIAssistant
                :sessionId="session.id"
                :terminal-context="terminalContext"
                @refresh-context="updateTerminalContext"
              />
            </div>
          </div>

          <!-- Session Status Bar -->
          <div class="h-8 bg-gray-800 border-t border-gray-700 text-xs flex items-center justify-between px-3">
            <div class="text-gray-300">
              {{ t('app.sessionDuration') }}: {{ activeSessionDuration || '0s' }}
            </div>
            <div class="flex-1 flex justify-end items-center space-x-4 ml-4">
              <template v-if="sessionStatus[session.id]">
                <!-- Uptime -->
                <div class="text-gray-400 truncate" :title="sessionStatus[session.id].uptime">
                  {{ sessionStatus[session.id].uptime }}
                </div>
                
                <!-- Disk Usage -->
                <div v-if="sessionStatus[session.id].disk" class="group relative flex items-center cursor-help text-gray-400 hover:text-gray-200">
                  <div class="flex items-center space-x-1">
                    <span>Disk:</span>
                    <span :class="{'text-red-400': parseInt(sessionStatus[session.id].disk!.percent) > 90}">
                      {{ sessionStatus[session.id].disk!.percent }}
                    </span>
                  </div>
                  <!-- Custom Tooltip -->
                  <div class="absolute bottom-full right-0 mb-2 hidden group-hover:block z-50">
                    <div class="bg-gray-900 border border-gray-700 rounded shadow-lg p-2 text-xs whitespace-nowrap">
                      <div class="grid grid-cols-2 gap-x-4 gap-y-1">
                        <span class="text-gray-500">Size:</span>
                        <span class="text-gray-200 text-right">{{ sessionStatus[session.id].disk!.size }}</span>
                        <span class="text-gray-500">Used:</span>
                        <span class="text-gray-200 text-right">{{ sessionStatus[session.id].disk!.used }}</span>
                        <span class="text-gray-500">Avail:</span>
                        <span class="text-gray-200 text-right">{{ sessionStatus[session.id].disk!.avail }}</span>
                      </div>
                    </div>
                  </div>
                </div>

                <!-- IP -->
                <div class="text-gray-400 truncate" :title="sessionStatus[session.id].ip">
                  IP: {{ sessionStatus[session.id].ip }}
                </div>
              </template>
              <div v-else class="text-gray-500 italic">
                {{ t('app.loadingStatus') }}
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="flex-1 flex items-center justify-center text-gray-500" v-else>
        {{ t('app.selectConnectionToStart') }}
      </div>
    </main>

    <ConnectionModal :show="showConnectionModal" :connectionToEdit="editingConnection"
      @close="showConnectionModal = false" @save="handleSaveConnection" />
    <SettingsModal :show="showSettingsModal" @close="showSettingsModal = false" />
  </div>
</template>
