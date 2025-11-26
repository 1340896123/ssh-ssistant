<script setup lang="ts">
import { ref, computed } from 'vue';
import { Monitor, Folder, FolderOpen, ChevronRight, ChevronDown, Pencil, Trash2, Plus } from 'lucide-vue-next';
import type { Connection, ConnectionGroup } from '../types';
import { useI18n } from '../composables/useI18n';
import { useConnectionStore } from '../stores/connections';
import draggable from 'vuedraggable';

const props = defineProps<{
    item: Connection | ConnectionGroup;
    level: number;
}>();

const emit = defineEmits(['connect', 'edit', 'delete', 'create-group', 'edit-group', 'delete-group']);

const { t } = useI18n();
const connectionStore = useConnectionStore();
const isExpanded = ref(false);

const isGroup = computed(() => 'children' in props.item || 'parentId' in props.item);
const paddingLeft = computed(() => `${props.level * 16 + 8}px`);

function toggleExpand() {
    if (isGroup.value) {
        isExpanded.value = !isExpanded.value;
    }
}

function handleConnect() {
    if (!isGroup.value) {
        emit('connect', props.item);
    }
}

function handleEdit() {
    if (isGroup.value) {
        emit('edit-group', props.item);
    } else {
        emit('edit', props.item);
    }
}

function handleDelete() {
    if (isGroup.value) {
        emit('delete-group', props.item);
    } else {
        emit('delete', props.item);
    }
}

function handleCreateSubGroup() {
    if (isGroup.value) {
        emit('create-group', props.item.id);
    }
}

function getItemType(item: Connection | ConnectionGroup): 'connection' | 'group' {
    return ('children' in item) ? 'group' : 'connection';
}

function getItemKey(item: Connection | ConnectionGroup) {
    return getItemType(item) + '-' + item.id;
}

async function onGroupChange(event: any) {
    if (event.added) {
        const item = event.added.element;
        const type = getItemType(item);
        await connectionStore.moveItem(type, item.id, props.item.id!);
        isExpanded.value = true;
    }
}
</script>

<template>
    <div>
        <div class="group flex items-center justify-between p-2 hover:bg-gray-700 rounded cursor-pointer select-none transition-colors duration-200"
            :style="{ paddingLeft }" @click="toggleExpand" @dblclick="handleConnect">
            <div class="flex items-center space-x-2 overflow-hidden flex-1">
                <template v-if="isGroup">
                    <button class="p-0.5 hover:bg-gray-600 rounded text-gray-400">
                        <ChevronDown v-if="isExpanded" class="w-3 h-3" />
                        <ChevronRight v-else class="w-3 h-3" />
                    </button>
                    <FolderOpen v-if="isExpanded" class="w-4 h-4 text-yellow-400" />
                    <Folder v-else class="w-4 h-4 text-yellow-400" />
                </template>
                <template v-else>
                    <span class="w-4"></span> <!-- Spacer for alignment -->
                    <Monitor class="w-4 h-4 text-blue-400" />
                </template>
                <span class="text-sm text-gray-200 truncate" :title="item.name">{{ item.name }}</span>
            </div>

            <div class="flex items-center opacity-0 group-hover:opacity-100 transition-opacity">
                <button v-if="isGroup" @click.stop="handleCreateSubGroup"
                    class="p-1 text-gray-500 hover:text-green-400 cursor-pointer mr-1" title="New Subgroup">
                    <Plus class="w-3 h-3" />
                </button>
                <button @click.stop="handleEdit" class="p-1 text-gray-500 hover:text-blue-400 cursor-pointer mr-1"
                    :title="t('connections.edit')">
                    <Pencil class="w-3 h-3" />
                </button>
                <button @click.stop="handleDelete" class="p-1 text-gray-500 hover:text-red-400 cursor-pointer"
                    :title="t('connections.delete')">
                    <Trash2 class="w-3 h-3" />
                </button>
            </div>
        </div>

        <div v-if="isGroup && isExpanded">
            <draggable :list="(item as ConnectionGroup).children" group="connections" :item-key="getItemKey"
                class="min-h-[10px]" @change="onGroupChange">
                <template #item="{ element }">
                    <ConnectionTreeItem :item="element" :level="level + 1" @connect="$emit('connect', $event)"
                        @edit="$emit('edit', $event)" @delete="$emit('delete', $event)"
                        @create-group="$emit('create-group', $event)" @edit-group="$emit('edit-group', $event)"
                        @delete-group="$emit('delete-group', $event)" />
                </template>
            </draggable>
            <div v-if="!(item as ConnectionGroup).children?.length" class="text-xs text-gray-500 py-1"
                :style="{ paddingLeft: `${(level + 1) * 16 + 24}px` }">
                (Empty)
            </div>
        </div>
    </div>
</template>
