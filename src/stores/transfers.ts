import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type TransferStatus = 'pending' | 'running' | 'paused' | 'completed' | 'error' | 'cancelled';

export interface TransferItem {
    id: string;
    type: 'upload' | 'download';
    name: string;
    localPath: string;
    remotePath: string;
    size: number;
    transferred: number;
    progress: number; // 0-100
    status: TransferStatus;
    error?: string;
    speed?: string;
    sessionId: string;
}

export const useTransferStore = defineStore('transfers', () => {
    const items = ref<TransferItem[]>([]);
    const active = ref(false);

    // Listen for progress events
    // We need to set this up globally once
    let unlisten: (() => void) | null = null;

    async function initListeners() {
        if (unlisten) return;
        unlisten = await listen('transfer-progress', (event: any) => {
            const payload = event.payload as { id: string, transferred: number, total: number };
            const item = items.value.find(i => i.id === payload.id);
            if (item && item.status === 'running') {
                item.transferred = payload.transferred;
                item.size = payload.total; // Update total if needed
                item.progress = Math.round((payload.transferred / payload.total) * 100);
            }
        });
    }

    function addTransfer(item: TransferItem) {
        items.value.unshift(item);
        void startTransfer(item.id);
    }

    async function startTransfer(id: string) {
        const item = items.value.find(i => i.id === id);
        if (!item) return;

        // If already completed, don't restart unless explicitly reset (not handled here)
        if (item.status === 'completed') return;

        item.status = 'running';
        item.error = undefined;
        active.value = true;
        
        try {
            if (item.type === 'upload') {
                await invoke('upload_file_with_progress', {
                    id: item.sessionId,
                    transferId: item.id,
                    localPath: item.localPath,
                    remotePath: item.remotePath,
                    resume: item.transferred > 0
                });
            } else {
                await invoke('download_file_with_progress', {
                    id: item.sessionId,
                    transferId: item.id,
                    remotePath: item.remotePath,
                    localPath: item.localPath,
                    resume: item.transferred > 0
                });
            }
            // Check if it was cancelled during transfer
            if ((item.status as string) === 'cancelled') return;
            
            item.status = 'completed';
            item.progress = 100;
        } catch (e: any) {
            // Check if it was cancelled during transfer
            // Use 'as string' to avoid type narrowing issues if TS is confused
            if ((item.status as string) === 'cancelled') return; 
            
            if (e.toString().includes('Cancelled')) {
                item.status = 'paused'; 
            } else {
                console.error(e);
                item.status = 'error';
                item.error = e.toString();
            }
        } finally {
            // Update active state
            active.value = items.value.some(i => i.status === 'running');
        }
    }

    async function pauseTransfer(id: string) {
        const item = items.value.find(i => i.id === id);
        if (!item || item.status !== 'running') return;

        // Call backend to cancel/stop the current task
        try {
            await invoke('cancel_transfer', { transferId: id });
            item.status = 'paused';
        } catch (e) {
            console.error("Failed to pause", e);
        }
    }

    function resumeTransfer(id: string) {
        startTransfer(id);
    }

    async function cancelTransfer(id: string) {
        const item = items.value.find(i => i.id === id);
        if (!item) return;

        if (item.status === 'running') {
             await invoke('cancel_transfer', { transferId: id });
        }
        item.status = 'cancelled';
    }

    function clearHistory() {
        // Remove completed, cancelled, error
        items.value = items.value.filter(i => ['running', 'pending', 'paused'].includes(i.status));
    }

    function removeTransfer(id: string) {
         const idx = items.value.findIndex(i => i.id === id);
         if (idx !== -1) items.value.splice(idx, 1);
    }

    return {
        items,
        addTransfer,
        pauseTransfer,
        resumeTransfer,
        cancelTransfer,
        clearHistory,
        removeTransfer,
        initListeners
    };
});
