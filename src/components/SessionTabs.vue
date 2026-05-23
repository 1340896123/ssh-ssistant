<script setup lang="ts">
import { computed } from "vue";
import { useSessionStore } from "../stores/sessions";
import { Circle, Loader2, Rows3, X } from "lucide-vue-next";
import { useI18n } from "../composables/useI18n";

const sessionStore = useSessionStore();
const { t } = useI18n();

const sessions = computed(() => sessionStore.sessions);
const activeSession = computed(() => sessionStore.activeSession);

function closeOtherSessions(sessionId: string) {
  sessionStore.sessions
    .filter((session) => session.id !== sessionId)
    .forEach((session) => sessionStore.closeSession(session.id));
}

function activateSession(sessionId: string) {
  sessionStore.setActiveSession(sessionId);
}

function handleSessionKeydown(event: KeyboardEvent, sessionId: string) {
  if (event.key !== "Enter" && event.key !== " ") return;
  event.preventDefault();
  activateSession(sessionId);
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
  <div class="flex h-full min-w-0 items-center">
    <div class="flex items-center gap-2 border-r border-border-primary px-3 text-xs text-text-secondary">
      <span>{{ sessions.length }}</span>
      <span>{{ t("workbench.sessionTabsLabel") }}</span>
    </div>

    <div class="scrollbar-hide flex h-full min-w-0 flex-1 items-center gap-1 overflow-x-auto px-2">
      <div
        v-for="session in sessions"
        :key="session.id"
        role="button"
        tabindex="0"
        class="group flex h-[34px] min-w-[180px] max-w-[240px] items-center gap-2 rounded-lg border px-3 text-left transition-colors focus:outline-none focus:ring-1 focus:ring-accent focus:ring-offset-0 focus:ring-offset-bg-secondary"
        :class="
          session.id === sessionStore.activeSessionId
            ? 'border-accent bg-bg-elevated'
            : 'border-transparent bg-bg-secondary hover:border-border-primary hover:bg-bg-elevated'
        "
        @click="activateSession(session.id)"
        @keydown="handleSessionKeydown($event, session.id)"
      >
        <Loader2
          v-if="session.status === 'connecting'"
          class="h-3.5 w-3.5 shrink-0 animate-spin text-warning"
        />
        <Circle
          v-else-if="session.status === 'connected'"
          class="h-3.5 w-3.5 shrink-0 fill-current text-success"
        />
        <Circle
          v-else
          class="h-3.5 w-3.5 shrink-0 fill-current text-error"
        />

        <div class="min-w-0 flex-1">
          <div class="truncate text-sm text-text-primary">
            {{ session.assetName }}
          </div>
        </div>

        <div class="hidden shrink-0 text-[11px] text-text-secondary lg:block">
          {{ session.os || t("sessionsPane.unknownOs") }}
        </div>

        <div class="hidden shrink-0 text-[11px] text-text-secondary md:block">
          {{ formatDuration(session.connectedAt) }}
        </div>

        <span
          v-if="session.id === activeSession?.id"
          class="hidden rounded-full bg-accent/10 px-2 py-0.5 text-[11px] text-accent xl:inline-flex"
        >
          {{ t("sessionsPane.current") }}
        </span>

        <div class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
          <button
            class="rounded-md p-1 text-text-secondary hover:bg-bg-primary hover:text-text-primary"
            :title="t('sessionsPane.closeOther')"
            @click.stop="closeOtherSessions(session.id)"
          >
            <Rows3 class="h-3.5 w-3.5" />
          </button>
          <button
            class="rounded-md p-1 text-text-secondary hover:bg-bg-primary hover:text-error"
            :title="t('sessionsPane.close')"
            @click.stop="sessionStore.closeSession(session.id)"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
