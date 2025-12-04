<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onBeforeUpdate } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ConnectionList from "./components/ConnectionList.vue";
import ConnectionModal from "./components/ConnectionModal.vue";
import SessionTabs from "./components/SessionTabs.vue";
import TerminalTabArea from "./components/TerminalTabArea.vue";
import FileManager from "./components/FileManager.vue";
import AIAssistant from "./components/AIAssistant.vue";
import SettingsModal from "./components/SettingsModal.vue";
import NotificationModal from "./components/NotificationModal.vue";
import { useSessionStore } from "./stores/sessions";
import { useConnectionStore } from "./stores/connections";
import { useSettingsStore } from "./stores/settings";
import { useNotificationStore } from "./stores/notifications";
import { useI18n } from "./composables/useI18n";
import type { Connection } from "./types";
import { Settings } from "lucide-vue-next";

const sessionStore = useSessionStore();
const connectionStore = useConnectionStore();
const settingsStore = useSettingsStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();
const showConnectionModal = ref(false);
const showSettingsModal = ref(false);
const editingConnection = ref<Connection | null>(null);

// AI Context Refs
const terminalTabAreaRefs = ref<any[]>([]);
const terminalContext = ref('');

onBeforeUpdate(() => {
  terminalTabAreaRefs.value = [];
});

