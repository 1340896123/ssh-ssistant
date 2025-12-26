<script setup lang="ts">
import { useTransferStore } from '../stores/transfers';
import { useSessionStore } from '../stores/sessions';
import { X, Pause, RefreshCw, Trash2, FileUp, FileDown, ChevronUp, ChevronDown, Folder, Play, Square } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { useVirtualizer } from '@tanstack/vue-virtual';

const store = useTransferStore();
const sessionStore = useSessionStore();
const isExpanded = ref(false);
const virtualizerContainerRef = ref<HTMLElement>();

// 限制显示的项目数量以优化性能
const MAX_VISIBLE_ITEMS = 100;

// Only show items for the ACTIVE SESSION
const sessionItems = computed(() => {
    if (!sessionStore.activeSessionId) return [];
    return store.items.filter(i => i.sessionId === sessionStore.activeSessionId);
});

const visibleItems = computed(() => {
    const items = sessionItems.value;
    if (items.length <= MAX_VISIBLE_ITEMS) {
        return items;
    }
    return items.slice(0, MAX_VISIBLE_ITEMS);
});

const visible = computed(() => sessionItems.value.length > 0);

const summary = computed(() => {
    const total = sessionItems.value.length;
    const running = sessionItems.value.filter(i => i.status === 'running').length;
    const failed = sessionItems.value.filter(i => i.status === 'error').length;
    const hidden = Math.max(0, total - MAX_VISIBLE_ITEMS);
    let result = '';
    if (running > 0) result = `${running} running`;
    else if (failed > 0) result = `${failed} failed`;
    else result = `${total} items`;
    if (hidden > 0) result += ` (${hidden} hidden)`;
    return result;
});

// 批量操作按钮的可见性计算 (Scoped to session)
const canBatchPause = computed(() => sessionItems.value.some(i => i.status === 'running'));
const canBatchResume = computed(() => sessionItems.value.some(i => ['paused', 'error', 'cancelled'].includes(i.status)));
const canBatchCancel = computed(() => sessionItems.value.some(i => ['running', 'paused', 'pending'].includes(i.status)));
const canBatchDelete = computed(() => sessionItems.value.some(i => ['completed', 'cancelled', 'error', 'paused'].includes(i.status)));

