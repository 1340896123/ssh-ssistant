<script setup lang="ts">
import { useConnectionStore } from '../stores/connections';
import { useSessionStore } from '../stores/sessions';
import { useI18n } from '../composables/useI18n';
import { onMounted, computed, ref, onUnmounted } from 'vue';
import {
  FolderPlus,
  ChevronRight,
  ChevronDown,
  FolderOpen,
  Folder,
  Monitor,
  Pencil,
  Trash2,
  Copy,
  Cable,
  Search,
  Plus,
  FolderTree,
  FileDown,
  Star,
  History,
  TriangleAlert,
  CircleCheck,
  Filter,
  Network,
} from 'lucide-vue-next';
import ConnectionTreeItem from './ConnectionTreeItem.vue';
import type { Connection, ConnectionGroup, ConnectionHistoryEntry, ConnectionHistorySource } from '../types';
import { listen } from '@tauri-apps/api/event';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { ask } from '@tauri-apps/plugin-dialog';
import { useNotificationStore } from '../stores/notifications';
import ContextMenu, { type MenuItem } from './ContextMenu.vue';

type HistoryFilter = 'all' | 'success' | 'failed';

type SearchResultItem =
  | { kind: 'connection'; item: Connection; source: ConnectionHistorySource | 'favorite'; matchText: string }
  | { kind: 'group'; item: ConnectionGroup; source: 'tree'; matchText: string };

const connectionStore = useConnectionStore();
const sessionStore = useSessionStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();
const emit = defineEmits(['edit', 'tunnels']);

const menuVisible = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuItems = ref<MenuItem[]>([]);
const contextItem = ref<Connection | ConnectionGroup | null>(null);
const isRootExpanded = ref(true);
const isHistoryExpanded = ref(true);
const isFavoritesExpanded = ref(true);
const containerRef = ref<HTMLElement | null>(null);
let unlistenDrop: (() => void) | null = null;
let unlistenDragEnter: (() => void) | null = null;
let unlistenDragLeave: (() => void) | null = null;
const isDragOver = ref(false);
const searchQuery = ref('');
const historyFilter = ref<HistoryFilter>('all');

function closeMenu() {
  menuVisible.value = false;
  contextItem.value = null;
}

async function handleMenuAction(action: string) {
  if (!contextItem.value && action !== 'newConnection' && action !== 'newGroup') {
    return;
  }

  const item = contextItem.value;

  switch (action) {
    case 'connect':
      if (item && !('children' in item)) connect(item as Connection, 'tree');
      break;
    case 'edit':
      if (item && !('children' in item)) handleEdit(item as Connection);
      break;
    case 'copy':
      if (item && !('children' in item)) {
        const conn = item as Connection;
        const { id, ...rest } = conn;
        const newName = `${conn.name} (Copy)`;
        await connectionStore.addConnection({ ...rest, name: newName });
        notificationStore.success(t('connections.copied', { name: newName }) || `Copied to ${newName}`);
      }
      break;
    case 'delete':
      if (item && !('children' in item)) handleDelete(item as Connection);
      break;
    case 'tunnels':
      if (item && !('children' in item)) emit('tunnels', item as Connection);
      break;
    case 'favorite':
      if (item && !('children' in item) && item.id !== undefined) {
        connectionStore.toggleFavorite(item.id);
      }
      break;
    case 'newSubGroup':
      if (item && 'children' in item) handleCreateGroup(item.id);
      break;
    case 'editGroup':
      if (item && 'children' in item) handleEditGroup(item as ConnectionGroup);
      break;
    case 'deleteGroup':
      if (item && 'children' in item) handleDeleteGroup(item as ConnectionGroup);
      break;
    case 'newConnection':
      if (item && 'children' in item) {
        const newConn: Connection = {
          name: '',
          host: '',
          port: 22,
          username: '',
          groupId: item.id,
        };
        emit('edit', newConn);
      } else {
        emit('edit', null);
      }
      break;
    case 'newGroup':
      handleCreateGroup();
      break;
  }
}

function handleContextMenu(event: MouseEvent) {
  event.preventDefault();
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  contextItem.value = null;
  menuItems.value = [
    { label: t('connections.contextMenu.newConnection'), action: 'newConnection', icon: Monitor },
    { label: t('connections.contextMenu.newGroup'), action: 'newGroup', icon: FolderPlus },
  ];
  menuVisible.value = true;
}

