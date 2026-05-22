<script setup lang="ts">
import { computed, ref } from "vue";
import { useSessionStore } from "../stores/sessions";
import { useI18n } from "../composables/useI18n";
import {
  Circle,
  Loader2,
  Monitor,
  Bot,
  FolderOpen,
  Power,
  RotateCcw,
  X,
  Rows3,
} from "lucide-vue-next";

const emit = defineEmits<{
  (e: "new-connection"): void;
}>();

const sessionStore = useSessionStore();
const { t } = useI18n();
const showBulkActions = ref(false);

const sessions = computed(() => sessionStore.sessions);
const activeSession = computed(() => sessionStore.activeSession);
const connectedCount = computed(
  () => sessions.value.filter((session) => session.status === "connected").length
);
const disconnectedCount = computed(
  () => sessions.value.filter((session) => session.status === "disconnected").length
);
const hasDisconnectedSessions = computed(() => disconnectedCount.value > 0);
const hasConnectedSessions = computed(() => connectedCount.value > 0);

function closeOtherSessions(sessionId: string) {
  sessionStore.sessions
    .filter((session) => session.id !== sessionId)
    .forEach((session) => sessionStore.closeSession(session.id));
}

function closeDisconnectedSessions() {
  sessionStore.sessions
    .filter((session) => session.status === "disconnected")
    .forEach((session) => sessionStore.closeSession(session.id));
}

function closeAllSessions() {
  [...sessionStore.sessions].forEach((session) =>
    sessionStore.closeSession(session.id)
  );
}

function disconnectAllSessions() {
  sessionStore.sessions
    .filter((session) => session.status === "connected")
    .forEach((session) => sessionStore.disconnectSession(session.id));
}

function reconnectDisconnectedSessions() {
  sessionStore.sessions
    .filter((session) => session.status === "disconnected")
    .forEach((session) => sessionStore.reconnectSession(session.id));
}

function getActiveTabLabel(session: {
  activeTab: "terminal" | "files" | "ai";
}) {
  if (session.activeTab === "files") return t("sessionsPane.files");
  if (session.activeTab === "ai") return t("sessionsPane.ai");
  return t("sessionsPane.shell");
}

