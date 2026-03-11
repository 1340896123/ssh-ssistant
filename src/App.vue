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
import { Settings, PanelLeftClose, PanelLeftOpen } from "lucide-vue-next";

const sessionStore = useSessionStore();
const connectionStore = useConnectionStore();
const settingsStore = useSettingsStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();
const showConnectionModal = ref(false);
const showSettingsModal = ref(false);
const editingConnection = ref<Connection | null>(null);
const isSidebarCollapsed = ref(false);

// AI Context Refs
const terminalTabAreaRefs = ref<any[]>([]);
const terminalContext = ref("");

onBeforeUpdate(() => {
  terminalTabAreaRefs.value = [];
  mainColumnRefs.value = [];
});

function getActiveTerminalView() {
  if (!activeSession.value) return null;
  const activeIndex = sessionStore.sessions.findIndex(
    (s) => s.id === activeSession.value?.id
  );
  if (activeIndex !== -1 && terminalTabAreaRefs.value[activeIndex]) {
    // Get the terminal view from within the tab area
    const tabArea = terminalTabAreaRefs.value[activeIndex];
    return tabArea.terminalView || null;
  }
  return null;
}

function updateTerminalContext() {
  const activeTerminal = getActiveTerminalView();
  if (activeTerminal && typeof activeTerminal.getContent === "function") {
    terminalContext.value = activeTerminal.getContent();
  } else {
    terminalContext.value = "";
  }
}

// Layout state
// Layout state
const fileWidth = ref(30); // percentage width for file manager (Left Layout)
const fileHeight = ref(30); // percentage height for file manager (Bottom Layout)
const aiWidth = ref(30); // percentage width for AI assistant
// Terminal occupies remaining space in the left column

const layoutMode = computed(() => settingsStore.fileManager.layout || 'bottom');


// Sidebar width state - initialize from cache immediately
const getInitialSidebarWidth = () => {
  const cachedWidth =
    typeof localStorage !== "undefined"
      ? localStorage.getItem("sidebarWidth")
      : null;
  return cachedWidth ? parseInt(cachedWidth, 10) : 256;
};
const sidebarWidth = ref(getInitialSidebarWidth());

const containerRef = ref<HTMLElement | null>(null);
const mainColumnRefs = ref<HTMLElement[]>([]);
const isResizing = ref<"file" | "ai" | "sidebar" | null>(null);

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

interface ProcessInfo {
  pid: string;
  command: string;
  cpu: string;
  memory: string;
  memoryPercent: string;
}

interface CpuInfo {
  usage: string;
  topProcesses: ProcessInfo[];
}

interface MemoryInfo {
  usage: string;
  total: string;
  used: string;
  available: string;
  topProcesses: ProcessInfo[];
}

interface SessionStats {
  uptime: string;
  disk: DiskInfo | null;
  mounts: DiskInfo[];
  ip: string;
  cpu: CpuInfo | null;
  memory: MemoryInfo | null;
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
  if (minutes > 0) parts.push(`${minutes}m`);
  parts.push(`${seconds}s`);
  return parts.join(" ");
}

const activeSessionDuration = computed(() => {
  if (!activeSession.value || !activeSession.value.connectedAt) return "";
  const diffMs = now.value - activeSession.value.connectedAt;
  if (diffMs <= 0) return "0s";
  const diffSeconds = Math.floor(diffMs / 1000);
  return formatDuration(diffSeconds);
});

async function refreshActiveSessionStatus() {
  if (!activeSession.value || activeSession.value.status !== "connected")
    return;

  const id = activeSession.value.id;

  try {
    const stats = await invoke<SessionStats>("get_remote_system_status", { id });
    sessionStatus.value = {
      ...sessionStatus.value,
      [id]: stats,
    };
  } catch (error: any) {
    console.error(`System monitoring failed for ${id}:`, error);

    // Keep existing data or set simplified error state to avoid empty UI
    const current = sessionStatus.value[id];
    sessionStatus.value = {
      ...sessionStatus.value,
      [id]: {
        uptime: current?.uptime || "Connection Error",
        disk: current?.disk || null,
        mounts: current?.mounts || [],
        ip: current?.ip || "N/A",
        cpu: current?.cpu || null,
        memory: current?.memory || null,
      },
    };
  }
}

