<script setup lang="ts">
import { useConnectionStore } from '../stores/connections';
import { useSessionStore } from '../stores/sessions';
import { useI18n } from '../composables/useI18n';
import { onMounted, computed, ref, onUnmounted } from 'vue';
import {
  FolderPlus, ChevronRight, ChevronDown, FolderOpen, Folder,
  Monitor, Pencil, Trash2, Copy
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
const emit = defineEmits(['edit']);

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
      class="absolute inset-0 bg-blue-500/10 border-2 border-blue-500 z-50 rounded pointer-events-none flex items-center justify-center">
      <div class="bg-gray-800 text-blue-400 px-4 py-2 rounded shadow-lg font-medium">
        Drop JSON to Import
      </div>
    </div>
    <!-- Root Node -->
    <div class="group flex items-center justify-between p-2 hover:bg-gray-700 rounded cursor-pointer select-none"
      @click="isRootExpanded = !isRootExpanded" @contextmenu.stop.prevent="handleContextMenu">
      <div class="flex items-center space-x-2 overflow-hidden flex-1">
        <button class="p-0.5 hover:bg-gray-600 rounded text-gray-400">
          <ChevronDown v-if="isRootExpanded" class="w-3 h-3" />
          <ChevronRight v-else class="w-3 h-3" />
        </button>
        <FolderOpen v-if="isRootExpanded" class="w-4 h-4 text-yellow-400" />
        <Folder v-else class="w-4 h-4 text-yellow-400" />
        <span class="text-sm text-gray-200 font-bold">Root</span>
      </div>

      <div class="flex items-center opacity-0 group-hover:opacity-100 transition-opacity">
        <button @click.stop="handleCreateGroup()" class="p-1 text-gray-500 hover:text-green-400 cursor-pointer"
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
        class="mx-2 mb-2 p-3 border-2 border-dashed border-blue-500 rounded bg-blue-500/10 text-blue-400 text-sm text-center">
        拖放到此处以移动到根目录
      </div>
      <div class="space-y-0.5 min-h-[50px]">
        <ConnectionTreeItem v-for="item in treeData" :key="getItemKey(item)" :item="item" :level="1" @connect="connect"
          @edit="handleEdit" @delete="handleDelete" @create-group="handleCreateGroup" @edit-group="handleEditGroup"
          @delete-group="handleDeleteGroup" @drag-start="onDragStart" @drop-item="onDrop"
          @context-menu="handleItemContextMenu" />
      </div>
      <div v-if="treeData.length === 0" class="text-center text-gray-500 text-sm py-4 ml-4">
        (Empty)
      </div>
    </div>

    <ContextMenu v-if="menuVisible" :x="menuX" :y="menuY" :items="menuItems" @close="closeMenu"
      @action="handleMenuAction" />
  </div>
</template>

<style scoped>
.ghost {
  opacity: 0.5;
  background: #374151;
  border: 1px dashed #6b7280;
}

.drag {
  opacity: 1;
  background: #1f2937;
}
</style>
