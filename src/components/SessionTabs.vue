<script setup lang="ts">
import { computed, ref } from 'vue';
import { useSessionStore } from '../stores/sessions';
import { X, Loader2, Circle, TerminalSquare, FolderOpen, Bot, MoreHorizontal, Power, RotateCcw, Rows3 } from 'lucide-vue-next';

const sessionStore = useSessionStore();
const showBulkActions = ref(false);

const sessions = computed(() => sessionStore.sessions);
const activeSession = computed(() => sessionStore.activeSession);
const connectedCount = computed(() => sessions.value.filter(session => session.status === 'connected').length);
const disconnectedCount = computed(() => sessions.value.filter(session => session.status === 'disconnected').length);
const hasDisconnectedSessions = computed(() => disconnectedCount.value > 0);
const hasConnectedSessions = computed(() => connectedCount.value > 0);

function closeOtherSessions(sessionId: string) {
  sessionStore.sessions
    .filter(session => session.id !== sessionId)
    .forEach(session => sessionStore.closeSession(session.id));
}

function closeDisconnectedSessions() {
  sessionStore.sessions
    .filter(session => session.status === 'disconnected')
    .forEach(session => sessionStore.closeSession(session.id));
}

function closeAllSessions() {
  [...sessionStore.sessions].forEach(session => sessionStore.closeSession(session.id));
}

function disconnectAllSessions() {
  sessionStore.sessions
    .filter(session => session.status === 'connected')
    .forEach(session => sessionStore.disconnectSession(session.id));
}

function reconnectDisconnectedSessions() {
  sessionStore.sessions
    .filter(session => session.status === 'disconnected')
    .forEach(session => sessionStore.reconnectSession(session.id));
}

function getActiveTabLabel(session: { activeTab: 'terminal' | 'files' | 'ai' }) {
  if (session.activeTab === 'files') return 'Files';
  if (session.activeTab === 'ai') return 'AI';
  return 'Shell';
}

function getActiveTabIcon(session: { activeTab: 'terminal' | 'files' | 'ai' }) {
  if (session.activeTab === 'files') return FolderOpen;
  if (session.activeTab === 'ai') return Bot;
  return TerminalSquare;
}

function formatDuration(timestamp: number) {
  const minutes = Math.max(1, Math.floor((Date.now() - timestamp) / 60000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}
</script>

<template>
  <div class="flex items-center h-full min-w-0">
    <div class="flex items-center h-full px-2 border-r border-border-primary bg-bg-secondary/70 shrink-0 gap-2">
      <div class="hidden md:flex items-center gap-2 text-xs text-text-secondary">
        <span>{{ sessions.length }} 标签</span>
        <span class="w-1 h-1 rounded-full bg-border-primary"></span>
        <span>{{ connectedCount }} 在线</span>
        <span v-if="disconnectedCount > 0" class="text-warning">{{ disconnectedCount }} 离线</span>
      </div>

      <div class="relative">
        <button
          class="h-8 px-2 rounded border border-border-primary bg-bg-tertiary text-text-secondary hover:text-text-primary hover:bg-bg-elevated transition-all flex items-center gap-1.5 text-xs"
          @click="showBulkActions = !showBulkActions"
        >
          <Rows3 class="w-3.5 h-3.5" />
          <span class="hidden sm:inline">批量</span>
          <MoreHorizontal class="w-3.5 h-3.5" />
        </button>

        <div v-if="showBulkActions" class="absolute left-0 top-full mt-2 w-44 rounded-lg border border-border-primary bg-bg-elevated shadow-lg z-20 p-1.5">
          <button v-if="hasConnectedSessions" class="w-full px-2.5 py-2 rounded text-left text-xs text-text-primary hover:bg-bg-tertiary flex items-center gap-2" @click="disconnectAllSessions(); showBulkActions = false">
            <Power class="w-3.5 h-3.5 text-warning" />
            <span>断开在线会话</span>
          </button>
          <button v-if="hasDisconnectedSessions" class="w-full px-2.5 py-2 rounded text-left text-xs text-text-primary hover:bg-bg-tertiary flex items-center gap-2" @click="reconnectDisconnectedSessions(); showBulkActions = false">
            <RotateCcw class="w-3.5 h-3.5 text-success" />
            <span>重连离线会话</span>
          </button>
          <button v-if="hasDisconnectedSessions" class="w-full px-2.5 py-2 rounded text-left text-xs text-text-primary hover:bg-bg-tertiary flex items-center gap-2" @click="closeDisconnectedSessions(); showBulkActions = false">
            <X class="w-3.5 h-3.5 text-warning" />
            <span>关闭离线标签</span>
          </button>
          <button class="w-full px-2.5 py-2 rounded text-left text-xs text-text-primary hover:bg-bg-tertiary flex items-center gap-2" @click="closeAllSessions(); showBulkActions = false">
            <X class="w-3.5 h-3.5 text-error" />
            <span>关闭全部标签</span>
          </button>
        </div>
      </div>
    </div>

    <div class="flex items-center h-full space-x-1 overflow-x-auto scrollbar-hide min-w-0 flex-1 px-1">
      <div v-for="session in sessions" :key="session.id"
        class="group flex items-center h-full px-3 min-w-[180px] max-w-[260px] bg-bg-tertiary border-r border-border-primary cursor-pointer hover:bg-bg-elevated select-none relative"
        :class="{ 'bg-bg-elevated border-t-2 border-t-accent': session.id === sessionStore.activeSessionId }"
        @click="sessionStore.setActiveSession(session.id)">

        <div class="mr-2 flex items-center justify-center shrink-0">
          <Loader2 v-if="session.status === 'connecting'" class="w-3 h-3 text-warning animate-spin" />
          <Circle v-else-if="session.status === 'connected'" class="w-3 h-3 text-success fill-current" />
          <Circle v-else class="w-3 h-3 text-error fill-current" />
        </div>

        <component :is="getActiveTabIcon(session)" class="w-3.5 h-3.5 mr-2 text-text-secondary shrink-0" />

        <div class="min-w-0 flex-1 py-2">
          <div class="flex items-center gap-2">
            <span class="text-xs text-text-primary truncate">{{ session.connectionName }}</span>
            <span class="rounded-full border border-border-primary px-1.5 py-0.5 text-[10px] text-text-secondary shrink-0">{{ getActiveTabLabel(session) }}</span>
          </div>
          <div class="mt-0.5 flex items-center gap-2 text-[10px] text-text-secondary truncate">
            <span>{{ session.os || '未知系统' }}</span>
            <span class="w-1 h-1 rounded-full bg-border-primary"></span>
            <span>{{ formatDuration(session.connectedAt) }}</span>
            <span v-if="session.id === activeSession?.id" class="text-accent">当前</span>
          </div>
        </div>

        <div class="ml-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
          <button
            class="text-text-secondary hover:text-text-primary p-1 rounded-full hover:bg-bg-secondary"
            title="关闭其他标签"
            @click.stop="closeOtherSessions(session.id)"
          >
            <Rows3 class="w-3 h-3" />
          </button>
          <button @click.stop="sessionStore.closeSession(session.id)"
            class="text-text-secondary hover:text-text-primary p-1 rounded-full hover:bg-bg-secondary">
            <X class="w-3 h-3" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