function formatSize(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

const virtualizerOptions = {
    get count() { return visibleItems.value.length; },
    getScrollElement: () => virtualizerContainerRef.value as Element | null,
    estimateSize: () => 60, // 每行高度
    overscan: 3,
};

const virtualizer = useVirtualizer(virtualizerOptions);

const virtualItems = computed(() => virtualizer.value.getVirtualItems());

function toggleExpand() {
    isExpanded.value = !isExpanded.value;
}
</script>

<template>
    <div v-if="visible" class="border-t border-gray-700 bg-gray-800 flex flex-col transition-all duration-300" :class="{'h-48': isExpanded, 'h-8': !isExpanded}">
        <!-- Header / Summary Bar -->
        <div @click="toggleExpand" class="flex items-center justify-between px-2 h-8 bg-gray-900 cursor-pointer hover:bg-gray-800 select-none border-b border-gray-800">
            <div class="flex items-center space-x-2 text-xs text-gray-300">
                 <ChevronDown v-if="isExpanded" class="w-4 h-4" />
                 <ChevronUp v-else class="w-4 h-4" />
                 <span class="font-bold">Transfers</span>
                 <span class="bg-gray-700 px-1.5 rounded-full text-[10px]">{{ summary }}</span>
            </div>
            <div class="flex space-x-2" @click.stop>
                <!-- 批量操作按钮 -->
                <button v-if="isExpanded && canBatchPause" @click="store.batchPause(sessionStore.activeSessionId!)" title="Batch Pause" class="p-0.5 hover:bg-gray-700 rounded text-gray-400">
                    <Pause class="w-3 h-3" />
                </button>
                <button v-if="isExpanded && canBatchResume" @click="store.batchResume(sessionStore.activeSessionId!)" title="Batch Resume" class="p-0.5 hover:bg-gray-700 rounded text-gray-400">
                    <Play class="w-3 h-3" />
                </button>
                <button v-if="isExpanded && canBatchCancel" @click="store.batchCancel(sessionStore.activeSessionId!)" title="Batch Cancel" class="p-0.5 hover:bg-gray-700 rounded text-yellow-400">
                    <Square class="w-3 h-3" />
                </button>
                <button v-if="isExpanded && canBatchDelete" @click="store.batchDelete(sessionStore.activeSessionId!)" title="Batch Delete" class="p-0.5 hover:bg-gray-700 rounded text-red-400">
                    <Trash2 class="w-3 h-3" />
                </button>
                <!-- 原有的清除已完成按钮 -->
                <button v-if="isExpanded" @click="store.clearHistory(sessionStore.activeSessionId!)" title="Clear Completed" class="p-0.5 hover:bg-gray-700 rounded text-gray-400">
                    <Trash2 class="w-3 h-3" />
                </button>
            </div>
        </div>

        <!-- List -->
        <div v-if="isExpanded" ref="virtualizerContainerRef" class="flex-1 overflow-y-auto p-2 space-y-2 bg-gray-900/50">
            <div 
                :style="{ height: virtualizer.getTotalSize() + 'px', width: '100%', position: 'relative' }"
            >
                <div
                    v-for="virtualItem in virtualItems"
                    :key="visibleItems[virtualItem.index].id"
                    :style="{
                        position: 'absolute',
                        top: 0,
                        left: 0,
                        width: '100%',
                        height: `${virtualItem.size}px`,
                        transform: `translateY(${virtualItem.start}px)`,
                    }"
                >
                    <div class="bg-gray-800 border border-gray-700 rounded p-2 text-xs h-full">
                        <div class="flex items-center justify-between mb-1">
                            <div class="flex items-center space-x-2 truncate">
                                <FileUp v-if="visibleItems[virtualItem.index].type === 'upload'" class="w-3 h-3 text-blue-400" />
                                <FileDown v-else-if="!visibleItems[virtualItem.index].isDirectory" class="w-3 h-3 text-green-400" />
                                <Folder v-else class="w-3 h-3 text-yellow-400" />
                                <span class="truncate font-medium text-gray-200" :title="visibleItems[virtualItem.index].name">{{ visibleItems[virtualItem.index].name }}</span>
                                <span v-if="visibleItems[virtualItem.index].isDirectory" class="text-xs text-gray-400">({{ visibleItems[virtualItem.index].completedFiles || 0 }}/{{ visibleItems[virtualItem.index].childFiles || 0 }} files)</span>
                            </div>
                            <span class="text-gray-400 whitespace-nowrap ml-2">
                                {{ visibleItems[virtualItem.index].status }}
                            </span>
                        </div>
                        
                        <!-- Progress Bar -->
                        <div class="h-1.5 bg-gray-900 rounded-full overflow-hidden mb-1 border border-gray-700">
                            <div 
                                class="h-full transition-all duration-300"
                                :class="{
                                    'bg-blue-500': visibleItems[virtualItem.index].status === 'running',
                                    'bg-yellow-500': visibleItems[virtualItem.index].status === 'paused',
                                    'bg-green-500': visibleItems[virtualItem.index].status === 'completed',
                                    'bg-red-500': visibleItems[virtualItem.index].status === 'error' || visibleItems[virtualItem.index].status === 'cancelled'
                                }"
                                :style="{ width: `${visibleItems[virtualItem.index].progress}%` }"
                            ></div>
                        </div>
                        
                        <div class="flex items-center justify-between text-gray-400">
                            <span>{{ formatSize(visibleItems[virtualItem.index].transferred) }} / {{ formatSize(visibleItems[virtualItem.index].size) }}</span>
                            <div class="flex items-center space-x-1">
                                <button v-if="visibleItems[virtualItem.index].status === 'running' && !visibleItems[virtualItem.index].isDirectory" @click="store.pauseTransfer(visibleItems[virtualItem.index].id)" class="p-1 hover:text-white" title="Pause">
                                    <Pause class="w-3 h-3" />
                                </button>
                                <button v-if="(visibleItems[virtualItem.index].status === 'paused' || visibleItems[virtualItem.index].status === 'error' || visibleItems[virtualItem.index].status === 'cancelled') && !visibleItems[virtualItem.index].isDirectory" @click="store.resumeTransfer(visibleItems[virtualItem.index].id)" class="p-1 hover:text-white" title="Resume/Retry">
                                    <RefreshCw class="w-3 h-3" />
                                </button>
                                <button v-if="['running', 'paused', 'pending'].includes(visibleItems[virtualItem.index].status) && !visibleItems[virtualItem.index].isDirectory" @click="store.cancelTransfer(visibleItems[virtualItem.index].id)" class="p-1 hover:text-red-400" title="Cancel">
                                    <X class="w-3 h-3" />
                                </button>
                                <button v-if="['completed', 'cancelled', 'error'].includes(visibleItems[virtualItem.index].status)" @click="store.removeTransfer(visibleItems[virtualItem.index].id)" class="p-1 hover:text-red-400" title="Remove">
                                     <X class="w-3 h-3" />
                                </button>
                            </div>
                        </div>
                        <div v-if="visibleItems[virtualItem.index].error" class="text-red-400 mt-1 truncate" :title="visibleItems[virtualItem.index].error">
                            {{ visibleItems[virtualItem.index].error }}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
