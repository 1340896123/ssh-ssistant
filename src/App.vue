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

interface MountInfo {
  filesystem: string;
  size: string;
  used: string;
  avail: string;
  percent: string;
  mount: string;
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
  const startTime = Date.now();

  // 优化后的 Shell 命令
  // 1. 使用 LC_ALL=C 防止数字格式化问题
  // 2. CPU 计算使用纯 awk，不依赖 bc
  // 3. ps 命令明确输出格式，便于解析
  const command = `
    export LC_ALL=C;
    echo "UPTIME_START";
    (uptime -p 2>/dev/null || uptime 2>/dev/null);
    echo "UPTIME_END";
    
    echo "MOUNTS_START";
    df -Ph 2>/dev/null | awk 'NR>1 {print $1 "|" $2 "|" $3 "|" $4 "|" $5 "|" $6}';
    echo "MOUNTS_END";
    
    echo "IP_START";
    (hostname -I 2>/dev/null || echo 'n/a');
    echo "IP_END";
    
    echo "CPU_START";
    CPU1=$(grep '^cpu ' /proc/stat 2>/dev/null);
    sleep 0.1;
    CPU2=$(grep '^cpu ' /proc/stat 2>/dev/null);
    if [ -n "$CPU1" ] && [ -n "$CPU2" ]; then
        echo "$CPU1 $CPU2" | awk '{
            u1=$2+$4+$5; t1=$2+$4+$5+$6;
            u2=$8+$10+$11; t2=$8+$10+$11+$12;
            if (t2-t1 > 0) printf "%.1f", (u2-u1) * 100 / (t2-t1); else print "0"
        }';
    else
        top -bn1 2>/dev/null | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//' | sed 's/%id,.*//' || echo "0";
    fi;
    echo "";
    echo "CPU_END";
    
    echo "MEMORY_START";
    awk '/MemTotal:/ {total=$2} /MemAvailable:/ {avail=$2} END {if(total>0){used=total-avail; printf "%.1f%%|%.1fGB|%.1fGB|%.1fGB", (used/total)*100, total/1024/1024, used/1024/1024, avail/1024/1024} else {print "0%|0|0|0"}}' /proc/meminfo 2>/dev/null;
    echo ""; 
    echo "MEMORY_END";
    
    echo "PROCESSES_START";
    ps aux --sort=-%cpu --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\\n", $2, $11, $3"%", $4"%", $6/1024}';
    echo "PROCESSES_END";
    
    echo "MEMORY_PROCESSES_START";
    ps aux --sort=-%mem --no-headers 2>/dev/null | head -5 | awk '{printf "%s|%s|%s|%s|%.1fMB\\n", $2, $11, $3"%", $4"%", $6/1024}';
    echo "MEMORY_PROCESSES_END";
  `
    .replace(/[\r\n]+/g, " ")
    .trim();

