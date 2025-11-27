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

const emit = defineEmits(['connect', 'edit', 'delete', 'create-group', 'edit-group', 'delete-group', 'drag-start', 'drop-item']);

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

function getItemType(item: Connection | ConnectionGroup): 'connection' | 'group' {
    return ('children' in item) ? 'group' : 'connection';
}

function getItemKey(item: Connection | ConnectionGroup) {
    return getItemType(item) + '-' + item.id;
}

// Drag and Drop
const isDragOver = ref(false);

function onDragStart(event: DragEvent) {
    emit('drag-start', event, props.item);
}

function onDragOver(event: DragEvent) {
    if (isGroup.value) {
        event.preventDefault();
        isDragOver.value = true;
    }
}

function onDragLeave(_: DragEvent) {
    isDragOver.value = false;
}

function onDrop(event: DragEvent) {
    if (isGroup.value) {
        event.preventDefault();
        event.stopPropagation(); // Prevent dropping on parent group
        isDragOver.value = false;
        emit('drop-item', event, props.item.id);
    }
}

</script>

<template>
    <div :draggable="true" @dragstart.stop="onDragStart">
        <div class="group flex items-center justify-between p-2 hover:bg-gray-700 rounded cursor-pointer select-none transition-colors duration-200"
            :class="{ 'bg-blue-500/20 border border-blue-500': isDragOver }" :style="{ paddingLeft }"
            @click="toggleExpand" @dblclick="handleConnect" @dragover="onDragOver" @dragleave="onDragLeave"
            @drop="onDrop">
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
                    @drag-start="(e, i) => $emit('drag-start', e, i)"
                    @drop-item="(e, id) => $emit('drop-item', e, id)" />
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
