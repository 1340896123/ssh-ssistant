<script setup lang="ts">
import { useConnectionStore } from '../stores/connections';
import { useSessionStore } from '../stores/sessions';
import { useI18n } from '../composables/useI18n';
import { onMounted, computed, ref, onUnmounted } from 'vue';
import {
  FolderPlus, ChevronRight, ChevronDown, FolderOpen, Folder,
  Monitor, Pencil, Trash2, Copy, Cable, Search, Plus, Clock3,
  Network, FolderTree, FileDown
} from 'lucide-vue-next';
import ConnectionTreeItem from './ConnectionTreeItem.vue';
import type { Connection, ConnectionGroup } from '../types';
import { listen } from '@tauri-apps/api/event';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { ask } from '@tauri-apps/plugin-dialog';
import { useNotificationStore } from '../stores/notifications';
import ContextMenu, { type MenuItem } from './ContextMenu.vue';

const connectionStore = useConnectionStore();
const sessionStore = useSessionStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();
const emit = defineEmits(['edit', 'tunnels']);

// Context Menu State
const menuVisible = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const menuItems = ref<MenuItem[]>([]);
const contextItem = ref<Connection | ConnectionGroup | null>(null);

function closeMenu() {
  menuVisible.value = false;
  contextItem.value = null;
}

async function handleMenuAction(action: string) {
  if (!contextItem.value && action !== 'newConnection' && action !== 'newGroup') {
    if (action === 'newConnection' || action === 'newGroup') {
      // Pass
    } else {
      return;
    }
  }

  const item = contextItem.value;

  switch (action) {
    case 'connect':
      if (item && !('children' in item)) connect(item as Connection);
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
        // Create new connection in this group
        const newConn: Connection = {
          name: '',
          host: '',
          port: 22,
          username: '',
          groupId: item.id
        };
        emit('edit', newConn);
      } else {
        emit('edit', null); // Trigger new connection modal (Root)
      }
      break;
    case 'newGroup':
      handleCreateGroup();
      break;
  }
}

function handleContextMenu(event: MouseEvent) {
  // Background context menu
  event.preventDefault();
  menuX.value = event.clientX;
  menuY.value = event.clientY;
  contextItem.value = null;
  menuItems.value = [
    { label: t('connections.contextMenu.newConnection'), action: 'newConnection', icon: Monitor },
    { label: t('connections.contextMenu.newGroup'), action: 'newGroup', icon: FolderPlus }
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
      { label: t('connections.contextMenu.newConnection'), action: 'newConnection', icon: Monitor }, // Add New Connection to Group
      { label: t('connections.contextMenu.newSubGroup'), action: 'newSubGroup', icon: FolderPlus },
      { label: t('connections.contextMenu.editGroup'), action: 'editGroup', icon: Pencil },
      { label: t('connections.contextMenu.deleteGroup'), action: 'deleteGroup', icon: Trash2, danger: true }
    ];
  } else {
    menuItems.value = [
      { label: t('connections.contextMenu.connect'), action: 'connect', icon: Monitor },
      { label: t('connections.contextMenu.edit'), action: 'edit', icon: Pencil },
      { label: t('connections.contextMenu.tunnels'), action: 'tunnels', icon: Cable },
      { label: t('connections.contextMenu.copy'), action: 'copy', icon: Copy },
      { label: t('connections.contextMenu.delete'), action: 'delete', icon: Trash2, danger: true }
    ];
  }
  menuVisible.value = true;
}

const isRootExpanded = ref(true);
const containerRef = ref<HTMLElement | null>(null);
let unlistenDrop: (() => void) | null = null;
let unlistenDragEnter: (() => void) | null = null;
let unlistenDragLeave: (() => void) | null = null;
const isDragOver = ref(false);
const searchQuery = ref('');