function getActiveTabIcon(session: {
  activeTab: "terminal" | "files" | "ai";
}) {
  if (session.activeTab === "files") return FolderOpen;
  if (session.activeTab === "ai") return Bot;
  return Monitor;
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
  <div class="flex h-full flex-col bg-bg-secondary">
    <div class="border-b border-border-primary px-4 py-3">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="text-sm font-semibold text-text-primary">
            {{ t("sessionsPane.title") }}
          </div>
          <div class="mt-1 text-xs text-text-secondary">
            {{ t("sessionsPane.subtitle") }}
          </div>
        </div>
        <button
          class="rounded-md border border-border-primary bg-bg-tertiary px-3 py-2 text-xs text-text-primary transition-colors hover:bg-bg-elevated"
          @click="emit('new-connection')"
        >
          {{ t("app.newConnection") }}
        </button>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
        <span
          class="rounded-full border border-border-primary bg-bg-tertiary px-2.5 py-1 text-text-secondary"
        >
          {{ t("sessionsPane.summary.total") }} {{ sessions.length }}
        </span>
        <span
          class="rounded-full border border-success/30 bg-success/10 px-2.5 py-1 text-success"
        >
          {{ t("sessionsPane.summary.connected") }} {{ connectedCount }}
        </span>
        <span
          class="rounded-full border border-warning/30 bg-warning/10 px-2.5 py-1 text-warning"
        >
          {{ t("sessionsPane.summary.disconnected") }} {{ disconnectedCount }}
        </span>
      </div>
    </div>

    <div class="border-b border-border-primary px-4 py-3">
      <div class="relative">
        <button
          class="flex h-9 w-full items-center justify-between rounded-md border border-border-primary bg-bg-tertiary px-3 text-sm text-text-primary transition-colors hover:bg-bg-elevated"
          @click="showBulkActions = !showBulkActions"
        >
          <div class="flex items-center gap-2">
            <Rows3 class="h-4 w-4 text-text-secondary" />
            <span>{{ t("sessionsPane.bulk.title") }}</span>
          </div>
          <span class="text-xs text-text-secondary">
            {{ t("sessionsPane.bulk.hint") }}
          </span>
        </button>

        <div
          v-if="showBulkActions"
          class="absolute left-0 top-full z-20 mt-2 w-full rounded-xl border border-border-primary bg-bg-elevated p-2 shadow-lg"
        >
          <button
            v-if="hasConnectedSessions"
            class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-text-primary transition-colors hover:bg-bg-tertiary"
            @click="
              disconnectAllSessions();
              showBulkActions = false;
            "
          >
            <Power class="h-4 w-4 text-warning" />
            <span>{{ t("sessionsPane.bulk.disconnectConnected") }}</span>
          </button>
          <button
            v-if="hasDisconnectedSessions"
            class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-text-primary transition-colors hover:bg-bg-tertiary"
            @click="
              reconnectDisconnectedSessions();
              showBulkActions = false;
            "
          >
            <RotateCcw class="h-4 w-4 text-success" />
            <span>{{ t("sessionsPane.bulk.reconnectDisconnected") }}</span>
          </button>
          <button
            v-if="hasDisconnectedSessions"
            class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-text-primary transition-colors hover:bg-bg-tertiary"
            @click="
              closeDisconnectedSessions();
              showBulkActions = false;
            "
          >
            <X class="h-4 w-4 text-warning" />
            <span>{{ t("sessionsPane.bulk.closeDisconnected") }}</span>
          </button>
          <button
            class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-text-primary transition-colors hover:bg-bg-tertiary"
            @click="
              closeAllSessions();
              showBulkActions = false;
            "
          >
            <X class="h-4 w-4 text-error" />
            <span>{{ t("sessionsPane.bulk.closeAll") }}</span>
          </button>
        </div>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-3 py-3">
      <div v-if="sessions.length === 0" class="rounded-xl border border-dashed border-border-primary bg-bg-primary px-4 py-6 text-center">
        <div class="text-sm font-medium text-text-primary">
          {{ t("sessionsPane.empty") }}
        </div>
        <div class="mt-1 text-xs text-text-secondary">
          {{ t("sessionsPane.emptyHint") }}
        </div>
      </div>

      <div v-else class="space-y-2">
        <button
          v-for="session in sessions"
          :key="session.id"
          class="group w-full rounded-xl border px-3 py-3 text-left transition-colors"
          :class="
            session.id === sessionStore.activeSessionId
              ? 'border-accent bg-bg-elevated'
              : 'border-border-primary bg-bg-primary hover:bg-bg-elevated'
          "
          @click="sessionStore.setActiveSession(session.id)"
        >
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <Loader2
                  v-if="session.status === 'connecting'"
                  class="h-3.5 w-3.5 animate-spin text-warning"
                />
                <Circle
                  v-else-if="session.status === 'connected'"
                  class="h-3.5 w-3.5 fill-current text-success"
                />
                <Circle
                  v-else
                  class="h-3.5 w-3.5 fill-current text-error"
                />
                <span class="truncate text-sm font-medium text-text-primary">
                  {{ session.assetName }}
                </span>
                <span
                  v-if="session.id === activeSession?.id"
                  class="rounded-full bg-accent/10 px-2 py-0.5 text-[11px] text-accent"
                >
                  {{ t("sessionsPane.current") }}
                </span>
              </div>

              <div class="mt-2 flex flex-wrap items-center gap-2 text-[11px] text-text-secondary">
                <span class="rounded-full border border-border-primary px-2 py-0.5">
                  {{ session.os || t("sessionsPane.unknownOs") }}
                </span>
                <span class="rounded-full border border-border-primary px-2 py-0.5">
                  {{ getActiveTabLabel(session) }}
                </span>
                <span class="rounded-full border border-border-primary px-2 py-0.5">
                  {{ formatDuration(session.connectedAt) }}
                </span>
              </div>

              <div class="mt-2 flex items-center gap-1.5 text-xs text-text-secondary">
                <component
                  :is="getActiveTabIcon(session)"
                  class="h-3.5 w-3.5 shrink-0"
                />
                <span class="truncate">
                  {{
                    session.currentPath && session.currentPath !== "."
                      ? session.currentPath
                      : t("sessionsPane.noPath")
                  }}
                </span>
              </div>
            </div>

            <div class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <button
                class="rounded-md p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-text-primary"
                :title="t('sessionsPane.closeOther')"
                @click.stop="closeOtherSessions(session.id)"
              >
                <Rows3 class="h-3.5 w-3.5" />
              </button>
              <button
                v-if="session.status === 'connected'"
                class="rounded-md p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-warning"
                :title="t('sessionsPane.disconnect')"
                @click.stop="sessionStore.disconnectSession(session.id)"
              >
                <Power class="h-3.5 w-3.5" />
              </button>
              <button
                v-else
                class="rounded-md p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-success"
                :title="t('sessionsPane.reconnect')"
                @click.stop="sessionStore.reconnectSession(session.id)"
              >
                <RotateCcw class="h-3.5 w-3.5" />
              </button>
              <button
                class="rounded-md p-1.5 text-text-secondary hover:bg-bg-secondary hover:text-error"
                :title="t('sessionsPane.close')"
                @click.stop="sessionStore.closeSession(session.id)"
              >
                <X class="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </button>
      </div>
    </div>
  </div>
</template>
