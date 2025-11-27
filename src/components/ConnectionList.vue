<script setup lang="ts">
import { useConnectionStore } from '../stores/connections';
import { useSessionStore } from '../stores/sessions';
import { useI18n } from '../composables/useI18n';
import { onMounted, computed, ref, watch } from 'vue';
import { FolderPlus, ChevronRight, ChevronDown, FolderOpen, Folder } from 'lucide-vue-next';
import ConnectionTreeItem from './ConnectionTreeItem.vue';
import type { Connection, ConnectionGroup } from '../types';
import draggable from 'vuedraggable';
import { listen } from '@tauri-apps/api/event';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { onUnmounted } from 'vue';

const connectionStore = useConnectionStore();
const sessionStore = useSessionStore();
const { t } = useI18n();
const emit = defineEmits(['edit']);

const isRootExpanded = ref(true);
const containerRef = ref<HTMLElement | null>(null);
let unlistenDrop: (() => void) | null = null;

onMounted(async () => {
  connectionStore.loadConnections();

  unlistenDrop = await listen('tauri://drag-drop', async (event) => {
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
            alert(`Imported ${importedCount} connections.`);
          }
        } catch (e) {
          console.error('Failed to import connections:', e);
        }
      }
    }
  });
});

onUnmounted(() => {
  if (unlistenDrop) {
    unlistenDrop();
  }
});

const treeData = computed(() => connectionStore.treeData);
const localTreeData = ref<(Connection | ConnectionGroup)[]>([]);

watch(treeData, (newVal) => {
  localTreeData.value = [...newVal];
}, { immediate: true });

function connect(conn: Connection) {
  sessionStore.createSession(conn);
}

function handleEdit(conn: Connection) {
  emit('edit', conn);
}

async function handleDelete(conn: Connection) {
  if (confirm(t('connections.deleteConfirm', { name: conn.name }) || `Delete ${conn.name}?`)) {
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
  if (confirm(`Delete group "${group.name}" and all its contents?`)) {
    await connectionStore.deleteGroup(group.id!);
  }
}

function getItemType(item: Connection | ConnectionGroup): 'connection' | 'group' {
  return ('children' in item) ? 'group' : 'connection';
}

function getItemKey(item: Connection | ConnectionGroup) {
  return getItemType(item) + '-' + item.id;
}

async function onRootChange(event: any) {
  if (event.added) {
    const item = event.added.element;
    const type = getItemType(item);
    await connectionStore.moveItem(type, item.id, null);
  }
}
</script>

<template>
  <div ref="containerRef" class="flex flex-col h-full">
    <!-- Root Node -->
    <div class="group flex items-center justify-between p-2 hover:bg-gray-700 rounded cursor-pointer select-none"
      @click="isRootExpanded = !isRootExpanded">
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
    <div v-if="isRootExpanded" class="flex-1 overflow-y-auto">
      <draggable v-model="localTreeData" group="connections" :item-key="getItemKey" class="space-y-0.5 min-h-[50px]"
        ghost-class="ghost" drag-class="drag" @change="onRootChange">
        <template #item="{ element }">
          <ConnectionTreeItem :item="element" :level="1" @connect="connect" @edit="handleEdit" @delete="handleDelete"
            @create-group="handleCreateGroup" @edit-group="handleEditGroup" @delete-group="handleDeleteGroup" />
        </template>
      </draggable>
      <div v-if="treeData.length === 0" class="text-center text-gray-500 text-sm py-4 ml-4">
        (Empty)
      </div>
    </div>
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
