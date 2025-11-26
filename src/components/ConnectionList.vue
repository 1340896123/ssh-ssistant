<script setup lang="ts">
import { useConnectionStore } from '../stores/connections';
import { useSessionStore } from '../stores/sessions';
import { useI18n } from '../composables/useI18n';
import { onMounted, computed } from 'vue';
import { FolderPlus } from 'lucide-vue-next';
import ConnectionTreeItem from './ConnectionTreeItem.vue';
import type { Connection, ConnectionGroup } from '../types';
import draggable from 'vuedraggable';

const connectionStore = useConnectionStore();
const sessionStore = useSessionStore();
const { t } = useI18n();
const emit = defineEmits(['edit']);

onMounted(() => {
  connectionStore.loadConnections();
});

const treeData = computed(() => connectionStore.treeData);

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
  <div class="flex flex-col h-full">
    <div class="flex justify-end px-2 py-1">
      <button @click="handleCreateGroup()" class="p-1 text-gray-400 hover:text-white" title="New Group">
        <FolderPlus class="w-4 h-4" />
      </button>
    </div>
    <div class="flex-1 overflow-y-auto">
      <draggable :list="treeData" group="connections" :item-key="getItemKey" class="space-y-0.5 min-h-[50px]"
        @change="onRootChange">
        <template #item="{ element }">
          <ConnectionTreeItem :item="element" :level="0" @connect="connect" @edit="handleEdit" @delete="handleDelete"
            @create-group="handleCreateGroup" @edit-group="handleEditGroup" @delete-group="handleDeleteGroup" />
        </template>
      </draggable>
      <div v-if="treeData.length === 0" class="text-center text-gray-500 text-sm py-4">
        No connections found.
      </div>
    </div>
  </div>
</template>