function handleItemContextMenu(event: MouseEvent, item: Connection | ConnectionGroup) {
  event.preventDefault();
  event.stopPropagation();
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  contextItem.value = item;

  const isGroup = 'children' in item;

  if (isGroup) {
    menuItems.value = [
      { label: t('connections.contextMenu.newConnection'), action: 'newConnection', icon: Monitor },
      { label: t('connections.contextMenu.newSubGroup'), action: 'newSubGroup', icon: FolderPlus },
      { label: t('connections.contextMenu.editGroup'), action: 'editGroup', icon: Pencil },
      { label: t('connections.contextMenu.deleteGroup'), action: 'deleteGroup', icon: Trash2, danger: true },
    ];
  } else {
    const connection = item as Connection;
    const isFavorite = connection.id !== undefined && connectionStore.isFavorite(connection.id);

    menuItems.value = [
      { label: t('connections.contextMenu.connect'), action: 'connect', icon: Monitor },
      { label: isFavorite ? t('connections.contextMenu.unfavorite') : t('connections.contextMenu.favorite'), action: 'favorite', icon: Star },
      { label: t('connections.contextMenu.edit'), action: 'edit', icon: Pencil },
      { label: t('connections.contextMenu.tunnels'), action: 'tunnels', icon: Cable },
      { label: t('connections.contextMenu.copy'), action: 'copy', icon: Copy },
      { label: t('connections.contextMenu.delete'), action: 'delete', icon: Trash2, danger: true },
    ];
  }

  menuVisible.value = true;
}

onMounted(async () => {
  connectionStore.loadConnections();

  unlistenDragEnter = await listen('tauri://drag-enter', (event) => {
    const payload = event.payload as { paths: string[]; position: { x: number; y: number } };
    if (payload.paths && payload.paths.length > 0) {
      isDragOver.value = true;
    }
  });

  unlistenDragLeave = await listen('tauri://drag-leave', () => {
    isDragOver.value = false;
  });

  unlistenDrop = await listen('tauri://drag-drop', async (event) => {
    isDragOver.value = false;
    const payload = event.payload as { paths: string[]; position: { x: number; y: number } };
    if (!containerRef.value) return;
    const rect = containerRef.value.getBoundingClientRect();
    const { x, y } = payload.position;
    if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return;

    const paths = payload.paths || [];
    for (const path of paths) {
      if (path.toLowerCase().endsWith('.json')) {
        try {
          const content = await readTextFile(path);
          const data = JSON.parse(content);
          const conns = Array.isArray(data) ? data : (data.connections || []);
          let importedCount = 0;
          for (const c of conns) {
            if (c.host && c.username) {
              const { id, ...rest } = c;
              await connectionStore.addConnection(rest as Connection);
              importedCount++;
            }
          }
          if (importedCount > 0) {
            notificationStore.success(t('connections.imported', { count: importedCount }) || `Imported ${importedCount} connections.`);
          }
        } catch (e) {
          console.error('Failed to import connections:', e);
        }
      }
    }
  });
});

onUnmounted(() => {
  if (unlistenDrop) unlistenDrop();
  if (unlistenDragEnter) unlistenDragEnter();
  if (unlistenDragLeave) unlistenDragLeave();
});

const treeData = computed(() => connectionStore.treeData);
const totalConnections = computed(() => connectionStore.connections.length);
const totalGroups = computed(() => connectionStore.groups.length);
const activeConnections = computed(() => sessionStore.sessions.length);
const query = computed(() => searchQuery.value.trim().toLowerCase());
const favoriteConnections = computed(() => connectionStore.favoriteConnections.slice(0, 6));

const historyConnectionMap = computed(() => {
  const map = new Map<number, Connection>();
  for (const conn of connectionStore.connections) {
    if (conn.id !== undefined) {
      map.set(conn.id, conn);
    }
  }
  return map;
});

const historyItems = computed(() => {
  const filtered = connectionStore.historyEntries.filter((entry) => {
    if (historyFilter.value === 'all') return true;
    return entry.status === historyFilter.value;
  });

  return filtered
    .map((entry) => {
      const connection = historyConnectionMap.value.get(entry.connectionId);
      if (!connection) return null;
      return { entry, connection };
    })
    .filter((item): item is { entry: ConnectionHistoryEntry; connection: Connection } => item !== null)
    .slice(0, 8);
});