function getActiveTerminalView() {
  if (!activeSession.value) return null;
  const activeIndex = sessionStore.sessions.findIndex(s => s.id === activeSession.value?.id);
  if (activeIndex !== -1 && terminalTabAreaRefs.value[activeIndex]) {
    // Get the terminal view from within the tab area
    const tabArea = terminalTabAreaRefs.value[activeIndex];
    return tabArea.terminalView || null;
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
  mount: string;
  filesystem: string;
}

interface MountDetails {
  filesystem: string;
  size: string;
  used: string;
  avail: string;
  percent: string;
  mount: string;
}

interface ProcessInfo {
  pid: string;
  command: string;
  cpu: string;
  memory: string;
  memoryPercent: string;
}

interface SessionStats {
  uptime: string;
  disk: DiskInfo | null;
  mounts: MountDetails[];
  ip: string;
  cpu: {
    usage: string;
    topProcesses: ProcessInfo[];
  } | null;
  memory: {
    usage: string;
    total: string;
    used: string;
    available: string;
    topProcesses: ProcessInfo[];
  } | null;
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
  if (!activeSession.value || activeSession.value.status !== 'connected') return;
  const id = activeSession.value.id;
  try {
    // Command to get Uptime, All Mount Points with detailed info, IP, CPU, Memory, and Top Processes
    const command = `
      echo "UPTIME_START"; 
      (uptime -p 2>/dev/null || uptime 2>/dev/null); 
      echo "UPTIME_END";
      echo "MOUNTS_START"; 
      df -h 2>/dev/null | awk 'NR>1 {print $1 "|" $2 "|" $3 "|" $4 "|" $5 "|" $6}'; 
      echo "MOUNTS_END";
      echo "IP_START"; 
      (hostname -I 2>/dev/null || echo 'n/a');
      echo "IP_END";
      echo "CPU_START";
      top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//';
      echo "CPU_END";
      echo "MEMORY_START";
      free -h | grep "Mem:" | awk '{print $3 "/" $2 " (" $3/$2*100 "%)"}';
      echo "MEMORY_END";
      echo "PROCESSES_START";
      ps aux --sort=-%cpu --no-headers | head -6 | awk 'NR>1 {print $2 "|" $11 "|" $3 "%|" $4 "%|" $6}';
      echo "PROCESSES_END";
      echo "MEMORY_PROCESSES_START";
      ps aux --sort=-%mem --no-headers | head -6 | awk 'NR>1 {print $2 "|" $11 "|" $3 "%|" $4 "%|" $6}';
      echo "MEMORY_PROCESSES_END";
    `.replace(/\n/g, ' ');

    const result = await invoke<string>('exec_command', { id, command });

    // Parse result
    const uptimeMatch = result.match(/UPTIME_START\s*([\s\S]*?)\s*UPTIME_END/);
    const mountsMatch = result.match(/MOUNTS_START\s*([\s\S]*?)\s*MOUNTS_END/);
    const ipMatch = result.match(/IP_START\s*([\s\S]*?)\s*IP_END/);
    const cpuMatch = result.match(/CPU_START\s*([\s\S]*?)\s*CPU_END/);
    const memoryMatch = result.match(/MEMORY_START\s*([\s\S]*?)\s*MEMORY_END/);
    const processesMatch = result.match(/PROCESSES_START\s*([\s\S]*?)\s*PROCESSES_END/);
    const memoryProcessesMatch = result.match(/MEMORY_PROCESSES_START\s*([\s\S]*?)\s*MEMORY_PROCESSES_END/);

    const uptime = uptimeMatch ? uptimeMatch[1].trim() : 'N/A';
    const mountsRaw = mountsMatch ? mountsMatch[1].trim() : '';
    const ip = ipMatch ? ipMatch[1].trim().split(' ')[0] : 'N/A';
    const cpuUsage = cpuMatch ? cpuMatch[1].trim() : 'N/A';
    const memoryUsage = memoryMatch ? memoryMatch[1].trim() : 'N/A';
    const processesRaw = processesMatch ? processesMatch[1].trim() : '';
    const memoryProcessesRaw = memoryProcessesMatch ? memoryProcessesMatch[1].trim() : '';

    // Parse mounts data
    const mounts: MountDetails[] = [];
    let rootDisk: DiskInfo | null = null;

    if (mountsRaw) {
      const lines = mountsRaw.split('\n').filter(line => line.trim());
      for (const line of lines) {
        const parts = line.split('|');
        if (parts.length >= 6) {
          const mountInfo: MountDetails = {
            filesystem: parts[0],
            size: parts[1],
            used: parts[2],
            avail: parts[3],
            percent: parts[4],
            mount: parts[5]
          };
          mounts.push(mountInfo);

          // Set root disk as primary disk info (usually mount point "/")
          if (parts[5] === '/' || (!rootDisk && parts[5].startsWith('/'))) {
            rootDisk = {
              filesystem: parts[0],
              size: parts[1],
              used: parts[2],
              avail: parts[3],
              percent: parts[4],
              mount: parts[5]
            };
          }
        }
      }
    }

    // Parse CPU and Memory data
    const cpuTopProcesses: ProcessInfo[] = [];
    const memoryTopProcesses: ProcessInfo[] = [];
    let memoryDetails: { total: string; used: string; available: string } | null = null;

    if (processesRaw) {
      const lines = processesRaw.split('\n').filter(line => line.trim());
      for (const line of lines) {
        const parts = line.split('|');
        if (parts.length >= 5) {
          cpuTopProcesses.push({
            pid: parts[0],
            command: parts[1],
            cpu: parts[2],
            memory: parts[3],
            memoryPercent: parts[4]
          });
        }
      }
    }

    if (memoryProcessesRaw) {
      const lines = memoryProcessesRaw.split('\n').filter(line => line.trim());
      for (const line of lines) {
        const parts = line.split('|');
        if (parts.length >= 5) {
          memoryTopProcesses.push({
            pid: parts[0],
            command: parts[1],
            cpu: parts[2],
            memory: parts[3],
            memoryPercent: parts[4]
          });
        }
      }
    }

    // Parse memory details from free command output
    let memoryPercentage = 'N/A';
    if (memoryUsage && memoryUsage !== 'N/A') {
      const match = memoryUsage.match(/([\d.]+)%/);
      if (match) {
        memoryPercentage = match[1] + '%';
      }
      const detailMatch = memoryUsage.match(/([\d.]+[KMGT]?)(?:\s*\/\s*([\d.]+[KMGT]?))?/);
      if (detailMatch) {
        memoryDetails = {
          used: detailMatch[1] || 'N/A',
          total: detailMatch[2] || 'N/A',
          available: 'N/A'
        };
      }
    }

    sessionStatus.value = {
      ...sessionStatus.value,
      [id]: {
        uptime,
        disk: rootDisk,
        mounts,
        ip,
        cpu: {
          usage: cpuUsage,
          topProcesses: cpuTopProcesses
        },
        memory: {
          usage: memoryPercentage,
          total: memoryDetails?.total || 'N/A',
          used: memoryDetails?.used || 'N/A',
          available: memoryDetails?.available || 'N/A',
          topProcesses: memoryTopProcesses
        }
      },
    };
  } catch (e) {
    console.error(e);
    // Keep previous status or set to null/error state if needed
  }
}

onMounted(async () => {
  // 配置 Monaco Editor 的 Web Worker
  (window as any).MonacoEnvironment = {
    getWorkerUrl: function (_moduleId: string, label: string) {
      if (label === 'json') {
        return './json.worker.js';
      }
      if (label === 'css' || label === 'scss' || label === 'less') {
        return './css.worker.js';
      }
      if (label === 'html' || label === 'handlebars' || label === 'razor') {
        return './html.worker.js';
      }
      if (label === 'typescript' || label === 'javascript') {
        return './ts.worker.js';
      }
      return './editor.worker.js';
    }
  };

  // 确保设置在组件挂载前完成加载
  await settingsStore.loadSettings();
  window.addEventListener('mousemove', handleMouseMove);
  window.addEventListener('mouseup', handleMouseUp);
  window.addEventListener('keydown', handleGlobalKeydown);

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
  window.removeEventListener('keydown', handleGlobalKeydown);

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

function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && (e.key === 'f' || e.key === 'F')) {
    e.preventDefault();
    // Optional: Trigger your own search functionality here if needed
    // For example, if you want Ctrl+F to focus the terminal search bar if it's visible,
    // you could dispatch a custom event or handle it in the specific component.
    // But the user specifically asked to just "block default browser search".
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
      notificationStore.error('Failed to save connection. Please check the logs.');
    }
  });
}

