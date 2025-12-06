<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { Monitor, Folder, FolderOpen, ChevronRight, ChevronDown, Pencil, Trash2, Plus } from 'lucide-vue-next';
import type { Connection, ConnectionGroup } from '../types';
import { useI18n } from '../composables/useI18n';
// import draggable from 'vuedraggable'; // Removed

const props = defineProps<{
    item: Connection | ConnectionGroup;
    level: number;
}>();

const emit = defineEmits(['connect', 'edit', 'delete', 'create-group', 'edit-group', 'delete-group', 'drag-start', 'drop-item', 'context-menu']);

const { t } = useI18n();
const isExpanded = ref(false);

const isGroup = computed(() => 'children' in props.item || 'parentId' in props.item);
const paddingLeft = computed(() => `${props.level * 16 + 8}px`);

const children = computed(() => (props.item as ConnectionGroup).children || []);
const localChildren = ref<(Connection | ConnectionGroup)[]>([]);

watch(children, (newVal) => {
    localChildren.value = [...newVal];
}, { immediate: true });

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

function handleContextMenu(event: MouseEvent) {
    emit('context-menu', event, props.item);
}

function getItemType(item: Connection | ConnectionGroup): 'connection' | 'group' {
    return ('children' in item) ? 'group' : 'connection';
}

function getItemKey(item: Connection | ConnectionGroup) {
    return getItemType(item) + '-' + item.id;
}

// Drag and Drop
const isDragOver = ref(false);
const isDragging = ref(false);

function onDragStart(event: DragEvent) {
    console.log('Drag start:', props.item.name, 'Type:', getItemType(props.item));
    isDragging.value = true;
    emit('drag-start', event, props.item);
    // Add visual feedback
    if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = 'move';
        const data = JSON.stringify({ type: getItemType(props.item), id: props.item.id });
        event.dataTransfer.setData('application/json', data);
        console.log('Set drag data:', data);
    }
}

function onDragEnd() {
    isDragging.value = false;
}

function onDragOver(event: DragEvent) {
    // Allow drop on both groups and connections
    event.preventDefault();
    event.dataTransfer!.dropEffect = 'move';

    // Always allow drop to provide visual feedback
    isDragOver.value = true;
    console.log('Drag over:', props.item.name, 'isGroup:', isGroup.value);
}

function onDragLeave(event: DragEvent) {
    // Only set false if actually leaving the element
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = event.clientX;
    const y = event.clientY;

    if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) {
        isDragOver.value = false;
    }
}

function onDrop(event: DragEvent) {
    console.log('Drop on:', props.item.name, 'isGroup:', isGroup.value);
    event.preventDefault();
    event.stopPropagation();
    isDragOver.value = false;

    const targetId = isGroup.value ? props.item.id : null;
    console.log('Emitting drop-item with targetId:', targetId);
    emit('drop-item', event, targetId);
}

</script>

<template>
    <div :draggable="true" @dragstart.stop="onDragStart" @dragend="onDragEnd" :class="{ 'opacity-50': isDragging }">
        <div class="group flex items-center justify-between p-2 hover:bg-gray-700 rounded cursor-pointer select-none transition-colors duration-200"
            :class="{ 'bg-blue-500/20 border border-blue-500': isDragOver, 'border-2 border-dashed border-blue-400': isDragOver && isGroup }"
            :style="{ paddingLeft }" @click="toggleExpand" @dblclick="handleConnect" @dragover="onDragOver"
            @dragleave="onDragLeave" @drop="onDrop" @contextmenu.stop.prevent="handleContextMenu">
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
            <div class="min-h-[10px]">
                <ConnectionTreeItem v-for="child in localChildren" :key="getItemKey(child)" :item="child"
                    :level="level + 1" @connect="$emit('connect', $event)" @edit="$emit('edit', $event)"
                    @delete="$emit('delete', $event)" @create-group="$emit('create-group', $event)"
                    @edit-group="$emit('edit-group', $event)" @delete-group="$emit('delete-group', $event)"
                    @drag-start="(e, i) => $emit('drag-start', e, i)" @drop-item="(e, id) => $emit('drop-item', e, id)"
                    @context-menu="(e, i) => $emit('context-menu', e, i)" />
            </div>
            <div v-if="!(item as ConnectionGroup).children?.length" class="text-xs text-gray-500 py-1"
                :style="{ paddingLeft: `${(level + 1) * 16 + 24}px` }">
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