function sourceLabel(source: ConnectionHistorySource | 'favorite') {
  switch (source) {
    case 'favorite':
      return t('connections.sources.favorite');
    case 'history':
      return t('connections.sources.history');
    case 'quick':
      return t('connections.sources.quick');
    case 'search':
      return t('connections.sources.search');
    default:
      return t('connections.sources.tree');
  }
}

const searchResults = computed<SearchResultItem[]>(() => {
  if (!query.value) return [];

  const results: SearchResultItem[] = [];
  const seenConnectionIds = new Set<number>();
  const seenGroupIds = new Set<number>();

  const pushConnection = (connection: Connection, source: ConnectionHistorySource | 'favorite') => {
    if (connection.id === undefined || seenConnectionIds.has(connection.id)) return;
    const values = [connection.name, connection.host, connection.username].filter(Boolean);
    const matchText = values.find((value) => value.toLowerCase().includes(query.value)) || connection.host;
    results.push({ kind: 'connection', item: connection, source, matchText });
    seenConnectionIds.add(connection.id);
  };

  for (const connection of favoriteConnections.value) {
    if ([connection.name, connection.host, connection.username].some((value) => value?.toLowerCase().includes(query.value))) {
      pushConnection(connection, 'favorite');
    }
  }

  for (const { connection, entry } of historyItems.value) {
    if ([connection.name, connection.host, connection.username].some((value) => value?.toLowerCase().includes(query.value))) {
      pushConnection(connection, entry.source === 'tree' ? 'history' : entry.source);
    }
  }

  for (const connection of connectionStore.connections) {
    if ([connection.name, connection.host, connection.username].some((value) => value?.toLowerCase().includes(query.value))) {
      pushConnection(connection, 'tree');
    }
  }

  for (const group of connectionStore.groups) {
    if (group.id !== undefined && !seenGroupIds.has(group.id) && group.name.toLowerCase().includes(query.value)) {
      results.push({ kind: 'group', item: group, source: 'tree', matchText: group.name });
      seenGroupIds.add(group.id);
    }
  }

  return results;
});

function matchesQuery(item: Connection | ConnectionGroup): boolean {
  if (!query.value) return true;

  if ("host" in item && "username" in item) {
    return [item.name, item.host, item.username]
      .filter(Boolean)
      .some((value) => value.toLowerCase().includes(query.value));
  }

  if ("children" in item) {
    const nameMatched = item.name.toLowerCase().includes(query.value);
    if (nameMatched) return true;
    return (item.children ?? []).some((child) => matchesQuery(child));
  }

  return item.name.toLowerCase().includes(query.value);
}

const filteredTreeData = computed(() => query.value ? treeData.value.filter((item) => matchesQuery(item)) : treeData.value);
const hasConnections = computed(() => totalConnections.value > 0 || totalGroups.value > 0);
const hasFilteredResults = computed(() => filteredTreeData.value.length > 0 || searchResults.value.length > 0);

function openNewConnection() {
  emit('edit', null);
}