function openNewConnectionModal() {
  editingConnection.value = null;
  showConnectionModal.value = true;
}

function openFileEditor(sessionId: string, filePath: string, fileName: string) {
  const sessionIndex = sessionStore.sessions.findIndex(s => s.id === sessionId);
  if (sessionIndex !== -1 && terminalTabAreaRefs.value[sessionIndex]) {
    terminalTabAreaRefs.value[sessionIndex].openFileEditor(filePath, fileName);
  }
}

function openEditConnectionModal(conn: Connection) {
  editingConnection.value = conn;
  showConnectionModal.value = true;
}
</script>

<template>
  <div class="h-screen w-screen bg-gray-900 text-white flex overflow-hidden font-sans">
    <!-- Sidebar -->
    <aside class="bg-gray-800 border-r border-gray-700 flex flex-col flex-shrink-0"
      :style="{ width: sidebarWidth + 'px' }">
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
    <div class="w-1 bg-gray-600 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
      @mousedown.prevent="startResize('sidebar')"></div>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col bg-gray-900 min-w-0">
      <!-- Tabs -->
      <div class="h-10 bg-gray-800 border-b border-gray-700 flex flex-shrink-0">
        <SessionTabs />
      </div>

      <!-- Viewport -->
      <div class="flex-1 relative overflow-hidden" v-if="sessionStore.sessions.length > 0" ref="containerRef">
        <div v-for="(session, index) in sessionStore.sessions" :key="session.id"
          v-show="activeSession && session.id === activeSession.id" class="flex-1 absolute inset-0 flex flex-col">
          <div class="flex-1 flex overflow-hidden">
            <!-- Files -->
            <div class="overflow-hidden flex flex-col" :style="{ width: fileWidth + '%' }">
              <FileManager :sessionId="session.id" @openFileEditor="(filePath, fileName) => openFileEditor(session.id, filePath, fileName)" />
            </div>

            <!-- Resizer 1 -->
            <div class="w-1 bg-gray-800 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
              @mousedown.prevent="startResize('file')"></div>

            <!-- Terminal -->
            <div class="overflow-hidden flex flex-col flex-1 border-l border-r border-gray-700"
              :style="{ width: `calc(100% - ${fileWidth}% - ${aiWidth}%)` }">
              <TerminalTabArea :ref="(el: any) => { if (el) terminalTabAreaRefs[index] = el }" :sessionId="session.id" />
            </div>

            <!-- Resizer 2 -->
            <div class="w-1 bg-gray-800 hover:bg-blue-500 cursor-col-resize flex-shrink-0 z-10 transition-colors"
              @mousedown.prevent="startResize('ai')"></div>

            <!-- AI -->
            <div class="overflow-hidden flex flex-col" :style="{ width: aiWidth + '%' }">
              <AIAssistant :sessionId="session.id" :terminal-context="terminalContext"
                @refresh-context="updateTerminalContext" />
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
                <div v-if="sessionStatus[session.id].mounts && sessionStatus[session.id].mounts.length > 0"
                  class="group relative flex items-center cursor-help text-gray-400 hover:text-gray-200 py-1 -my-1 px-2 -mx-2">
                  <div class="flex items-center space-x-1">
                    <span>Disk:</span>
                    <span :class="{ 'text-red-400': sessionStatus[session.id].disk && parseInt(sessionStatus[session.id].disk!.percent) > 90 }">
                      {{ sessionStatus[session.id].disk?.percent || 'N/A' }}
                    </span>
                  </div>
                  <!-- Enhanced Tooltip with all mount points -->
                  <div class="absolute bottom-full right-0 mb-2 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-[100]">
                    <div class="bg-gray-800 border border-gray-600 rounded-lg shadow-xl p-3 text-xs whitespace-nowrap max-w-[300px] max-h-[400px] overflow-y-auto">
                      <div class="text-gray-300 font-medium mb-2 pb-1 border-b border-gray-600">
                        Disk Mounts ({{ sessionStatus[session.id].mounts.length }})
                      </div>
                      <div class="space-y-2">
                        <div v-for="mount in sessionStatus[session.id].mounts" :key="mount.mount" 
                          class="border-b border-gray-700/50 pb-2 last:border-b-0">
                          <div class="flex items-center justify-between mb-1">
                            <span class="text-blue-400 font-mono text-xs truncate flex-1 mr-2" :title="mount.mount">
                              {{ mount.mount }}
                            </span>
                            <span :class="{ 
                              'text-red-400': parseInt(mount.percent) > 90,
                              'text-yellow-400': parseInt(mount.percent) > 80,
                              'text-green-400': parseInt(mount.percent) <= 80 
                            }" class="font-mono text-xs">
                              {{ mount.percent }}
                            </span>
                          </div>
                          <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[10px]">
                            <div class="text-gray-500">FS:</div>
                            <div class="text-gray-300 font-mono truncate" :title="mount.filesystem">{{ mount.filesystem }}</div>
                            <div class="text-gray-500">Size:</div>
                            <div class="text-gray-300 font-mono text-right">{{ mount.size }}</div>
                            <div class="text-gray-500">Used:</div>
                            <div class="text-gray-300 font-mono text-right">{{ mount.used }}</div>
                            <div class="text-gray-500">Avail:</div>
                            <div class="text-gray-300 font-mono text-right">{{ mount.avail }}</div>
                          </div>
                        </div>
                      </div>
                      <!-- Tooltip arrow -->
                      <div class="absolute top-full right-4 -mt-1">
                        <div class="w-2 h-2 bg-gray-800 border-r border-b border-gray-600 transform rotate-45"></div>
                      </div>
                    </div>
                  </div>
                </div>
                <div v-else class="text-gray-500">
                  Disk: N/A
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
    
    <NotificationModal 
      v-if="notificationStore.show"
      :show="notificationStore.show"
      :type="notificationStore.type"
      :title="notificationStore.title"
      :message="notificationStore.message"
      :duration="notificationStore.duration"
      @close="notificationStore.close()"
    />
  </div>
</template>
