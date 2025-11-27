<script setup lang="ts">
import { useTransferStore } from '../stores/transfers';
import { X, Pause, RefreshCw, Trash2, FileUp, FileDown, ChevronUp, ChevronDown, Folder, Play, Square } from 'lucide-vue-next';
import { computed, ref } from 'vue';

const store = useTransferStore();
const isExpanded = ref(false);

const visible = computed(() => store.items.length > 0);

const summary = computed(() => {
    const total = store.items.length;
    const running = store.items.filter(i => i.status === 'running').length;
    const failed = store.items.filter(i => i.status === 'error').length;
    if (running > 0) return `${running} running`;
    if (failed > 0) return `${failed} failed`;
    return `${total} items`;
});

// 批量操作按钮的可见性计算
const canBatchPause = computed(() => store.items.some(i => i.status === 'running'));
const canBatchResume = computed(() => store.items.some(i => ['paused', 'error', 'cancelled'].includes(i.status)));
const canBatchCancel = computed(() => store.items.some(i => ['running', 'paused', 'pending'].includes(i.status)));
const canBatchDelete = computed(() => store.items.some(i => ['completed', 'cancelled', 'error', 'paused'].includes(i.status)));

function formatSize(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

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
                <button v-if="isExpanded && canBatchPause" @click="store.batchPause()" title="Batch Pause" class="p-0.5 hover:bg-gray-700 rounded text-gray-400">
                    <Pause class="w-3 h-3" />
                </button>
                <button v-if="isExpanded && canBatchResume" @click="store.batchResume()" title="Batch Resume" class="p-0.5 hover:bg-gray-700 rounded text-gray-400">
                    <Play class="w-3 h-3" />
                </button>
                <button v-if="isExpanded && canBatchCancel" @click="store.batchCancel()" title="Batch Cancel" class="p-0.5 hover:bg-gray-700 rounded text-yellow-400">
                    <Square class="w-3 h-3" />
                </button>
                <button v-if="isExpanded && canBatchDelete" @click="store.batchDelete()" title="Batch Delete" class="p-0.5 hover:bg-gray-700 rounded text-red-400">
                    <Trash2 class="w-3 h-3" />
                </button>
                <!-- 原有的清除已完成按钮 -->
                <button v-if="isExpanded" @click="store.clearHistory()" title="Clear Completed" class="p-0.5 hover:bg-gray-700 rounded text-gray-400">
                    <Trash2 class="w-3 h-3" />
                </button>
            </div>
        </div>

        <!-- List -->
        <div v-if="isExpanded" class="flex-1 overflow-y-auto p-2 space-y-2 bg-gray-900/50">
            <div v-for="item in store.items" :key="item.id" class="bg-gray-800 border border-gray-700 rounded p-2 text-xs">
                <div class="flex items-center justify-between mb-1">
                    <div class="flex items-center space-x-2 truncate">
                        <FileUp v-if="item.type === 'upload'" class="w-3 h-3 text-blue-400" />
                        <FileDown v-else-if="!item.isDirectory" class="w-3 h-3 text-green-400" />
                        <Folder v-else class="w-3 h-3 text-yellow-400" />
                        <span class="truncate font-medium text-gray-200" :title="item.name">{{ item.name }}</span>
                        <span v-if="item.isDirectory" class="text-xs text-gray-400">({{ item.completedFiles || 0 }}/{{ item.childFiles || 0 }} files)</span>
                    </div>
                    <span class="text-gray-400 whitespace-nowrap ml-2">
                        {{ item.status }}
                    </span>
                </div>
                
                <!-- Progress Bar -->
                <div class="h-1.5 bg-gray-900 rounded-full overflow-hidden mb-1 border border-gray-700">
                    <div 
                        class="h-full transition-all duration-300"
                        :class="{
                            'bg-blue-500': item.status === 'running',
                            'bg-yellow-500': item.status === 'paused',
                            'bg-green-500': item.status === 'completed',
                            'bg-red-500': item.status === 'error' || item.status === 'cancelled'
                        }"
                        :style="{ width: `${item.progress}%` }"
                    ></div>
                </div>
                
                <div class="flex items-center justify-between text-gray-400">
                    <span>{{ formatSize(item.transferred) }} / {{ formatSize(item.size) }}</span>
                    <div class="flex items-center space-x-1">
                        <button v-if="item.status === 'running' && !item.isDirectory" @click="store.pauseTransfer(item.id)" class="p-1 hover:text-white" title="Pause">
                            <Pause class="w-3 h-3" />
                        </button>
                        <button v-if="(item.status === 'paused' || item.status === 'error' || item.status === 'cancelled') && !item.isDirectory" @click="store.resumeTransfer(item.id)" class="p-1 hover:text-white" title="Resume/Retry">
                            <RefreshCw class="w-3 h-3" />
                        </button>
                        <button v-if="['running', 'paused', 'pending'].includes(item.status) && !item.isDirectory" @click="store.cancelTransfer(item.id)" class="p-1 hover:text-red-400" title="Cancel">
                            <X class="w-3 h-3" />
                        </button>
                        <button v-if="['completed', 'cancelled', 'error'].includes(item.status)" @click="store.removeTransfer(item.id)" class="p-1 hover:text-red-400" title="Remove">
                             <X class="w-3 h-3" />
                        </button>
                    </div>
                </div>
                <div v-if="item.error" class="text-red-400 mt-1 truncate" :title="item.error">
                    {{ item.error }}
                </div>
            </div>
        </div>
    </div>
</template>