function formatRecentTime(timestamp: number) {
  const diff = Date.now() - timestamp;
  const minutes = Math.max(1, Math.floor(diff / 60000));

  if (minutes < 60) {
    return `${minutes}m`;
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h`;
  }

  const days = Math.floor(hours / 24);
  return `${days}d`;
}

const draggedItem = ref<{ type: 'connection' | 'group'; id: number } | null>(null);

function onDragStart(event: DragEvent, item: Connection | ConnectionGroup) {
  if (event.dataTransfer) {
    const type = getItemType(item);
    draggedItem.value = { type, id: item.id! };
    event.dataTransfer.effectAllowed = 'move';
    const data = JSON.stringify({ type, id: item.id });
    event.dataTransfer.setData('application/json', data);
  }
}

function onDragOver(event: DragEvent) {
  event.preventDefault();
  event.stopPropagation();
  event.dataTransfer!.dropEffect = 'move';
}

async function onDrop(event: DragEvent, targetGroupId: number | null) {
  event.preventDefault();
  event.stopPropagation();
  const data = event.dataTransfer?.getData('application/json');
  if (data) {
    try {
      const { type, id } = JSON.parse(data);
      if (type === 'group' && id === targetGroupId) return;

      if (type === 'group' && targetGroupId !== null) {
        const isDescendant = (groupId: number, targetId: number): boolean => {
          const group = connectionStore.groups.find((g) => g.id === groupId);
          if (!group) return false;
          if (group.parentId === targetId) return true;
          return group.parentId ? isDescendant(group.parentId, targetId) : false;
        };

        if (isDescendant(id, targetGroupId)) {
          return;
        }
      }

      await connectionStore.moveItem(type, id, targetGroupId);
    } catch (e) {
      console.error('Invalid drop data', e);
    }
  }
  draggedItem.value = null;
}

function connect(conn: Connection, source: ConnectionHistorySource = 'tree') {
  if (conn.id !== undefined) {
    if (source === 'quick') {
      connectionStore.addSuccessfulConnection(conn.id, 'quick');
    }
  }
  sessionStore.createSession(conn);
}

function handleEdit(conn: Connection) {
  emit('edit', conn);
}

async function handleDelete(conn: Connection) {
  const confirmText = t('connections.deleteConfirm', { name: conn.name }) || `Delete ${conn.name}?`;
  const confirmed = await ask(confirmText, { title: t('connections.confirmTitle'), kind: 'warning' });
  if (confirmed) {
    await connectionStore.deleteConnection(conn.id!);
  }
}

async function handleCreateGroup(parentId?: number) {
  const name = prompt(t('connections.groupPrompt.create'));
  if (name) {
    await connectionStore.addGroup({ name, parentId: parentId || null });
  }
}

async function handleEditGroup(group: ConnectionGroup) {
  const name = prompt(t('connections.groupPrompt.rename'), group.name);
  if (name && name !== group.name) {
    await connectionStore.updateGroup({ ...group, name });
  }
}

async function handleDeleteGroup(group: ConnectionGroup) {
  const confirmText = t('connections.deleteGroupConfirm', { name: group.name }) || `Delete group "${group.name}" and all its contents?`;
  const confirmed = await ask(confirmText, { title: t('connections.confirmTitle'), kind: 'warning' });
  if (confirmed) {
    await connectionStore.deleteGroup(group.id!);
  }
}

function toggleFavorite(connectionId?: number) {
  if (connectionId === undefined) return;
  connectionStore.toggleFavorite(connectionId);
}

function isFavorite(connectionId?: number) {
  return connectionId !== undefined && connectionStore.isFavorite(connectionId);
}

function getItemType(item: Connection | ConnectionGroup): 'connection' | 'group' {
  return 'children' in item ? 'group' : 'connection';
}

function getItemKey(item: Connection | ConnectionGroup) {
  return getItemType(item) + '-' + item.id;
}

function historyStatusLabel(status: ConnectionHistoryEntry['status']) {
  return status === 'success' ? t('connections.history.status.success') : t('connections.history.status.failed');
}
</script>

<template>
  <div ref="containerRef" class="flex h-full flex-col relative" @contextmenu.prevent="handleContextMenu">
    <div
      v-if="isDragOver"
      class="absolute inset-0 z-50 flex items-center justify-center rounded border-2 border-accent bg-accent/10 pointer-events-none"
    >
      <div class="rounded border border-border-primary bg-bg-elevated px-4 py-2 font-medium text-text-primary shadow-md">
        {{ t('connections.importDropTitle') }}
      </div>
    </div>

    <div
      class="group flex items-center justify-between rounded p-2 shadow-interactive transition-all duration-200 hover:bg-bg-tertiary cursor-pointer select-none"
      @click="isRootExpanded = !isRootExpanded"
      @contextmenu.stop.prevent="handleContextMenu"
    >
      <div class="flex flex-1 items-center gap-2 overflow-hidden">
        <button class="rounded p-0.5 text-text-secondary transition-all hover:bg-bg-elevated hover:text-text-primary">
          <ChevronDown v-if="isRootExpanded" class="h-3 w-3" />
          <ChevronRight v-else class="h-3 w-3" />
        </button>
        <FolderOpen v-if="isRootExpanded" class="h-4 w-4 text-text-secondary" />
        <Folder v-else class="h-4 w-4 text-text-secondary" />
        <span class="truncate text-sm font-bold text-text-primary">{{ t('connections.rootLabel') }}</span>
      </div>

      <div class="flex items-center opacity-0 transition-opacity group-hover:opacity-100">
        <button
          class="cursor-pointer p-1 text-text-secondary hover:text-success"
          :title="t('connections.contextMenu.newGroup')"
          @click.stop="handleCreateGroup()"
        >
          <FolderPlus class="h-3 w-3" />
        </button>
      </div>
    </div>

    <div
      v-if="isRootExpanded"
      class="flex-1 overflow-y-auto"
      @dragover="onDragOver"
      @drop="onDrop($event, null)"
      @contextmenu="handleContextMenu"
    >
      <div
        v-if="isDragOver"
        class="mx-2 mb-2 rounded border-2 border-dashed border-accent bg-bg-secondary p-3 text-center text-sm text-text-primary shadow-interactive"
      >
        {{ t('connections.dropToRoot') }}
      </div>

      <div class="space-y-3 border-b border-border-primary bg-bg-secondary/60 px-2 pb-3 pt-2">
        <div class="flex items-center gap-2">
          <div class="relative flex-1">
            <Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-text-secondary" />
            <input
              v-model="searchQuery"
              type="text"
              :placeholder="t('connections.searchPlaceholder')"
              class="h-9 w-full rounded border border-border-primary bg-bg-tertiary pl-8 pr-3 text-sm text-text-primary outline-none focus:border-accent"
            />
          </div>
          <button
            class="flex h-9 items-center gap-1.5 rounded border border-border-primary bg-bg-tertiary px-3 text-sm text-text-primary transition-all hover:bg-bg-elevated"
            @click.stop="openNewConnection"
          >
            <Plus class="h-3.5 w-3.5" />
            <span>{{ t('connections.new') }}</span>
          </button>
        </div>

        <div v-if="hasConnections" class="flex flex-wrap items-center gap-2 text-xs">
          <span class="rounded-full border border-border-primary bg-bg-secondary px-2.5 py-1 text-text-secondary">
            {{ t('connections.summary.total') }} {{ totalConnections }}
          </span>
          <span class="rounded-full border border-border-primary bg-bg-secondary px-2.5 py-1 text-text-secondary">
            {{ t('connections.summary.groups') }} {{ totalGroups }}
          </span>
          <span class="rounded-full border border-border-primary bg-bg-secondary px-2.5 py-1 text-text-secondary">
            {{ t('connections.summary.active') }} {{ activeConnections }}
          </span>
        </div>

        <div v-if="favoriteConnections.length > 0" class="space-y-2 rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
          <button class="flex w-full items-center justify-between" @click="isFavoritesExpanded = !isFavoritesExpanded">
            <div class="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-text-secondary">
              <Star class="h-3.5 w-3.5" />
              <span>{{ t('connections.quickAccessTitle') }}</span>
            </div>
            <ChevronDown v-if="isFavoritesExpanded" class="h-3.5 w-3.5 text-text-secondary" />
            <ChevronRight v-else class="h-3.5 w-3.5 text-text-secondary" />
          </button>

          <div v-if="isFavoritesExpanded" class="grid gap-2">
            <button
              v-for="conn in favoriteConnections"
              :key="`favorite-${conn.id}`"
              class="w-full rounded-lg border border-border-primary bg-bg-secondary px-3 py-2 text-left transition-all hover:bg-bg-elevated"
              @click="connect(conn, 'quick')"
            >
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="truncate text-sm text-text-primary">{{ conn.name }}</span>
                    <span class="rounded-full bg-accent/10 px-2 py-0.5 text-[11px] text-accent">{{ t('connections.sources.favorite') }}</span>
                  </div>
                  <div class="mt-1 truncate text-xs text-text-secondary">{{ conn.username }}@{{ conn.host }}</div>
                </div>
                <button
                  class="rounded p-1 text-text-secondary transition-colors hover:bg-bg-tertiary hover:text-warning"
                  @click.stop="toggleFavorite(conn.id)"
                >
                  <Star class="h-3.5 w-3.5 fill-current" />
                </button>
              </div>
            </button>
          </div>
        </div>

        <div v-if="hasConnections" class="space-y-2 rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
          <div class="flex items-center justify-between gap-2">
            <button class="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-text-secondary" @click="isHistoryExpanded = !isHistoryExpanded">
              <History class="h-3.5 w-3.5" />
              <span>{{ t('connections.history.title') }}</span>
            </button>
            <div class="flex items-center gap-1 rounded-full bg-bg-secondary p-1 text-[11px]">
              <Filter class="h-3 w-3 text-text-secondary" />
              <button class="rounded-full px-2 py-1" :class="historyFilter === 'all' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary'" @click="historyFilter = 'all'">{{ t('connections.history.filters.all') }}</button>
              <button class="rounded-full px-2 py-1" :class="historyFilter === 'success' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary'" @click="historyFilter = 'success'">{{ t('connections.history.filters.success') }}</button>
              <button class="rounded-full px-2 py-1" :class="historyFilter === 'failed' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary'" @click="historyFilter = 'failed'">{{ t('connections.history.filters.failed') }}</button>
            </div>
          </div>

          <div v-if="isHistoryExpanded" class="space-y-2">
            <div v-if="historyItems.length === 0" class="rounded-lg border border-dashed border-border-primary bg-bg-secondary px-3 py-4 text-center text-xs text-text-secondary">
              {{ t('connections.history.empty') }}
            </div>

            <div v-for="item in historyItems" :key="`${item.connection.id}-${item.entry.connectedAt}`" class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-2">
              <div class="flex items-start justify-between gap-3">
                <button class="min-w-0 flex-1 text-left" @click="connect(item.connection, 'history')">
                  <div class="flex items-center gap-2">
                    <span class="truncate text-sm text-text-primary">{{ item.connection.name }}</span>
                    <span class="rounded-full px-2 py-0.5 text-[11px]" :class="item.entry.status === 'success' ? 'bg-success/10 text-success' : 'bg-error/10 text-error'">
                      {{ historyStatusLabel(item.entry.status) }}
                    </span>
                    <span class="rounded-full bg-bg-tertiary px-2 py-0.5 text-[11px] text-text-secondary">{{ sourceLabel(item.entry.source) }}</span>
                  </div>
                  <div class="mt-1 truncate text-xs text-text-secondary">{{ item.connection.username }}@{{ item.connection.host }}</div>
                  <div class="mt-1 flex items-center gap-2 text-[11px] text-text-secondary">
                    <CircleCheck v-if="item.entry.status === 'success'" class="h-3.5 w-3.5 text-success" />
                    <TriangleAlert v-else class="h-3.5 w-3.5 text-error" />
                    <span>{{ formatRecentTime(item.entry.connectedAt) }}</span>
                    <span v-if="item.entry.reason" class="truncate">{{ item.entry.reason }}</span>
                  </div>
                </button>
                <div class="flex items-center gap-1">
                  <button class="rounded p-1 text-text-secondary transition-colors hover:bg-bg-tertiary hover:text-warning" @click.stop="toggleFavorite(item.connection.id)">
                    <Star class="h-3.5 w-3.5" :class="isFavorite(item.connection.id) ? 'fill-current text-warning' : ''" />
                  </button>
                  <button class="rounded p-1 text-text-secondary transition-colors hover:bg-bg-tertiary hover:text-text-primary" @click.stop="handleEdit(item.connection)">
                    <Pencil class="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="query && searchResults.length > 0" class="space-y-2 rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
          <div class="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-text-secondary">
            <Search class="h-3.5 w-3.5" />
            <span>{{ t('connections.searchResultsTitle') }}</span>
          </div>
          <div class="grid gap-2">
            <div v-for="result in searchResults" :key="`${result.kind}-${result.item.id}`" class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-2">
              <template v-if="result.kind === 'connection'">
                <div class="flex items-start justify-between gap-3">
                  <button class="min-w-0 flex-1 text-left" @click="connect(result.item, result.source === 'favorite' ? 'quick' : result.source)">
                    <div class="flex items-center gap-2">
                      <span class="truncate text-sm text-text-primary">{{ result.item.name }}</span>
                      <span class="rounded-full bg-bg-tertiary px-2 py-0.5 text-[11px] text-text-secondary">{{ sourceLabel(result.source) }}</span>
                    </div>
                    <div class="mt-1 truncate text-xs text-text-secondary">{{ result.item.username }}@{{ result.item.host }}</div>
                  </button>
                  <button class="rounded p-1 text-text-secondary transition-colors hover:bg-bg-tertiary hover:text-warning" @click.stop="toggleFavorite(result.item.id)">
                    <Star class="h-3.5 w-3.5" :class="isFavorite(result.item.id) ? 'fill-current text-warning' : ''" />
                  </button>
                </div>
              </template>
              <template v-else>
                <div class="flex items-center justify-between gap-3">
                  <div class="min-w-0">
                    <div class="truncate text-sm text-text-primary">{{ result.item.name }}</div>
                    <div class="mt-1 flex items-center gap-2 text-xs text-text-secondary">
                      <Network class="h-3.5 w-3.5" />
                      <span>{{ sourceLabel(result.source) }}</span>
                    </div>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </div>
      </div>

      <div v-if="!hasConnections" class="mx-2 my-3 flex-1 rounded-xl border border-dashed border-border-primary bg-bg-secondary/70 p-5 text-center">
        <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-bg-tertiary text-accent">
          <FolderTree class="h-6 w-6" />
        </div>
        <div class="text-sm font-medium text-text-primary">{{ t('connections.empty.title') }}</div>
        <div class="mt-1 text-xs leading-5 text-text-secondary">{{ t('connections.empty.description') }}</div>
        <div class="mt-4 flex items-center justify-center gap-2">
          <button class="h-9 rounded border border-border-primary bg-accent px-3 text-sm text-white transition-all hover:opacity-90" @click.stop="openNewConnection">
            {{ t('connections.empty.createConnection') }}
          </button>
          <button class="flex h-9 items-center gap-1.5 rounded border border-border-primary bg-bg-tertiary px-3 text-sm text-text-primary transition-all hover:bg-bg-elevated" @click.stop="handleCreateGroup()">
            <FolderPlus class="h-3.5 w-3.5" />
            <span>{{ t('connections.empty.createGroup') }}</span>
          </button>
        </div>
        <div class="mt-4 grid grid-cols-2 gap-2 text-left text-xs text-text-secondary">
          <div class="rounded border border-border-primary bg-bg-tertiary/80 px-3 py-2">
            <div class="flex items-center gap-1.5 text-text-primary">
              <Monitor class="h-3.5 w-3.5" />
              <span>{{ t('connections.empty.tipConnect') }}</span>
            </div>
          </div>
          <div class="rounded border border-border-primary bg-bg-tertiary/80 px-3 py-2">
            <div class="flex items-center gap-1.5 text-text-primary">
              <FileDown class="h-3.5 w-3.5" />
              <span>{{ t('connections.empty.tipImport') }}</span>
            </div>
          </div>
        </div>
      </div>

      <div v-else-if="query && !hasFilteredResults" class="mx-2 my-3 rounded-xl border border-dashed border-border-primary bg-bg-secondary/70 p-5 text-center">
        <div class="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-bg-tertiary text-text-secondary">
          <Search class="h-5 w-5" />
        </div>
        <div class="text-sm font-medium text-text-primary">{{ t('connections.searchEmpty.title') }}</div>
        <div class="mt-1 text-xs text-text-secondary">{{ t('connections.searchEmpty.description') }}</div>
      </div>

      <div v-else class="space-y-2 px-2 py-3">
        <div class="flex items-center justify-between px-1 text-xs font-medium uppercase tracking-wide text-text-secondary">
          <div class="flex items-center gap-1.5">
            <FolderTree class="h-3.5 w-3.5" />
            <span>{{ t('connections.treeTitle') }}</span>
          </div>
          <span>{{ t('connections.treeHint') }}</span>
        </div>
        <div class="min-h-[50px] space-y-0.5">
          <ConnectionTreeItem
            v-for="item in filteredTreeData"
            :key="getItemKey(item)"
            :item="item"
            :level="1"
            @connect="connect"
            @edit="handleEdit"
            @delete="handleDelete"
            @create-group="handleCreateGroup"
            @edit-group="handleEditGroup"
            @delete-group="handleDeleteGroup"
            @drag-start="onDragStart"
            @drop-item="onDrop"
            @context-menu="handleItemContextMenu"
          />
        </div>
      </div>
    </div>

    <ContextMenu v-if="menuVisible" :x="menuX" :y="menuY" :items="menuItems" @close="closeMenu" @action="handleMenuAction" />
  </div>
</template>

<style scoped>
.ghost {
  opacity: 0.5;
  background: var(--bg-tertiary);
  border: 1px dashed var(--border-subtle);
}

.drag {
  opacity: 1;
  background: var(--bg-elevated);
}
</style>
