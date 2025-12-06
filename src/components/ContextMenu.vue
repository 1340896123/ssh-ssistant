<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';

export interface MenuItem {
    label: string;
    action: string;
    icon?: any; // Component type
    disabled?: boolean;
    danger?: boolean;
    separator?: boolean;
}

defineProps<{
    x: number;
    y: number;
    items: MenuItem[];
}>();

const emit = defineEmits(['close', 'action']);

const menuRef = ref<HTMLElement | null>(null);

function handleClickOutside(event: MouseEvent) {
    if (menuRef.value && !menuRef.value.contains(event.target as Node)) {
        emit('close');
    }
}

function handleItemClick(item: MenuItem) {
    if (item.disabled || item.separator) return;
    emit('action', item.action);
    emit('close');
}

onMounted(() => {
    document.addEventListener('mousedown', handleClickOutside);
    document.addEventListener('contextmenu', handleClickOutside); // also close on right click elsewhere
});

onUnmounted(() => {
    document.removeEventListener('mousedown', handleClickOutside);
    document.removeEventListener('contextmenu', handleClickOutside);
});
</script>

<template>
    <div ref="menuRef" class="fixed z-[100] bg-[#1f2937] border border-gray-700 rounded-lg shadow-xl py-1 min-w-[160px]"
        :style="{ top: `${y}px`, left: `${x}px` }" @contextmenu.prevent>
        <template v-for="(item, index) in items" :key="index">
            <div v-if="item.separator" class="my-1 border-t border-gray-700"></div>
            <button v-else
                class="w-full text-left px-4 py-2 text-sm flex items-center space-x-2 transition-colors hover:bg-gray-700"
                :class="{
                    'text-gray-400 cursor-not-allowed': item.disabled,
                    'text-red-400 hover:text-red-300': item.danger,
                    'text-gray-200': !item.disabled && !item.danger
                }" @click="handleItemClick(item)" :disabled="item.disabled">
                <component :is="item.icon" v-if="item.icon" class="w-4 h-4" />
                <span>{{ item.label }}</span>
            </button>
        </template>
    </div>
</template>