onMounted(async () => {
  // 配置 Monaco Editor 的 Web Worker
  (window as any).MonacoEnvironment = {
    getWorkerUrl: function (_moduleId: string, label: string) {
      if (label === "json") {
        return "./json.worker.js";
      }
      if (label === "css" || label === "scss" || label === "less") {
        return "./css.worker.js";
      }
      if (label === "html" || label === "handlebars" || label === "razor") {
        return "./html.worker.js";
      }
      if (label === "typescript" || label === "javascript") {
        return "./ts.worker.js";
      }
      return "./editor.worker.js";
    },
  };

  // 确保设置在组件挂载前完成加载
  await settingsStore.loadSettings();
  window.addEventListener("mousemove", handleMouseMove);
  window.addEventListener("mouseup", handleMouseUp);
  window.addEventListener("keydown", handleGlobalKeydown);

  clockTimer = window.setInterval(() => {
    now.value = Date.now();
  }, 1000);

  // Fixed refresh interval - update every second
  statusTimer = window.setInterval(() => {
    refreshActiveSessionStatus();
  }, 1000); // 1 second refresh interval
});

onUnmounted(() => {
  window.removeEventListener("mousemove", handleMouseMove);
  window.removeEventListener("mouseup", handleMouseUp);
  window.removeEventListener("keydown", handleGlobalKeydown);

  // User activity listeners no longer needed - using fixed interval

  if (clockTimer !== null) {
    clearInterval(clockTimer);
    clockTimer = null;
  }

  if (statusTimer !== null) {
    clearInterval(statusTimer);
    statusTimer = null;
  }
});


function startResize(target: "file" | "ai" | "sidebar") {
  isResizing.value = target;
  if (target === "file") {
    // Cursor depends on layout mode
    if (layoutMode.value === 'bottom') {
      document.body.style.cursor = "row-resize";
    } else {
      document.body.style.cursor = "col-resize";
    }
  } else {
    document.body.style.cursor = "col-resize";
  }
  document.body.style.userSelect = "none";
}

function handleMouseMove(e: MouseEvent) {
  if (!isResizing.value) return;

  if (isResizing.value === "sidebar") {
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
      localStorage.setItem("sidebarWidth", newSidebarWidth.toString());
    }
    return;
  }

  // For ai resizer, we need containerRef
  if (isResizing.value === "ai" && containerRef.value) {
    const containerRect = containerRef.value.getBoundingClientRect();
    const totalWidth = containerRect.width;

    // Calculate new AI width. Mouse X is the left edge of AI panel approximately.
    // AI width = 100 - (percent at mouse X)
    const mousePercent = ((e.clientX - containerRect.left) / totalWidth) * 100;
    const newAiWidth = 100 - mousePercent;

    if (newAiWidth > 10 && newAiWidth < 90) {
      aiWidth.value = newAiWidth;
    }
  } else if (isResizing.value === "file") {
    if (layoutMode.value === 'bottom') {
      // BOTTOM LAYOUT: Vertical Resize

      // For file resizer, we need mainColumnRefs (vertical resize) of the active session
      // Find active session index
      const activeIndex = sessionStore.sessions.findIndex(
        (s) => s.id === activeSession.value?.id
      );

      const colEl = activeIndex !== -1 ? mainColumnRefs.value[activeIndex] : null;

      if (colEl) {
        const columnRect = colEl.getBoundingClientRect();
        const totalHeight = columnRect.height;

        if (totalHeight > 0) {
          // Calculate new file height percentage based on mouse Y relative to main column
          // The resizer is at the top of the file manager, so mouse Y decides the top edge of file manager
          // File Height = 100 - (percent at mouse Y)
          const mousePercent = ((e.clientY - columnRect.top) / totalHeight) * 100;
          const newFileHeight = 100 - mousePercent;

          // Constraints: min 10%, max 80% (leave space for terminal)
          if (newFileHeight > 5 && newFileHeight < 90) {
            fileHeight.value = newFileHeight;
          }
        }
      }
    } else {
      // LEFT LAYOUT: Horizontal Resize

      if (!containerRef.value) return;
      const containerRect = containerRef.value.getBoundingClientRect();
      const totalWidth = containerRect.width;

      // Calculate new file width percentage based on mouse X relative to container
      const newFileWidth = ((e.clientX - containerRect.left) / totalWidth) * 100;

      // Constraints
      // Max width limited by AI width
      if (newFileWidth > 10 && newFileWidth < 100 - aiWidth.value - 10) {
        fileWidth.value = newFileWidth;
      }
    }
  }
}

function handleMouseUp() {
  if (isResizing.value) {
    isResizing.value = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }
}