  try {
    const result = await invoke<string>("exec_command", { id, command });

    // 1. 正则提取各部分数据块
    const extractBlock = (marker: string) => {
      const regex = new RegExp(
        `${marker}_START\\s*([\\s\\S]*?)\\s*${marker}_END`
      );
      const match = result.match(regex);
      return match ? match[1].trim() : "";
    };

    const blocks = {
      uptime: extractBlock("UPTIME"),
      mounts: extractBlock("MOUNTS"),
      ip: extractBlock("IP"),
      cpu: extractBlock("CPU"),
      memory: extractBlock("MEMORY"),
      procCpu: extractBlock("PROCESSES"),
      procMem: extractBlock("MEMORY_PROCESSES"),
    };

    // 2. 解析基础信息
    const uptime = blocks.uptime || "N/A";
    const ip = blocks.ip.split(" ")[0] || "N/A";

    // 3. 解析 CPU 使用率
    let cpuUsage = "0%";
    const cpuVal = parseFloat(blocks.cpu);
    if (!isNaN(cpuVal)) {
      cpuUsage = `${Math.min(Math.max(cpuVal, 0), 100).toFixed(1)}%`;
    }

    // 4. 解析内存 (Shell脚本输出了：百分比|Total|Used|Avail)
    const memParts = blocks.memory.split("|");
    const memoryInfo = {
      usage: memParts[0] || "0%",
      total: memParts[1] ? `${memParts[1]}GB` : "N/A",
      used: memParts[2] ? `${memParts[2]}GB` : "N/A",
      available: memParts[3] ? `${memParts[3]}GB` : "N/A",
    };

    // 5. 解析挂载点
    const mounts = parseTable<MountInfo>(
      blocks.mounts,
      (p) => ({
        filesystem: p[0],
        size: p[1],
        used: p[2],
        avail: p[3],
        percent: p[4],
        mount: p[5],
      }),
      6
    );

    // 找到根目录或第一个挂载点作为主磁盘显示
    const rootDisk = mounts.find((m) => m.mount === "/") || mounts[0] || null;

    // 6. 解析进程列表 (PID|Command|CPU%|Mem%|MemVal)
    const processMapper = (p: string[]): ProcessInfo => ({
      pid: p[0],
      command: p[1],
      cpu: p[2],
      memory: p[3],
      memoryPercent: p[4],
    });

    const cpuTopProcesses = parseTable<ProcessInfo>(
      blocks.procCpu,
      processMapper,
      5
    );
    const memoryTopProcesses = parseTable<ProcessInfo>(
      blocks.procMem,
      processMapper,
      5
    );

    // 7. 更新状态
    sessionStatus.value = {
      ...sessionStatus.value,
      [id]: {
        uptime,
        disk: rootDisk,
        mounts,
        ip,
        cpu: {
          usage: cpuUsage,
          topProcesses: cpuTopProcesses,
        },
        memory: {
          ...memoryInfo,
          topProcesses: memoryTopProcesses,
        },
      },
    };
  } catch (error: any) {
    const duration = Date.now() - startTime;
    console.error(
      `System monitoring failed for ${id} after ${duration}ms:`,
      error
    );

    // 错误状态处理
    sessionStatus.value = {
      ...sessionStatus.value,
      [id]: {
        // 保留旧数据或重置为 Error，视需求而定。
        // 这里仅更新关键报错字段，防止UI完全空白
        ...sessionStatus.value?.[id],
        uptime: "Connection Error",
        cpu: { usage: "N/A", topProcesses: [] },
        memory: {
          usage: "N/A",
          total: "N/A",
          used: "N/A",
          available: "N/A",
          topProcesses: [],
        },
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

const parseTable = <T>(
  raw: string,
  mapper: (parts: string[]) => T | null,
  minColumns: number = 1
): T[] => {
  if (!raw) return [];
  return raw
    .trim()
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => line.split("|"))
    .filter((parts) => parts.length >= minColumns)
    .map(mapper)
    .filter((item): item is T => item !== null);
};
function startResize(target: "file" | "ai" | "sidebar") {
  isResizing.value = target;
  document.body.style.cursor = "col-resize";
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

                <!-- CPU Usage -->
                <div v-if="sessionStatus[session.id].cpu"
                  class="group relative flex items-center cursor-help text-gray-400 hover:text-gray-200 py-1 -my-1 px-2 -mx-2">
                  <div class="flex items-center space-x-1">
                    <span>CPU:</span>
                    <span :class="{ 
                      'text-red-400': sessionStatus[session.id].cpu && parseFloat(sessionStatus[session.id].cpu!.usage) > 90,
                      'text-yellow-400': sessionStatus[session.id].cpu && parseFloat(sessionStatus[session.id].cpu!.usage) > 70,
                      'text-green-400': sessionStatus[session.id].cpu && parseFloat(sessionStatus[session.id].cpu!.usage) <= 70 
                    }">
                      {{ sessionStatus[session.id].cpu?.usage || 'N/A' }}
                    </span>
                  </div>
                  <!-- CPU Tooltip with top 5 processes -->
                  <div class="absolute bottom-full right-0 mb-2 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-[100]">
                    <div class="bg-gray-800 border border-gray-600 rounded-lg shadow-xl p-3 text-xs whitespace-nowrap max-w-[350px] max-h-[400px] overflow-y-auto">
                      <div class="text-gray-300 font-medium mb-2 pb-1 border-b border-gray-600">
                        Top 5 CPU Processes
                      </div>
                      <div class="space-y-2">
                        <div v-for="process in sessionStatus[session.id].cpu?.topProcesses.slice(0, 5)" :key="process.pid" 
                          class="border-b border-gray-700/50 pb-2 last:border-b-0">
                          <div class="flex items-center justify-between mb-1">
                            <span class="text-blue-400 font-mono text-xs truncate flex-1 mr-2" :title="process.command">
                              {{ process.command }}
                            </span>
                            <span class="text-orange-400 font-mono text-xs">
                              {{ process.cpu }}
                            </span>
                          </div>
                          <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[10px]">
                            <div class="text-gray-500">PID:</div>
                            <div class="text-gray-300 font-mono">{{ process.pid }}</div>
                            <div class="text-gray-500">Memory:</div>
                            <div class="text-gray-300 font-mono text-right">{{ process.memory }}</div>
                            <div class="text-gray-500">Mem Usage:</div>
                            <div class="text-gray-300 font-mono text-right">{{ process.memoryPercent }}</div>
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
                  CPU: N/A
                </div>

                <!-- Memory Usage -->
                <div v-if="sessionStatus[session.id].memory"
                  class="group relative flex items-center cursor-help text-gray-400 hover:text-gray-200 py-1 -my-1 px-2 -mx-2">
                  <div class="flex items-center space-x-1">
                    <span>Mem:</span>
                    <span :class="{ 
                      'text-red-400': sessionStatus[session.id].memory && sessionStatus[session.id].memory!.usage.includes('%') && parseFloat(sessionStatus[session.id].memory!.usage!.match(/[\d.]+/)?.[0] || '0') > 90,
                      'text-yellow-400': sessionStatus[session.id].memory && sessionStatus[session.id].memory!.usage.includes('%') && parseFloat(sessionStatus[session.id].memory!.usage!.match(/[\d.]+/)?.[0] || '0') > 70,
                      'text-green-400': sessionStatus[session.id].memory && (!sessionStatus[session.id].memory!.usage.includes('%') || parseFloat(sessionStatus[session.id].memory!.usage!.match(/[\d.]+/)?.[0] || '0') <= 70)
                    }">
                      {{ sessionStatus[session.id].memory?.usage || 'N/A' }}
                    </span>
                  </div>
                  <!-- Memory Tooltip with top 5 processes -->
                  <div class="absolute bottom-full right-0 mb-2 opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 z-[100]">
                    <div class="bg-gray-800 border border-gray-600 rounded-lg shadow-xl p-3 text-xs whitespace-nowrap max-w-[350px] max-h-[400px] overflow-y-auto">
                      <div class="text-gray-300 font-medium mb-2 pb-1 border-b border-gray-600">
                        Top 5 Memory Processes
                      </div>
                      <div class="space-y-2">
                        <div v-for="process in sessionStatus[session.id].memory?.topProcesses.slice(0, 5)" :key="process.pid" 
                          class="border-b border-gray-700/50 pb-2 last:border-b-0">
                          <div class="flex items-center justify-between mb-1">
                            <span class="text-blue-400 font-mono text-xs truncate flex-1 mr-2" :title="process.command">
                              {{ process.command }}
                            </span>
                            <span class="text-purple-400 font-mono text-xs">
                              {{ process.memory }}
                            </span>
                          </div>
                          <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[10px]">
                            <div class="text-gray-500">PID:</div>
                            <div class="text-gray-300 font-mono">{{ process.pid }}</div>
                            <div class="text-gray-500">CPU:</div>
                            <div class="text-gray-300 font-mono text-right">{{ process.cpu }}</div>
                            <div class="text-gray-500">Mem Usage:</div>
                            <div class="text-gray-300 font-mono text-right">{{ process.memoryPercent }}</div>
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
                  Mem: N/A
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
