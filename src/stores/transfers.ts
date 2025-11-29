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
    isDirectory?: boolean; // 新增：标识是否为目录
    childFiles?: number; // 新增：子文件数量（用于目录）
    completedFiles?: number; // 新增：已完成文件数量（用于目录）
}

export const useTransferStore = defineStore('transfers', () => {
    const items = ref<TransferItem[]>([]);
    const active = ref(false);
    const maxConcurrent = 3; // 最大并发数，建议与后端 SessionPool 大小一致

    // Listen for progress events
    // We need to set this up globally once
    let unlisten: (() => void) | null = null;
    
    // 目录下载进度跟踪
    const directoryProgress = new Map<string, {
        totalFiles: number;
        completedFiles: number;
        totalSize: number;
        transferredSize: number;
        isPaused: boolean; // 新增：暂停状态
        pausedFiles: Set<string>; // 新增：已暂停的文件列表
    }>();

    async function initListeners() {
        if (unlisten) return;
        unlisten = await listen('transfer-progress', (event: any) => {
            const payload = event.payload as { id: string, transferred: number, total: number };
            const item = items.value.find(i => i.id === payload.id);
            if (item && item.status === 'running') {
                item.transferred = payload.transferred;
                item.size = payload.total; // Update total if needed
                item.progress = Math.round((payload.transferred / payload.total) * 100);
                
                // 如果是目录传输的一部分，更新目录总体进度
                updateDirectoryProgress(item.id, payload.transferred, payload.total);
            }
        });
    }
    
    function updateDirectoryProgress(fileTransferId: string, transferred: number, _total: number) {
        // 查找包含此文件的目录传输项
        const directoryItem = items.value.find(item => 
            item.isDirectory && item.remotePath && fileTransferId.startsWith(item.remotePath)
        );
        
        if (directoryItem) {
            const progress = directoryProgress.get(directoryItem.id);
            if (progress) {
                // 更新目录总体进度
                progress.transferredSize += transferred;
                directoryItem.transferred = progress.transferredSize;
                directoryItem.progress = Math.round((progress.transferredSize / progress.totalSize) * 100);
            }
        }
    }

    // 新增：处理队列的函数
    function processQueue() {
        const runningCount = items.value.filter(i => i.status === 'running').length;
        if (runningCount >= maxConcurrent) return;

        // 找到下一个待处理的任务
        const nextItem = items.value.find(i => i.status === 'pending');
        if (nextItem) {
            void startTransfer(nextItem.id);
        }
    }

    function addTransfer(item: TransferItem) {
        items.value.unshift(item);
        if (!item.isDirectory) {
            // 尝试处理队列
            processQueue();
        }
    }
    
    function addDirectoryTransfer(remotePath: string, localPath: string, sessionId: string) {
        const dirName = remotePath.split('/').pop() || 'directory';
        const transferId = typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);
        
        // 创建目录传输项
        const directoryItem: TransferItem = {
            id: transferId,
            type: 'download',
            name: dirName,
            localPath,
            remotePath,
            size: 0,
            transferred: 0,
            progress: 0,
            status: 'pending',
            sessionId,
            isDirectory: true,
            childFiles: 0,
            completedFiles: 0
        };
        
        items.value.unshift(directoryItem);
        return transferId;
    }
    
    function updateDirectoryStats(directoryId: string, totalFiles: number, totalSize: number) {
        const item = items.value.find(i => i.id === directoryId);
        if (item && item.isDirectory) {
            item.childFiles = totalFiles;
            item.size = totalSize;
            
            // 初始化进度跟踪
            directoryProgress.set(directoryId, {
                totalFiles,
                completedFiles: 0,
                totalSize,
                transferredSize: 0,
                isPaused: false,
                pausedFiles: new Set()
            });
        }
    }
    
    function incrementDirectoryCompleted(directoryId: string) {
        const progress = directoryProgress.get(directoryId);
        const item = items.value.find(i => i.id === directoryId);
        
        if (progress && item && item.isDirectory) {
            progress.completedFiles++;
            item.completedFiles = progress.completedFiles;
            
            // 如果所有文件都完成了，标记目录为完成
            if (progress.completedFiles >= progress.totalFiles) {
                item.status = 'completed';
                item.progress = 100;
                item.transferred = progress.totalSize;
            }
        }
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
            // 关键：无论成功失败，触发队列处理下一个
            processQueue();
            
            // Update active state
            active.value = items.value.some(i => i.status === 'running');
        }
    }

    async function pauseTransfer(id: string) {
        const item = items.value.find(i => i.id === id);
        if (!item) return;
        
        // 目录传输的暂停处理
        if (item.isDirectory) {
            const progress = directoryProgress.get(id);
            if (progress) {
                progress.isPaused = true;
                item.status = 'paused';
                
                // 暂停所有正在运行的子文件传输
                const runningChildFiles = items.value.filter(i => 
                    !i.isDirectory && 
                    i.status === 'running' && 
                    i.remotePath && i.remotePath.startsWith(item.remotePath!)
                );
                
                for (const childFile of runningChildFiles) {
                    try {
                        await invoke('cancel_transfer', { transferId: childFile.id });
                        childFile.status = 'paused';
                        progress.pausedFiles.add(childFile.id);
                    } catch (e) {
                        console.error('Failed to pause child file:', childFile.id, e);
                    }
                }
            }
            return;
        }
        
        // 普通文件传输的暂停处理
        if (item.status !== 'running') return;

        try {
            await invoke('cancel_transfer', { transferId: id });
            item.status = 'paused';
        } catch (e) {
            console.error("Failed to pause", e);
        }
    }

    function resumeTransfer(id: string) {
        const item = items.value.find(i => i.id === id);
        if (!item) return;
        
        // 目录传输的恢复处理
        if (item.isDirectory) {
            const progress = directoryProgress.get(id);
            if (progress && progress.isPaused) {
                progress.isPaused = false;
                item.status = 'running';
                
                // 恢复所有已暂停的子文件传输
                for (const childFileId of progress.pausedFiles) {
                    const childFile = items.value.find(i => i.id === childFileId);
                    if (childFile && childFile.status === 'paused') {
                        startTransfer(childFileId);
                    }
                }
                progress.pausedFiles.clear();
            }
            return;
        }
        
        // 普通文件传输的恢复处理
        startTransfer(id);
    }

    async function cancelTransfer(id: string) {
        const item = items.value.find(i => i.id === id);
        if (!item) return;

        // 目录传输的取消处理
        if (item.isDirectory) {
            const progress = directoryProgress.get(id);
            if (progress) {
                progress.isPaused = false;
                item.status = 'cancelled';
                
                // 取消所有子文件传输
                const childFiles = items.value.filter(i => 
                    !i.isDirectory && 
                    i.remotePath && i.remotePath.startsWith(item.remotePath!)
                );
                
                for (const childFile of childFiles) {
                    if (childFile.status === 'running') {
                        try {
                            await invoke('cancel_transfer', { transferId: childFile.id });
                        } catch (e) {
                            console.error('Failed to cancel child file:', childFile.id, e);
                        }
                    }
                    childFile.status = 'cancelled';
                }
                
                progress.pausedFiles.clear();
            }
            return;
        }

        // 普通文件传输的取消处理
        if (item.status === 'running') {
             await invoke('cancel_transfer', { transferId: id });
        }
        item.status = 'cancelled';
    }

    function clearHistory() {
        // Remove completed, cancelled, error
        items.value = items.value.filter(i => ['running', 'pending', 'paused'].includes(i.status));
    }
    
    // 批量操作函数
    async function batchPause() {
        const runningItems = items.value.filter(i => i.status === 'running');
        await Promise.all(runningItems.map(item => pauseTransfer(item.id)));
    }
    
    function batchResume() {
        const pausedItems = items.value.filter(i => ['paused', 'error', 'cancelled'].includes(i.status));
        pausedItems.forEach(item => resumeTransfer(item.id));
    }
    
    async function batchCancel() {
        const activeItems = items.value.filter(i => ['running', 'paused', 'pending'].includes(i.status));
        await Promise.all(activeItems.map(item => cancelTransfer(item.id)));
    }
    
    async function batchDelete() {
        // 删除所有非活动状态的传输项
        const deletableItems = items.value.filter(i => ['completed', 'cancelled', 'error', 'paused'].includes(i.status));
        await Promise.all(deletableItems.map(item => removeTransfer(item.id)));
    }

    function removeTransfer(id: string) {
         const idx = items.value.findIndex(i => i.id === id);
         if (idx !== -1) items.value.splice(idx, 1);
    }

    return {
        items,
        addTransfer,
        addDirectoryTransfer,
        updateDirectoryStats,
        incrementDirectoryCompleted,
        pauseTransfer,
        resumeTransfer,
        cancelTransfer,
        clearHistory,
        batchPause,
        batchResume,
        batchCancel,
        batchDelete,
        removeTransfer,
        initListeners,
        processQueue // 导出以便手动触发（可选）
    };
});