function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && (e.key === "f" || e.key === "F")) {
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
      notificationStore.error(
        "Failed to save connection. Please check the logs."
      );
    }
  });
}

function openNewConnectionModal() {
  editingConnection.value = null;
  showConnectionModal.value = true;
}

function openFileEditor(sessionId: string, filePath: string, fileName: string) {
  const sessionIndex = sessionStore.sessions.findIndex(
    (s) => s.id === sessionId
  );
  if (sessionIndex !== -1 && terminalTabAreaRefs.value[sessionIndex]) {
    terminalTabAreaRefs.value[sessionIndex].openFileEditor(filePath, fileName);
  }
}

function openEditConnectionModal(conn: Connection) {
  editingConnection.value = conn;
  showConnectionModal.value = true;
}

function switchTerminalToPath(sessionId: string, path: string) {
  const sessionIndex = sessionStore.sessions.findIndex(s => s.id === sessionId);
  if (sessionIndex !== -1 && terminalTabAreaRefs.value[sessionIndex]) {
    terminalTabAreaRefs.value[sessionIndex].switchToPath(path);
  }
}
</script>

<template>
  <div class="h-screen w-screen bg-bg-primary text-text-primary flex overflow-hidden font-sans relative">
    <!-- Sidebar -->
    <aside v-show="!isSidebarCollapsed" class="bg-bg-secondary/95 backdrop-blur-sm border-r border-subtle flex flex-col flex-shrink-0"
      :style="{ width: sidebarWidth + 'px' }">
      <div class="p-4 border-b border-subtle flex justify-between items-center bg-bg-tertiary/80">
        <h1 class="text-lg font-semibold text-text-primary">{{ t("app.title") }}</h1>
        <div class="flex items-center space-x-2">
          <button @click="showSettingsModal = true" class="p-1.5 rounded-md text-text-muted hover:text-primary hover:bg-bg-elevated transition-all duration-fast"
            :title="t('app.settings')">
            <Settings class="w-4 h-4" />
          </button>
          <button @click="isSidebarCollapsed = true" class="p-1.5 rounded-md text-text-muted hover:text-primary hover:bg-bg-elevated transition-all duration-fast"
            :title="t('app.collapseSidebar') || 'Collapse Sidebar'">
            <PanelLeftClose class="w-4 h-4" />
          </button>
        </div>
      </div>
      <div class="flex-1 overflow-y-auto p-2">
        <ConnectionList @edit="openEditConnectionModal" />
      </div>
      <div class="p-4 border-t border-subtle bg-bg-tertiary/80">
        <button @click="openNewConnectionModal"
          class="w-full btn btn-primary">
          {{ t("app.newConnection") }}
        </button>
      </div>
    </aside>

    <!-- Sidebar Resizer -->
    <div v-show="!isSidebarCollapsed"
      class="w-1 bg-bg-tertiary hover:bg-primary cursor-col-resize flex-shrink-0 transition-all duration-normal"
      @mousedown.prevent="startResize('sidebar')">
    </div>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col bg-bg-primary min-w-0 relative">
      <!-- Tabs -->
      <div class="h-10 bg-bg-secondary/95 backdrop-blur-sm border-b border-subtle flex flex-shrink-0">
        <button v-if="isSidebarCollapsed" @click="isSidebarCollapsed = false"
          class="px-3 hover:bg-bg-tertiary border-r border-subtle flex items-center justify-center text-text-muted hover:text-primary transition-all duration-fast"
          :title="t('app.expandSidebar') || 'Expand Sidebar'">
          <PanelLeftOpen class="w-4 h-4" />
        </button>
        <SessionTabs />
      </div>

      <!-- Viewport -->
      <div class="flex-1 relative overflow-hidden" v-if="sessionStore.sessions.length > 0" ref="containerRef">
        <div v-for="(session, index) in sessionStore.sessions" :key="session.id"
          v-show="activeSession && session.id === activeSession.id" class="flex-1 absolute inset-0 flex flex-col fade-in">
          <div class="flex-1 flex overflow-hidden">
            <!-- Left Column: Terminal & Files -->

            <!-- LAYOUT: BOTTOM (Flex Column) -->
            <div v-if="layoutMode === 'bottom'" class="flex flex-col h-full overflow-hidden"
              :style="{ width: `calc(100% - ${aiWidth}%)` }" :ref="(el: any) => { if (el) mainColumnRefs[index] = el }">
              <!-- Terminal -->
              <div class="overflow-hidden flex flex-col flex-1 border-r border-subtle"
                :style="{ height: `calc(100% - ${fileHeight}%)` }">
                <TerminalTabArea :ref="(el: any) => { if (el) terminalTabAreaRefs[index] = el }"
                  :sessionId="session.id" />
              </div>

              <!-- Resizer (Horizontal) -->
              <div
                class="h-1 bg-bg-tertiary hover:bg-primary cursor-row-resize flex-shrink-0 transition-all duration-normal"
                @mousedown.prevent="startResize('file')">
              </div>

              <!-- Files -->
              <div class="overflow-hidden flex flex-col border-r border-subtle bg-bg-secondary/30" :style="{ height: fileHeight + '%' }">
                <FileManager :sessionId="session.id" @openFileEditor="
                  (filePath, fileName) =>
                    openFileEditor(session.id, filePath, fileName)
                " @switchToTerminalPath="switchTerminalToPath" />
              </div>
            </div>

            <!-- LAYOUT: LEFT (Side by Side) -->
            <template v-else>
              <!-- Files (Left) -->
              <div class="overflow-hidden flex flex-col bg-bg-secondary/30" :style="{ width: fileWidth + '%' }">
                <FileManager :sessionId="session.id" @openFileEditor="
                  (filePath, fileName) =>
                    openFileEditor(session.id, filePath, fileName)
                " @switchToTerminalPath="switchTerminalToPath" />
              </div>

              <!-- Resizer (Vertical) -->
              <div class="w-1 bg-bg-tertiary hover:bg-primary cursor-col-resize flex-shrink-0 transition-all duration-normal"
                @mousedown.prevent="startResize('file')">
              </div>

              <!-- Terminal (Center) -->
              <div class="overflow-hidden flex flex-col flex-1 border-l border-r border-subtle"
                :style="{ width: `calc(100% - ${fileWidth}% - ${aiWidth}%)` }">
                <TerminalTabArea :ref="(el: any) => { if (el) terminalTabAreaRefs[index] = el }"
                  :sessionId="session.id" />
              </div>
            </template>

            <!-- Resizer (Vertical separator) -->
            <div class="w-1 bg-bg-tertiary hover:bg-primary cursor-col-resize flex-shrink-0 transition-all duration-normal"
              @mousedown.prevent="startResize('ai')">
            </div>

            <!-- AI -->
            <div class="overflow-hidden flex flex-col bg-bg-secondary/90 backdrop-blur-sm border border-border-primary" :style="{ width: aiWidth + '%' }">
              <AIAssistant :sessionId="session.id" :terminal-context="terminalContext"
                @refresh-context="updateTerminalContext" />
            </div>
          </div>

          <!-- Session Status Bar -->
          <div class="h-8 bg-bg-secondary border-t border-subtle text-xs flex items-center justify-between px-3 backdrop-blur-sm">
            <div class="text-text-primary">
              {{ t("app.sessionDuration") }}:
              {{ activeSessionDuration || "0s" }}
            </div>
            <div class="flex-1 flex justify-end items-center space-x-4 ml-4">
              <template v-if="sessionStatus[session.id]">
                <!-- Uptime -->
                <div class="text-text-secondary truncate" :title="sessionStatus[session.id].uptime">
                  {{ sessionStatus[session.id].uptime }}
                </div>

                <!-- Disk Usage -->
                <div v-if="
                  sessionStatus[session.id].mounts &&
                  sessionStatus[session.id].mounts.length > 0
                "
                  class="group relative flex items-center cursor-help text-text-secondary hover:text-primary transition-colors duration-fast py-1 -my-1 px-2 -mx-2 rounded hover:bg-bg-tertiary/50">
                  <div class="flex items-center space-x-1">
                    <span>Disk:</span>
                    <span
                      :class="{ 'text-error': sessionStatus[session.id].disk && parseInt(sessionStatus[session.id].disk!.percent) > 90, 'text-warning': sessionStatus[session.id].disk && parseInt(sessionStatus[session.id].disk!.percent) > 80, 'status-online': sessionStatus[session.id].disk && parseInt(sessionStatus[session.id].disk!.percent) <= 80 }">
                      {{ sessionStatus[session.id].disk?.percent || "N/A" }}
                    </span>
                  </div>
                  <!-- Enhanced Tooltip with all mount points -->
                  <div
                    class="absolute bottom-full right-0 mb-2 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-tooltip">
                    <div
                      class="bg-bg-elevated rounded-lg shadow-md p-3 text-xs whitespace-nowrap max-w-[300px] max-h-[400px] overflow-y-auto border border-border-primary">
                      <div class="text-text-primary font-medium mb-2 pb-1 border-b border-subtle">
                        Disk Mounts ({{
                          sessionStatus[session.id].mounts.length
                        }})
                      </div>
                      <div class="space-y-2">
                        <div v-for="mount in sessionStatus[session.id].mounts" :key="mount.mount"
                          class="border-b border-subtle/50 pb-2 last:border-b-0">
                          <div class="flex items-center justify-between mb-1">
                            <span class="text-primary font-mono text-xs truncate flex-1 mr-2" :title="mount.mount">
                              {{ mount.mount }}
                            </span>
                            <span :class="{
                              'text-error': parseInt(mount.percent) > 90,
                              'text-warning': parseInt(mount.percent) > 80,
                              'status-online': parseInt(mount.percent) <= 80,
                            }" class="font-mono text-xs">
                              {{ mount.percent }}
                            </span>
                          </div>
                          <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[10px]">
                            <div class="text-text-muted">FS:</div>
                            <div class="text-text-primary font-mono truncate" :title="mount.filesystem">
                              {{ mount.filesystem }}
                            </div>
                            <div class="text-text-muted">Size:</div>
                            <div class="text-text-primary font-mono text-right">
                              {{ mount.size }}
                            </div>
                            <div class="text-text-muted">Used:</div>
                            <div class="text-text-primary font-mono text-right">
                              {{ mount.used }}
                            </div>
                            <div class="text-text-muted">Avail:</div>
                            <div class="text-text-primary font-mono text-right">
                              {{ mount.avail }}
                            </div>
                          </div>
                        </div>
                      </div>
                      <!-- Tooltip arrow -->
                      <div class="absolute top-full right-4 -mt-1">
                        <div class="w-2 h-2 bg-bg-secondary border-r border-b border-subtle transform rotate-45"></div>
                      </div>
                    </div>
                  </div>
                </div>
                <div v-else class="text-text-muted">Disk: N/A</div>

                <!-- CPU Usage -->
                <div v-if="sessionStatus[session.id].cpu"
                  class="group relative flex items-center cursor-help text-text-secondary hover:text-primary transition-colors duration-fast py-1 -my-1 px-2 -mx-2 rounded hover:bg-bg-tertiary/50">
                  <div class="flex items-center space-x-1">
                    <span>CPU:</span>
                    <span :class="{
                      'text-error': sessionStatus[session.id].cpu && parseFloat(sessionStatus[session.id].cpu!.usage) > 90,
                      'text-warning': sessionStatus[session.id].cpu && parseFloat(sessionStatus[session.id].cpu!.usage) > 70,
                      'status-online': sessionStatus[session.id].cpu && parseFloat(sessionStatus[session.id].cpu!.usage) <= 70
                    }">
                      {{ sessionStatus[session.id].cpu?.usage || "N/A" }}
                    </span>
                  </div>
                  <!-- CPU Tooltip with top 5 processes -->
                  <div
                    class="absolute bottom-full right-0 mb-2 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-tooltip">
                    <div
                      class="bg-bg-elevated rounded-lg shadow-md p-3 text-xs whitespace-nowrap max-w-[350px] max-h-[400px] overflow-y-auto border border-border-primary">
                      <div class="text-text-primary font-medium mb-2 pb-1 border-b border-subtle">
                        Top 5 CPU Processes
                      </div>
                      <div class="space-y-2">
                        <div v-for="process in sessionStatus[
                          session.id
                        ].cpu?.topProcesses.slice(0, 5)" :key="process.pid"
                          class="border-b border-subtle/50 pb-2 last:border-b-0">
                          <div class="flex items-center justify-between mb-1">
                            <span class="text-primary font-mono text-xs truncate flex-1 mr-2" :title="process.command">
                              {{ process.command }}
                            </span>
                            <span class="text-secondary font-mono text-xs">
                              {{ process.cpu }}
                            </span>
                          </div>
                          <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[10px]">
                            <div class="text-text-muted">PID:</div>
                            <div class="text-text-primary font-mono">
                              {{ process.pid }}
                            </div>
                            <div class="text-text-muted">Memory:</div>
                            <div class="text-text-primary font-mono text-right">
                              {{ process.memory }}
                            </div>
                            <div class="text-text-muted">Mem Usage:</div>
                            <div class="text-text-primary font-mono text-right">
                              {{ process.memoryPercent }}
                            </div>
                          </div>
                        </div>
                      </div>
                      <!-- Tooltip arrow -->
                      <div class="absolute top-full right-4 -mt-1">
                        <div class="w-2 h-2 bg-bg-secondary border-r border-b border-subtle transform rotate-45"></div>
                      </div>
                    </div>
                  </div>
                </div>
                <div v-else class="text-text-muted">CPU: N/A</div>

                <!-- Memory Usage -->
                <div v-if="sessionStatus[session.id].memory"
                  class="group relative flex items-center cursor-help text-text-secondary hover:text-primary transition-colors duration-fast py-1 -my-1 px-2 -mx-2 rounded hover:bg-bg-tertiary/50">
                  <div class="flex items-center space-x-1">
                    <span>Mem:</span>
                    <span :class="{
                      'text-error': sessionStatus[session.id].memory && sessionStatus[session.id].memory!.usage.includes('%') && parseFloat(sessionStatus[session.id].memory!.usage!.match(/[\d.]+/)?.[0] || '0') > 90,
                      'text-warning': sessionStatus[session.id].memory && sessionStatus[session.id].memory!.usage.includes('%') && parseFloat(sessionStatus[session.id].memory!.usage!.match(/[\d.]+/)?.[0] || '0') > 70,
                      'status-online': sessionStatus[session.id].memory && (!sessionStatus[session.id].memory!.usage.includes('%') || parseFloat(sessionStatus[session.id].memory!.usage!.match(/[\d.]+/)?.[0] || '0') <= 70)
                    }">
                      {{ sessionStatus[session.id].memory?.usage || "N/A" }}
                    </span>
                  </div>
                  <!-- Memory Tooltip with top 5 processes -->
                  <div
                    class="absolute bottom-full right-0 mb-2 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-tooltip">
                    <div
                      class="bg-bg-elevated rounded-lg shadow-md p-3 text-xs whitespace-nowrap max-w-[350px] max-h-[400px] overflow-y-auto border border-border-primary">
                      <div class="text-text-primary font-medium mb-2 pb-1 border-b border-subtle">
                        Top 5 Memory Processes
                      </div>
                      <div class="space-y-2">
                        <div v-for="process in sessionStatus[
                          session.id
                        ].memory?.topProcesses.slice(0, 5)" :key="process.pid"
                          class="border-b border-subtle/50 pb-2 last:border-b-0">
                          <div class="flex items-center justify-between mb-1">
                            <span class="text-primary font-mono text-xs truncate flex-1 mr-2" :title="process.command">
                              {{ process.command }}
                            </span>
                            <span class="text-accent font-mono text-xs">
                              {{ process.memory }}
                            </span>
                          </div>
                          <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[10px]">
                            <div class="text-text-muted">PID:</div>
                            <div class="text-text-primary font-mono">
                              {{ process.pid }}
                            </div>
                            <div class="text-text-muted">CPU:</div>
                            <div class="text-text-primary font-mono text-right">
                              {{ process.cpu }}
                            </div>
                            <div class="text-text-muted">Mem Usage:</div>
                            <div class="text-text-primary font-mono text-right">
                              {{ process.memoryPercent }}
                            </div>
                          </div>
                        </div>
                      </div>
                      <!-- Tooltip arrow -->
                      <div class="absolute top-full right-4 -mt-1">
                        <div class="w-2 h-2 bg-bg-secondary border-r border-b border-subtle transform rotate-45"></div>
                      </div>
                    </div>
                  </div>
                </div>
                <div v-else class="text-text-muted">Mem: N/A</div>

                <!-- IP -->
                <div class="text-text-secondary truncate font-mono" :title="sessionStatus[session.id].ip">
                  IP: {{ sessionStatus[session.id].ip }}
                </div>
              </template>
              <div v-else class="text-text-muted italic">
                {{ t("app.loadingStatus") }}
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="flex-1 flex items-center justify-center text-text-muted" v-else>
        {{ t("app.selectConnectionToStart") }}
      </div>
    </main>

    <ConnectionModal :show="showConnectionModal" :connectionToEdit="editingConnection"
      @close="showConnectionModal = false" @save="handleSaveConnection" />
    <SettingsModal :show="showSettingsModal" @close="showSettingsModal = false" />

    <NotificationModal v-if="notificationStore.show" :show="notificationStore.show" :type="notificationStore.type"
      :title="notificationStore.title" :message="notificationStore.message" :duration="notificationStore.duration"
      @close="notificationStore.close()" />
  </div>
</template>