onMounted(async () => {
  connectionStore.loadConnections();

  unlistenDragEnter = await listen('tauri://drag-enter', (event) => {
    const payload = event.payload as { paths: string[], position: { x: number, y: number } };
    if (payload.paths && payload.paths.length > 0) {
      console.log("Drag enter with files");
      isDragOver.value = true;
    }
  });

  unlistenDragLeave = await listen('tauri://drag-leave', () => {
    console.log("Drag leave");
    isDragOver.value = false;
  });

  unlistenDrop = await listen('tauri://drag-drop', async (event) => {
    console.log("Dropped files:", event);
    isDragOver.value = false;
    const payload = event.payload as { paths: string[], position: { x: number, y: number } };
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
              // Ensure new connection
              const { id, ...rest } = c;
              await connectionStore.addConnection(rest as any);
              importedCount++;
            }
          }
          if (importedCount > 0) {
            notificationStore.success(`Imported ${importedCount} connections.`);
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

const recentConnections = computed(() => {
  const sessionsByConnectionId = new Map<number, number>();

  for (const session of sessionStore.sessions) {
    const previous = sessionsByConnectionId.get(session.connectionId) ?? 0;
    sessionsByConnectionId.set(session.connectionId, Math.max(previous, session.connectedAt));
  }

  return connectionStore.connections
    .filter(conn => conn.id !== undefined && sessionsByConnectionId.has(conn.id))
    .map(conn => ({
      ...conn,
      recentAt: sessionsByConnectionId.get(conn.id!) ?? 0
    }))
    .sort((a, b) => b.recentAt - a.recentAt)
    .slice(0, 4);
});

const quickConnections = computed(() =>
  connectionStore.connections
    .filter(conn => conn.id !== undefined)
    .slice(0, 4)
);

function matchesQuery(item: Connection | ConnectionGroup): boolean {
  if (!query.value) return true;

  if ('children' in item) {
    const nameMatched = item.name.toLowerCase().includes(query.value);
    if (nameMatched) return true;
    return (item.children ?? []).some(child => matchesQuery(child));
  }

  return [item.name, item.host, item.username]
    .filter(Boolean)
    .some(value => value.toLowerCase().includes(query.value));
}

const filteredTreeData = computed(() =>
  query.value ? treeData.value.filter(item => matchesQuery(item)) : treeData.value
);

const hasConnections = computed(() => totalConnections.value > 0 || totalGroups.value > 0);
const hasFilteredResults = computed(() => filteredTreeData.value.length > 0);

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

// Drag and Drop Logic
const draggedItem = ref<{ type: 'connection' | 'group', id: number } | null>(null);

function onDragStart(event: DragEvent, item: Connection | ConnectionGroup) {
  console.log('ConnectionList: Drag start:', item.name, 'Type:', getItemType(item));
  if (event.dataTransfer) {
    const type = getItemType(item);
    draggedItem.value = { type, id: item.id! };
    event.dataTransfer.effectAllowed = 'move';
    const data = JSON.stringify({ type, id: item.id });
    event.dataTransfer.setData('application/json', data);
    console.log('ConnectionList: Set drag data:', data);
  }
}

function onDragOver(event: DragEvent) {
  event.preventDefault();
  event.stopPropagation();
  event.dataTransfer!.dropEffect = 'move';
}

async function onDrop(event: DragEvent, targetGroupId: number | null) {
  console.log('ConnectionList: Drop event, targetGroupId:', targetGroupId);
  event.preventDefault();
  event.stopPropagation();
  const data = event.dataTransfer?.getData('application/json');
  console.log('ConnectionList: Got drop data:', data);
  if (data) {
    try {
      const { type, id } = JSON.parse(data);
      console.log('ConnectionList: Parsed data:', { type, id });

      // Prevent dropping into itself or its children (for groups)
      if (type === 'group' && id === targetGroupId) return;

      // Check if we're trying to drop a group into its own child
      if (type === 'group' && targetGroupId !== null) {
        const isDescendant = (groupId: number, targetId: number): boolean => {
          const group = connectionStore.groups.find(g => g.id === groupId);
          if (!group) return false;
          if (group.parentId === targetId) return true;
          return group.parentId ? isDescendant(group.parentId, targetId) : false;
        };

        if (isDescendant(id, targetGroupId)) {
          console.log('Cannot drop group into its own descendant');
          return;
        }
      }

      console.log('ConnectionList: Calling moveItem:', type, id, targetGroupId);
      await connectionStore.moveItem(type, id, targetGroupId);
      console.log(`ConnectionList: Moved ${type} ${id} to group ${targetGroupId}`);
    } catch (e) {
      console.error("Invalid drop data", e);
    }
  }
  draggedItem.value = null;
}


function connect(conn: Connection) {
  sessionStore.createSession(conn);
}

function handleEdit(conn: Connection) {
  emit('edit', conn);
}

async function handleDelete(conn: Connection) {
  const confirmText = t('connections.deleteConfirm', { name: conn.name }) || `Delete ${conn.name}?`;
  const confirmed = await ask(confirmText, { title: '确认删除', kind: 'warning' });
  if (confirmed) {
    await connectionStore.deleteConnection(conn.id!);
  }
}

async function handleCreateGroup(parentId?: number) {
  const name = prompt("Enter group name:");
  if (name) {
    await connectionStore.addGroup({ name, parentId: parentId || null });
  }
}

async function handleEditGroup(group: ConnectionGroup) {
  const name = prompt("Enter new group name:", group.name);
  if (name && name !== group.name) {
    await connectionStore.updateGroup({ ...group, name });
  }
}

async function handleDeleteGroup(group: ConnectionGroup) {
  const confirmText = t('connections.deleteGroupConfirm', { name: group.name }) || `Delete group "${group.name}" and all its contents?`;
  const confirmed = await ask(confirmText, { title: '确认删除', kind: 'warning' });
  if (confirmed) {
    await connectionStore.deleteGroup(group.id!);
  }
}

function getItemType(item: Connection | ConnectionGroup): 'connection' | 'group' {
  return ('children' in item) ? 'group' : 'connection';
}

function getItemKey(item: Connection | ConnectionGroup) {
  return getItemType(item) + '-' + item.id;
}

// Removed onRootChange as we use native drop

</script>

<template>
  <div ref="containerRef" class="flex flex-col h-full relative" @contextmenu.prevent="handleContextMenu">
    <div v-if="isDragOver"
      class="absolute inset-0 bg-accent/10 border-2 border-accent z-50 rounded pointer-events-none flex items-center justify-center">
      <div class="bg-bg-elevated border border-border-primary px-4 py-2 rounded shadow-md font-medium text-text-primary">
        Drop JSON to Import
      </div>
    </div>
    <!-- Root Node -->
    <div class="group shadow-interactive flex items-center justify-between p-2 hover:bg-bg-tertiary rounded cursor-pointer select-none transition-all duration-200"
      @click="isRootExpanded = !isRootExpanded" @contextmenu.stop.prevent="handleContextMenu">
      <div class="flex items-center space-x-2 overflow-hidden flex-1">
        <button class="p-0.5 hover:bg-bg-elevated rounded text-text-secondary hover:text-text-primary transition-all">
          <ChevronDown v-if="isRootExpanded" class="w-3 h-3" />
          <ChevronRight v-else class="w-3 h-3" />
        </button>
        <FolderOpen v-if="isRootExpanded" class="w-4 h-4 text-text-secondary" />
        <Folder v-else class="w-4 h-4 text-text-secondary" />
        <span class="text-sm text-text-primary font-bold">Root</span>
      </div>

      <div class="flex items-center opacity-0 group-hover:opacity-100 transition-opacity">
        <button @click.stop="handleCreateGroup()" class="p-1 text-text-secondary hover:text-success cursor-pointer"
          title="New Group">
          <FolderPlus class="w-3 h-3" />
        </button>
      </div>
    </div>

    <!-- Root Children -->
    <div v-if="isRootExpanded" class="flex-1 overflow-y-auto" @dragover="onDragOver" @drop="onDrop($event, null)"
      @contextmenu="handleContextMenu">
      <!-- Root Drop Zone Indicator -->
      <div v-if="isDragOver"
        class="shadow-interactive mx-2 mb-2 p-3 border-2 border-dashed border-accent rounded bg-bg-secondary text-text-primary text-sm text-center">
        拖放到此处以移动到根目录
      </div>
      <div class="px-2 pt-2 pb-1 space-y-3 border-b border-border-primary bg-bg-secondary/60">
        <div class="flex items-center gap-2">
          <div class="relative flex-1">
            <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-text-secondary" />
            <input
              v-model="searchQuery"
              type="text"
              :placeholder="t('connections.searchPlaceholder')"
              class="w-full h-9 pl-8 pr-3 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none text-sm"
            />
          </div>
          <button
            class="h-9 px-3 rounded border border-border-primary bg-bg-tertiary text-text-primary hover:bg-bg-elevated transition-all flex items-center gap-1.5 text-sm"
            @click.stop="openNewConnection"
          >
            <Plus class="w-3.5 h-3.5" />
            <span>{{ t('connections.new') }}</span>
          </button>
        </div>

        <div v-if="hasConnections" class="grid grid-cols-3 gap-2 text-xs">
          <div class="rounded border border-border-primary bg-bg-tertiary px-2.5 py-2">
            <div class="text-text-secondary">{{ t('connections.summary.total') }}</div>
            <div class="mt-1 text-sm font-semibold text-text-primary">{{ totalConnections }}</div>
          </div>
          <div class="rounded border border-border-primary bg-bg-tertiary px-2.5 py-2">
            <div class="text-text-secondary">{{ t('connections.summary.groups') }}</div>
            <div class="mt-1 text-sm font-semibold text-text-primary">{{ totalGroups }}</div>
          </div>
          <div class="rounded border border-border-primary bg-bg-tertiary px-2.5 py-2">
            <div class="text-text-secondary">{{ t('connections.summary.active') }}</div>
            <div class="mt-1 text-sm font-semibold text-text-primary">{{ activeConnections }}</div>
          </div>
        </div>

        <div v-if="recentConnections.length > 0" class="space-y-2">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-1.5 text-xs font-medium text-text-secondary uppercase tracking-wide">
              <Clock3 class="w-3.5 h-3.5" />
              <span>{{ t('connections.recentTitle') }}</span>
            </div>
            <span class="text-[11px] text-text-secondary">{{ t('connections.recentHint') }}</span>
          </div>
          <div class="grid gap-2">
            <button
              v-for="conn in recentConnections"
              :key="`recent-${conn.id}`"
              class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-left hover:bg-bg-elevated transition-all"
              @click="connect(conn)"
            >
              <div class="flex items-center justify-between gap-3">
                <div class="min-w-0">
                  <div class="text-sm text-text-primary truncate">{{ conn.name }}</div>
                  <div class="text-xs text-text-secondary truncate">{{ conn.username }}@{{ conn.host }}</div>
                </div>
                <span class="text-[11px] text-text-secondary whitespace-nowrap">{{ formatRecentTime(conn.recentAt) }}</span>
              </div>
            </button>
          </div>
        </div>

        <div v-else-if="hasConnections && quickConnections.length > 0" class="space-y-2">
          <div class="flex items-center gap-1.5 text-xs font-medium text-text-secondary uppercase tracking-wide">
            <Network class="w-3.5 h-3.5" />
            <span>{{ t('connections.quickAccessTitle') }}</span>
          </div>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="conn in quickConnections"
              :key="`quick-${conn.id}`"
              class="max-w-full rounded-full border border-border-primary bg-bg-tertiary px-3 py-1.5 text-xs text-text-primary hover:bg-bg-elevated transition-all truncate"
              @click="connect(conn)"
            >
              {{ conn.name }}
            </button>
          </div>
        </div>
      </div>

      <div v-if="!hasConnections" class="mx-2 my-3 flex-1 rounded-xl border border-dashed border-border-primary bg-bg-secondary/70 p-5 text-center">
        <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-bg-tertiary text-accent">
          <FolderTree class="w-6 h-6" />
        </div>
        <div class="text-sm font-medium text-text-primary">{{ t('connections.empty.title') }}</div>
        <div class="mt-1 text-xs leading-5 text-text-secondary">{{ t('connections.empty.description') }}</div>
        <div class="mt-4 flex items-center justify-center gap-2">
          <button
            class="h-9 px-3 rounded border border-border-primary bg-accent text-white hover:opacity-90 transition-all text-sm"
            @click.stop="openNewConnection"
          >
            {{ t('connections.empty.createConnection') }}
          </button>
          <button
            class="h-9 px-3 rounded border border-border-primary bg-bg-tertiary text-text-primary hover:bg-bg-elevated transition-all text-sm flex items-center gap-1.5"
            @click.stop="handleCreateGroup()"
          >
            <FolderPlus class="w-3.5 h-3.5" />
            <span>{{ t('connections.empty.createGroup') }}</span>
          </button>
        </div>
        <div class="mt-4 grid grid-cols-2 gap-2 text-left text-xs text-text-secondary">
          <div class="rounded border border-border-primary bg-bg-tertiary/80 px-3 py-2">
            <div class="flex items-center gap-1.5 text-text-primary">
              <Monitor class="w-3.5 h-3.5" />
              <span>{{ t('connections.empty.tipConnect') }}</span>
            </div>
          </div>
          <div class="rounded border border-border-primary bg-bg-tertiary/80 px-3 py-2">
            <div class="flex items-center gap-1.5 text-text-primary">
              <FileDown class="w-3.5 h-3.5" />
              <span>{{ t('connections.empty.tipImport') }}</span>
            </div>
          </div>
        </div>
      </div>

      <div v-else-if="query && !hasFilteredResults" class="mx-2 my-3 rounded-xl border border-dashed border-border-primary bg-bg-secondary/70 p-5 text-center">
        <div class="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-bg-tertiary text-text-secondary">
          <Search class="w-5 h-5" />
        </div>
        <div class="text-sm font-medium text-text-primary">{{ t('connections.searchEmpty.title') }}</div>
        <div class="mt-1 text-xs text-text-secondary">{{ t('connections.searchEmpty.description') }}</div>
      </div>

      <div v-else class="space-y-0.5 min-h-[50px]">
        <ConnectionTreeItem v-for="item in filteredTreeData" :key="getItemKey(item)" :item="item" :level="1" @connect="connect"
          @edit="handleEdit" @delete="handleDelete" @create-group="handleCreateGroup" @edit-group="handleEditGroup"
          @delete-group="handleDeleteGroup" @drag-start="onDragStart" @drop-item="onDrop"
          @context-menu="handleItemContextMenu" />
      </div>
    </div>

    <ContextMenu v-if="menuVisible" :x="menuX" :y="menuY" :items="menuItems" @close="closeMenu"
      @action="handleMenuAction" />
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
