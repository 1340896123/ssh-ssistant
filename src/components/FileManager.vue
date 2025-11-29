<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, nextTick, shallowRef, triggerRef } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ArrowUp, RefreshCw, Upload, FilePlus, FolderPlus, Briefcase } from 'lucide-vue-next';
import { open, save, ask } from '@tauri-apps/plugin-dialog';
import { readDir, mkdir, stat } from '@tauri-apps/plugin-fs';
import type { FileEntry, FileManagerViewMode } from '../types';
import { useSessionStore } from '../stores/sessions'; // Import session store
import { useNotificationStore } from '../stores/notifications';
import { useTransferStore } from '../stores/transfers';
import { useSettingsStore } from '../stores/settings';
import TransferList from './TransferList.vue';
import VirtualFileList from './VirtualFileList.vue';
import { useI18n } from '../composables/useI18n';
// import draggable from 'vuedraggable'; // Removed

type ColumnKey = 'name' | 'size' | 'date' | 'owner';

interface TreeNode {
    entry: FileEntry;
    path: string;
    depth: number;
    parentPath: string | null;
    childrenLoaded: boolean;
    loading: boolean;
}

function showTreeContextMenu(e: MouseEvent, node: TreeNode) {
    e.preventDefault();

    const next = new Set(selectedTreePaths.value);
    if (!next.has(node.path)) {
        next.clear();
        next.add(node.path);
        selectedTreePaths.value = next;
    }

    const { x, y } = calculateContextMenuPosition(e.clientX, e.clientY);
    contextMenu.value = { show: true, x, y, file: node.entry, treePath: node.path, isTree: true, isBackground: false };
    updateContextMenuPosition();
}

function calculateContextMenuPosition(clientX: number, clientY: number) {
    // Return raw coordinates and let updateContextMenuPosition handle boundaries
    // after the menu is rendered and we know its actual size
    return { x: clientX, y: clientY };
}

async function updateContextMenuPosition() {
    await nextTick();
    if (!contextMenuRef.value) return;

    const rect = contextMenuRef.value.getBoundingClientRect();
    let { x, y } = contextMenu.value;
    
    // Adjust if overflowing
    if (rect.bottom > window.innerHeight) {
        y = window.innerHeight - rect.height - 10; // 10px padding
    }
    if (rect.right > window.innerWidth) {
        x = window.innerWidth - rect.width - 10;
    }
    
    // Ensure not negative (top/left)
    x = Math.max(0, x);
    y = Math.max(0, y);

    contextMenu.value.x = x;
    contextMenu.value.y = y;
}

function handleContainerContextMenu(e: MouseEvent) {
    // 检查点击的是否是背景区域（不是文件项）
    const target = e.target as HTMLElement;
    const fileItem = target.closest('[data-file-item]');
    
    if (!fileItem) {
        // 点击的是背景区域
        e.preventDefault();
        showBackgroundContextMenu(e);
    }
    // 如果点击的是文件项，让文件项自己处理右键菜单
}

function showBackgroundContextMenu(e: MouseEvent) {
    e.preventDefault();
    // If clicking on background, clear file selection unless Ctrl/Shift is held (handled by other listeners? usually background click clears selection)
    // For now, just show menu.
    // Maybe clear selection?
    if (!e.ctrlKey && !e.shiftKey) {
        selectedFiles.value.clear();
        selectedTreePaths.value.clear();
    }

    const { x, y } = calculateContextMenuPosition(e.clientX, e.clientY);
    contextMenu.value = { show: true, x, y, file: null, treePath: null, isTree: viewMode.value === 'tree', isBackground: true };
    updateContextMenuPosition();
}

const props = defineProps<{ sessionId: string }>();
const { t } = useI18n();
const settingsStore = useSettingsStore();
const sessionStore = useSessionStore(); // Init session store
const notificationStore = useNotificationStore();
const viewMode = computed<FileManagerViewMode>(() => settingsStore.fileManager.viewMode);
const currentPath = ref('.');
const files = shallowRef<FileEntry[]>([]);
const contextMenu = ref<{ show: boolean, x: number, y: number, file: FileEntry | null, treePath: string | null, isTree: boolean, isBackground: boolean }>({ show: false, x: 0, y: 0, file: null, treePath: null, isTree: false, isBackground: false });
const contextMenuRef = ref<HTMLElement | null>(null);
const isEditingPath = ref(false);
const pathInput = ref('');
const containerRef = ref<HTMLElement | null>(null);
const selectedFiles = ref<Set<string>>(new Set());
const lastSelectedIndex = ref<number>(-1);
const transferStore = useTransferStore();
const treeRootPath = ref<string>('.');
const treeNodes = shallowRef<Map<string, TreeNode>>(new Map());
const childrenMap = shallowRef<Map<string | null, string[]>>(new Map());
const expandedPaths = ref<Set<string>>(new Set());
const selectedTreePaths = ref<Set<string>>(new Set());
const columnWidths = ref<Record<ColumnKey, number>>({
    name: 260,
    size: 100,
    date: 200,
    owner: 120,
});
const resizingColumn = ref<ColumnKey | null>(null);
const resizeStartX = ref(0);
const resizeStartWidth = ref(0);

const isOpeningFile = ref(false);
const unlistenDrop = ref<UnlistenFn | null>(null);

const visibleTreeNodes = computed<TreeNode[]>(() => {
    const result: TreeNode[] = [];
    const nodes = treeNodes.value;
    const children = childrenMap.value;

    const collectChildren = (parentPath: string | null, depth: number) => {
        const childPaths = children.get(parentPath) || [];
        const childNodes: TreeNode[] = [];
        
        for (const childPath of childPaths) {
            const node = nodes.get(childPath);
            if (node) {
                childNodes.push({ ...node, depth });
            }
        }

        childNodes.sort((a, b) => {
            if (a.entry.isDir === b.entry.isDir) {
                return a.entry.name.localeCompare(b.entry.name);
            }
            return a.entry.isDir ? -1 : 1;
        });

        for (const child of childNodes) {
            result.push(child);
            if (child.entry.isDir && expandedPaths.value.has(child.path)) {
                collectChildren(child.path, depth + 1);
            }
        }
    };

    collectChildren(treeRootPath.value === '.' ? null : treeRootPath.value, 0);
    return result;
});



function joinPath(parent: string, child: string): string {
    if (parent === '.') return child;
    return parent.endsWith('/') ? `${parent}${child}` : `${parent}/${child}`;
}

function onDragStart(event: DragEvent, element: FileEntry | TreeNode) {
    let entry: FileEntry;
    let path: string;

    if ('entry' in element) { // TreeNode
        entry = (element as TreeNode).entry;
        path = (element as TreeNode).path;
    } else { // FileEntry
        entry = element as FileEntry;
        path = joinPath(currentPath.value, entry.name);
    }

    const data = {
        path: path,
        isDir: entry.isDir
    };

    if (event.dataTransfer) {
        event.dataTransfer.setData('application/json', JSON.stringify(data));
        event.dataTransfer.effectAllowed = 'copy';
    }
}

async function loadFiles(path: string) {
    try {
        files.value = await invoke<FileEntry[]>('list_files', { id: props.sessionId, path });
        triggerRef(files);
        currentPath.value = path;
        pathInput.value = path;
        selectedFiles.value.clear();
        lastSelectedIndex.value = -1;

        if (viewMode.value === 'tree') {
            treeRootPath.value = path;
            treeNodes.value = new Map();
            childrenMap.value = new Map();
            expandedPaths.value = new Set();
            selectedTreePaths.value = new Set();
            const parentPath = path === '.' ? null : path;
            const newChildrenMap = new Map<string | null, string[]>();
            const childPaths: string[] = [];
            
            for (const entry of files.value) {
                const fullPath = joinPath(path, entry.name);
                treeNodes.value.set(fullPath, {
                    entry,
                    path: fullPath,
                    depth: 0,
                    parentPath,
                    childrenLoaded: false,
                    loading: false,
                });
                childPaths.push(fullPath);
            }
            
            newChildrenMap.set(parentPath, childPaths);
            childrenMap.value = newChildrenMap;
            triggerRef(treeNodes);
            triggerRef(childrenMap);
        }
    } catch (e) {
        console.error(e);
        files.value = [];
        triggerRef(files);
    }
}


async function toggleDirectory(node: TreeNode) {
    if (!node.entry.isDir) {
        await openTreeFile(node);
        return;
    }

    const isExpanded = expandedPaths.value.has(node.path);
    if (isExpanded) {
        expandedPaths.value.delete(node.path);
        return;
    }

    expandedPaths.value.add(node.path);

    if (node.childrenLoaded || node.loading) {
        return;
    }

    const existing = treeNodes.value.get(node.path);
    if (!existing) return;
    existing.loading = true;
    treeNodes.value.set(node.path, existing);
    triggerRef(treeNodes);

    try {
        const children = await invoke<FileEntry[]>('list_files', { id: props.sessionId, path: node.path });
        const currentChildrenMap = new Map(childrenMap.value);
        const childPaths: string[] = [];
        
        for (const child of children) {
            const childPath = joinPath(node.path, child.name);
            if (!treeNodes.value.has(childPath)) {
                treeNodes.value.set(childPath, {
                    entry: child,
                    path: childPath,
                    depth: node.depth + 1,
                    parentPath: node.path,
                    childrenLoaded: false,
                    loading: false,
                });
            }
            childPaths.push(childPath);
        }
        
        currentChildrenMap.set(node.path, childPaths);
        childrenMap.value = currentChildrenMap;
        existing.childrenLoaded = true;
        treeNodes.value.set(node.path, existing);
        triggerRef(treeNodes);
        triggerRef(childrenMap);
    } catch (e) {
        console.error(e);
    } finally {
        const updated = treeNodes.value.get(node.path);
        if (updated) {
            updated.loading = false;
            treeNodes.value.set(node.path, updated);
            triggerRef(treeNodes);
        }
    }
}

async function openTreeFile(node: TreeNode) {
    if (node.entry.isDir) {
        await toggleDirectory(node);
        return;
    }
    isOpeningFile.value = true;
    try {
        await invoke('edit_remote_file', {
            id: props.sessionId,
            remotePath: node.path,
            remoteName: node.entry.name,
        });
    } catch (e) {
        notificationStore.error("Failed to open file: " + e);
    } finally {
        isOpeningFile.value = false;
    }
}

function handleTreeSelection(node: TreeNode) {
    closeContextMenu();
    const next = new Set(selectedTreePaths.value);
    if (next.has(node.path)) {
        next.delete(node.path);
    } else {
        next.clear();
        next.add(node.path);
    }
    selectedTreePaths.value = next;
}

function startResize(column: ColumnKey, event: MouseEvent) {
    resizingColumn.value = column;
    resizeStartX.value = event.clientX;
    resizeStartWidth.value = columnWidths.value[column];
}

let resizeAnimationFrame: number | null = null;

function handleMouseMove(event: MouseEvent) {
    if (!resizingColumn.value) return;
    
    if (!resizeAnimationFrame) {
        resizeAnimationFrame = requestAnimationFrame(() => {
            const delta = event.clientX - resizeStartX.value;
            const newWidth = Math.max(80, resizeStartWidth.value + delta);
            columnWidths.value[resizingColumn.value!] = newWidth;
            resizeAnimationFrame = null;
        });
    }
}

function stopResize() {
    resizingColumn.value = null;
}

// Handle native browser drag and drop for files
function handleNativeDragOver(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();

    // Only accept file drops, not connection drags
    if (event.dataTransfer && event.dataTransfer.types.includes('Files')) {
        event.dataTransfer.dropEffect = 'copy';
    }
}

function handleNativeDrop(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();
    // Handled by tauri://drag-drop listener
}

async function handleTauriFileDrop(paths: string[]) {
    for (const fullPath of paths) {
        const name = fullPath.split(/[\\/]/).pop() || 'uploaded';
        const remotePath = joinPath(currentPath.value, name);

        try {
            // Check if it's a directory
            const fileStat = await stat(fullPath);

            if (fileStat.isDirectory) {
                try {
                    await invoke('create_directory', { id: props.sessionId, path: remotePath });
                } catch (e) {
                    // Ignore if directory already exists
                }
                await processDirectory(fullPath, remotePath);
            } else {
                const transferId = typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);

                transferStore.addTransfer({
                    id: transferId,
                    type: 'upload',
                    name,
                    localPath: fullPath,
                    remotePath,
                    size: fileStat.size,
                    transferred: 0,
                    progress: 0,
                    status: 'pending',
                    sessionId: props.sessionId
                });
            }
        } catch (e) {
            console.error('Error processing dropped item:', name, e);
            notificationStore.error(`Failed to process ${name}: ${e}`);
        }
    }

    loadFiles(currentPath.value);
}

onMounted(async () => {
    loadFiles('.');
    transferStore.initListeners();
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', stopResize);
    window.addEventListener('keydown', handleKeyDown);

    unlistenDrop.value = await listen('tauri://drag-drop', (event) => {
        const payload = event.payload as { paths: string[], position: { x: number, y: number } };
        if (containerRef.value) {
            const rect = containerRef.value.getBoundingClientRect();
            // Check if drop is within the file manager container
            if (payload.position.x >= rect.left &&
                payload.position.x <= rect.right &&
                payload.position.y >= rect.top &&
                payload.position.y <= rect.bottom) {
                handleTauriFileDrop(payload.paths);
            }
        }
    });
});

onUnmounted(() => {
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', stopResize);
    window.removeEventListener('keydown', handleKeyDown);
    if (unlistenDrop.value) {
        unlistenDrop.value();
    }
});

watch(viewMode, (mode) => {
    if (mode === 'tree') {
        loadFiles(currentPath.value);
    }
});

async function navigate(entry: FileEntry) {
    if (entry.isDir) {
        const newPath = joinPath(currentPath.value, entry.name);
        loadFiles(newPath);
    } else {
        // Edit remote file
        isOpeningFile.value = true;
        try {
            await invoke('edit_remote_file', {
                id: props.sessionId,
                remotePath: joinPath(currentPath.value, entry.name),
                remoteName: entry.name
            });
        } catch (e) {
            alert("Failed to open file: " + e);
        } finally {
            isOpeningFile.value = false;
        }
    }
}

function goUp() {
    if (currentPath.value === '.') return;
    const parts = currentPath.value.split('/');
    parts.pop();
    const newPath = parts.join('/') || '.';
    loadFiles(newPath);
}

function refresh() {
    loadFiles(currentPath.value);
}

function handlePathSubmit() {
    if (pathInput.value && pathInput.value !== currentPath.value) {
        loadFiles(pathInput.value);
    }
    isEditingPath.value = false;
}

function handleSelection(event: MouseEvent, file: FileEntry, index: number) {
    closeContextMenu();
    if (event.ctrlKey || event.metaKey) {
        // Toggle selection
        if (selectedFiles.value.has(file.name)) {
            selectedFiles.value.delete(file.name);
        } else {
            selectedFiles.value.add(file.name);
            lastSelectedIndex.value = index;
        }
    } else if (event.shiftKey && lastSelectedIndex.value !== -1) {
        // Range selection
        const start = Math.min(lastSelectedIndex.value, index);
        const end = Math.max(lastSelectedIndex.value, index);
        selectedFiles.value.clear();
        for (let i = start; i <= end; i++) {
            if (files.value[i]) {
                selectedFiles.value.add(files.value[i].name);
            }
        }
    } else {
        // Single selection
        selectedFiles.value.clear();
        selectedFiles.value.add(file.name);
        lastSelectedIndex.value = index;
    }
}

function showContextMenu(e: MouseEvent, file: FileEntry) {
    e.preventDefault();
    // If the file is not in selection, select it exclusively
    if (!selectedFiles.value.has(file.name)) {
        selectedFiles.value.clear();
        selectedFiles.value.add(file.name);
        const idx = files.value.findIndex(f => f.name === file.name);
        if (idx !== -1) lastSelectedIndex.value = idx;
    }

    const { x, y } = calculateContextMenuPosition(e.clientX, e.clientY);
    contextMenu.value = { show: true, x, y, file, treePath: null, isTree: false, isBackground: false };
    updateContextMenuPosition();
}

function closeContextMenu() {
    contextMenu.value.show = false;
}

async function handleUpload() {
    try {
        const selected = await open({
            multiple: false,
            title: 'Select file to upload'
        });
        if (selected && typeof selected === 'string') {
            const name = selected.split(/[\\/]/).pop() || 'uploaded_file';
            const remotePath = joinPath(currentPath.value, name);

            const transferId = typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);

            transferStore.addTransfer({
                id: transferId,
                type: 'upload',
                name,
                localPath: selected,
                remotePath,
                size: 0,
                transferred: 0,
                progress: 0,
                status: 'pending',
                sessionId: props.sessionId
            });

            loadFiles(currentPath.value);
        }
    } catch (e) {
        console.error(e);
        notificationStore.error("Upload failed: " + e);
    }
}

async function processDirectory(localPath: string, remoteBasePath: string) {
    const entries = await readDir(localPath);
    for (const entry of entries) {
        const sep = localPath.includes('\\') ? '\\' : '/';
        const fullLocalPath = localPath.endsWith(sep) ? `${localPath}${entry.name}` : `${localPath}${sep}${entry.name}`;
        const fullRemotePath = `${remoteBasePath}/${entry.name}`;

        if (entry.isDirectory) {
            try {
                await invoke('create_directory', { id: props.sessionId, path: fullRemotePath });
            } catch (e) {
                // Ignore if directory already exists
            }
            await processDirectory(fullLocalPath, fullRemotePath);
        } else {
            const transferId = typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);
            transferStore.addTransfer({
                id: transferId,
                type: 'upload',
                name: entry.name,
                localPath: fullLocalPath,
                remotePath: fullRemotePath,
                size: 0, // Backend will calculate
                transferred: 0,
                progress: 0,
                status: 'pending',
                sessionId: props.sessionId
            });
        }
    }
}

async function handleUploadDirectory() {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Select directory to upload'
        });

        if (selected && typeof selected === 'string') {
            const name = selected.split(/[\\/]/).pop() || 'uploaded_dir';
            const remotePath = joinPath(currentPath.value, name);

            try {
                await invoke('create_directory', { id: props.sessionId, path: remotePath });
            } catch (e) {
                // Ignore
            }

            await processDirectory(selected, remotePath);
            loadFiles(currentPath.value);
        }
    } catch (e) {
        console.error(e);
        notificationStore.error("Upload directory failed: " + e);
    }
}

async function handleSetWorkspace() {
    let path = '';

    if (contextMenu.value.isBackground) {
        path = currentPath.value;
    } else if (contextMenu.value.file?.isDir) {
        path = contextMenu.value.isTree && contextMenu.value.treePath
            ? contextMenu.value.treePath
            : (currentPath.value === '.' ? contextMenu.value.file.name : `${currentPath.value}/${contextMenu.value.file.name}`);
    } else {
        return;
    }

    try {
        await sessionStore.setSessionWorkspace(props.sessionId, path);
        useNotificationStore().success(`Workspace set to: ${path}`);
         // Switch to AI tab?
        sessionStore.setActiveTab('ai');
        //await message(`Workspace set to: ${path}`, { title: 'Success', kind: 'info' });
       
    } catch (e) {
        notificationStore.error(`Failed to set workspace: ${e}`);
    }
    closeContextMenu();
}

async function downloadDirectory(remoteDirPath: string, localDirPath: string, sessionId: string) {
    try {
        // Create local directory if it doesn't exist
        try {
            await mkdir(localDirPath, { recursive: true });
        } catch (e) {
            // Directory might already exist, continue
            console.log('Directory might already exist:', e);
        }

        // 创建目录传输项
        const directoryTransferId = transferStore.addDirectoryTransfer(remoteDirPath, localDirPath, sessionId);

        // 扫描目录并计算统计信息
        const { totalFiles, totalSize } = await calculateDirectoryStats(remoteDirPath, sessionId);
        transferStore.updateDirectoryStats(directoryTransferId, totalFiles, totalSize);

        // 开始传输
        const directoryItem = transferStore.items.find(item => item.id === directoryTransferId);
        if (directoryItem) {
            directoryItem.status = 'running';
        }

        // 递归下载所有文件
        await downloadDirectoryRecursive(remoteDirPath, localDirPath, sessionId, directoryTransferId);

        // 标记目录传输为完成
        if (directoryItem) {
            directoryItem.status = 'completed';
            directoryItem.progress = 100;
        }
    } catch (e) {
        console.error(`Failed to download directory ${remoteDirPath}:`, e);
        // 标记目录传输为失败
        const directoryItem = transferStore.items.find(item => item.remotePath === remoteDirPath && item.isDirectory);
        if (directoryItem) {
            directoryItem.status = 'error';
            directoryItem.error = (e as Error).toString();
        }
        throw e;
    }
}

async function calculateDirectoryStats(remotePath: string, sessionId: string): Promise<{ totalFiles: number, totalSize: number }> {
    let totalFiles = 0;
    let totalSize = 0;

    async function scanDirectory(path: string) {
        const entries = await invoke<FileEntry[]>('list_files', { id: sessionId, path });

        for (const entry of entries) {
            const fullPath = `${path}/${entry.name}`;

            if (entry.isDir) {
                await scanDirectory(fullPath);
            } else {
                totalFiles++;
                totalSize += entry.size;
            }
        }
    }

    await scanDirectory(remotePath);
    return { totalFiles, totalSize };
}

async function downloadDirectoryRecursive(remoteDirPath: string, localDirPath: string, sessionId: string, directoryTransferId: string) {
    try {
        // 检查目录是否被暂停
        const directoryItem = transferStore.items.find(item => item.id === directoryTransferId);
        if (directoryItem && directoryItem.status === 'paused') {
            // 等待恢复
            await waitForDirectoryResume(directoryTransferId);
        }

        if (directoryItem && (directoryItem.status === 'cancelled' || directoryItem.status === 'error')) {
            return; // 停止下载
        }

        // List remote directory contents
        const entries = await invoke<FileEntry[]>('list_files', { id: sessionId, path: remoteDirPath });

        for (const entry of entries) {
            // 再次检查状态
            const currentDirectoryItem = transferStore.items.find(item => item.id === directoryTransferId);
            if (!currentDirectoryItem || currentDirectoryItem.status === 'cancelled' || currentDirectoryItem.status === 'error') {
                return; // 停止下载
            }

            const remotePath = `${remoteDirPath}/${entry.name}`;
            const localPath = `${localDirPath}/${entry.name}`;

            if (entry.isDir) {
                // Create local subdirectory
                try {
                    await mkdir(localPath, { recursive: true });
                } catch (e) {
                    console.log('Subdirectory might already exist:', e);
                }

                // Recursively download subdirectory
                await downloadDirectoryRecursive(remotePath, localPath, sessionId, directoryTransferId);
            } else {
                // Download file
                const fileTransferId = crypto.randomUUID();

                transferStore.addTransfer({
                    id: fileTransferId,
                    type: 'download',
                    name: entry.name,
                    localPath,
                    remotePath,
                    size: entry.size,
                    transferred: 0,
                    progress: 0,
                    status: 'pending',
                    sessionId
                });

                // 等待文件下载完成
                await waitForFileCompletion(fileTransferId);

                // 更新目录完成计数
                transferStore.incrementDirectoryCompleted(directoryTransferId);
            }
        }
    } catch (e) {
        console.error(`Failed to download directory contents ${remoteDirPath}:`, e);
        throw e;
    }
}

async function waitForDirectoryResume(directoryTransferId: string): Promise<void> {
    return new Promise((resolve) => {
        const checkResume = () => {
            const item = transferStore.items.find(i => i.id === directoryTransferId);
            if (!item || item.status === 'cancelled' || item.status === 'error') {
                resolve(); // 停止等待
            } else if (item.status === 'running') {
                resolve(); // 恢复了
            } else {
                // 继续等待
                setTimeout(checkResume, 500);
            }
        };

        checkResume();
    });
}

async function waitForFileCompletion(fileTransferId: string): Promise<void> {
    return new Promise((resolve, reject) => {
        const checkCompletion = () => {
            const item = transferStore.items.find(i => i.id === fileTransferId);
            if (!item) {
                reject(new Error('Transfer item not found'));
                return;
            }

            if (item.status === 'completed') {
                resolve();
            } else if (item.status === 'error' || item.status === 'cancelled') {
                reject(new Error(item.error || 'Transfer failed'));
            } else {
                // 继续等待
                setTimeout(checkCompletion, 500);
            }
        };

        checkCompletion();
    });
}

async function handleDownload(file?: FileEntry) {
    try {
        // Determine if this is batch download or single download
        const isTreeMode = contextMenu.value.isTree;
        const isMultiSelect = !isTreeMode && selectedFiles.value.size > 1;

        if (isMultiSelect) {
            // Batch download for multiple selected files
            const selectedDirectory = await open({
                directory: true,
                title: 'Select download directory for batch download'
            });

            if (selectedDirectory && typeof selectedDirectory === 'string') {
                const targets = Array.from(selectedFiles.value);
                for (const fileName of targets) {
                    const entry = files.value.find(f => f.name === fileName);
                    if (!entry) continue;

                    const remotePath = joinPath(currentPath.value, entry.name);
                    const localPath = selectedDirectory.endsWith('/') || selectedDirectory.endsWith('\\')
                        ? `${selectedDirectory}${entry.name}`
                        : `${selectedDirectory}/${entry.name}`;

                    if (entry.isDir) {
                        // Download directory recursively
                        await downloadDirectory(remotePath, localPath, props.sessionId);
                    } else {
                        // Download single file
                        transferStore.addTransfer({
                            id: crypto.randomUUID(),
                            type: 'download',
                            name: entry.name,
                            localPath,
                            remotePath,
                            size: entry.size,
                            transferred: 0,
                            progress: 0,
                            status: 'pending',
                            sessionId: props.sessionId
                        });
                    }
                }
            }
        } else {
            // Single file or directory download
            const targetFile = file || contextMenu.value.file;
            if (!targetFile) return;

            if (targetFile.isDir) {
                // Directory download - ask for local directory
                const selectedDirectory = await open({
                    directory: true,
                    title: 'Select directory to save folder'
                });

                if (selectedDirectory && typeof selectedDirectory === 'string') {
                    const remotePath = contextMenu.value.isTree && contextMenu.value.treePath
                        ? contextMenu.value.treePath
                        : (currentPath.value === '.' ? targetFile.name : `${currentPath.value}/${targetFile.name}`);

                    const localPath = selectedDirectory.endsWith('/') || selectedDirectory.endsWith('\\')
                        ? `${selectedDirectory}${targetFile.name}`
                        : `${selectedDirectory}/${targetFile.name}`;

                    await downloadDirectory(remotePath, localPath, props.sessionId);
                }
            } else {
                // Single file download
                const savePath = await save({
                    defaultPath: targetFile.name,
                    title: 'Save file as'
                });

                if (savePath) {
                    const remotePath = contextMenu.value.isTree && contextMenu.value.treePath
                        ? contextMenu.value.treePath
                        : (currentPath.value === '.' ? targetFile.name : `${currentPath.value}/${targetFile.name}`);

                    transferStore.addTransfer({
                        id: crypto.randomUUID(),
                        type: 'download',
                        name: targetFile.name,
                        localPath: savePath,
                        remotePath,
                        size: targetFile.size,
                        transferred: 0,
                        progress: 0,
                        status: 'pending',
                        sessionId: props.sessionId
                    });
                }
            }
        }
    } catch (e) {
        console.error(e);
        notificationStore.error("Download failed: " + e);
    }
    closeContextMenu();
}

async function handleChangePermissions(file: FileEntry) {
    if (!file) return;
    const newPerms = prompt("Enter new permissions (e.g., 755, u+x):", file.permissions ? file.permissions.toString() : '');
    if (newPerms) {
        try {
            const remotePath = contextMenu.value.isTree && contextMenu.value.treePath
                ? contextMenu.value.treePath
                : (currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`);
            await invoke('change_file_permission', {
                id: props.sessionId,
                path: remotePath,
                perms: newPerms
            });
            await loadFiles(currentPath.value);
        } catch (e) {
            notificationStore.error("Failed to change permissions: " + e);
        }
    }
    closeContextMenu();
}

async function performDelete(skipConfirm: boolean) {
    if (!contextMenu.value.isTree && selectedFiles.value.size === 0) return;

    if (!skipConfirm) {
        const count = contextMenu.value.isTree ? 1 : selectedFiles.value.size;
        const yes = await ask(t('fileManager.deleteConfirm.message', { count }), {
            title: t('fileManager.deleteConfirm.title'),
            kind: 'warning'
        });
        if (!yes) return;
    }

    try {
        // Convert Set to Array for iteration
        if (contextMenu.value.isTree && contextMenu.value.treePath && contextMenu.value.file) {
            const path = contextMenu.value.treePath;
            const isDir = contextMenu.value.file.isDir;
            await invoke('delete_item', { id: props.sessionId, path, isDir });
        } else {
            const targets = Array.from(selectedFiles.value);
            for (const name of targets) {
                // Find file entry to get isDir
                const entry = files.value.find(f => f.name === name);
                if (!entry) continue;

                const remotePath = currentPath.value === '.' ? name : `${currentPath.value}/${name}`;
                await invoke('delete_item', { id: props.sessionId, path: remotePath, isDir: entry.isDir });
            }
        }
        await loadFiles(currentPath.value);
    } catch (e) {
        notificationStore.error("Delete failed: " + e);
    }
    closeContextMenu();
}

async function handleDelete(file?: FileEntry) {
    if (!contextMenu.value.isTree) {
        if (selectedFiles.value.size === 0 && file) {
            selectedFiles.value.add(file.name);
        }
    }
    await performDelete(false);
}

function handleKeyDown(e: KeyboardEvent) {
    if (isEditingPath.value) return;
    if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return;

    if (e.key === 'Delete') {
        performDelete(e.shiftKey);
    }
}

async function handleRename(file: FileEntry) {
    const newName = prompt("New name:", file.name);
    if (!newName || newName === file.name) return;
    try {
        let oldPath: string;
        let newPath: string;

        if (contextMenu.value.isTree && contextMenu.value.treePath) {
            oldPath = contextMenu.value.treePath;
            const parts = oldPath.split('/');
            parts.pop();
            const parent = parts.join('/');
            newPath = parent ? `${parent}/${newName}` : newName;
        } else {
            oldPath = currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`;
            newPath = currentPath.value === '.' ? newName : `${currentPath.value}/${newName}`;
        }
        await invoke('rename_item', { id: props.sessionId, oldPath, newPath });
        await loadFiles(currentPath.value);
    } catch (e) {
        notificationStore.error("Rename failed: " + e);
    }
    closeContextMenu();
}

async function createFolder() {
    const name = prompt("Folder name:");
    if (!name) return;
    try {
        const remotePath = currentPath.value === '.' ? name : `${currentPath.value}/${name}`;
        await invoke('create_directory', { id: props.sessionId, path: remotePath });
        await loadFiles(currentPath.value);
    } catch (e) {
        notificationStore.error("Create folder failed: " + e);
    }
}

async function createFile() {
    const name = prompt("File name:");
    if (!name) return;
    try {
        const remotePath = currentPath.value === '.' ? name : `${currentPath.value}/${name}`;
        await invoke('create_file', { id: props.sessionId, path: remotePath });
        await loadFiles(currentPath.value);
    } catch (e) {
        notificationStore.error("Create file failed: " + e);
    }
}

function copyPath(file: FileEntry) {
    const remotePath = contextMenu.value.isTree && contextMenu.value.treePath
        ? contextMenu.value.treePath
        : (currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`);
    navigator.clipboard.writeText(remotePath);
    closeContextMenu();
}

function copyName(file: FileEntry) {
    navigator.clipboard.writeText(file.name);
    closeContextMenu();
}

function formatDate(timestamp: number) {
    return new Date(timestamp * 1000).toLocaleString();
}

function formatSize(size: number): string {
    if (!Number.isFinite(size) || size < 0) return '-';
    if (size === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let value = size;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
        value /= 1024;
        unitIndex++;
    }
    return `${value.toFixed(value >= 10 || unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
}
</script>

<template>
    <div ref="containerRef" class="h-full bg-gray-900 text-white p-2 flex flex-col" @click="closeContextMenu">
        <!-- Toolbar -->
        <div class="flex flex-col space-y-2 mb-2 bg-gray-800 p-2 rounded">
            <!-- Path Bar -->
            <div class="flex items-center space-x-2">
                <button @click="goUp" class="p-1 hover:bg-gray-700 rounded text-gray-300"
                    :title="t('fileManager.toolbar.upLevel')">
                    <ArrowUp class="w-4 h-4" />
                </button>
                <div class="flex-1 relative">
                    <input v-model="pathInput" @keydown.enter="handlePathSubmit"
                        class="w-full bg-gray-900 border border-gray-700 rounded px-2 py-1 text-sm font-mono text-gray-300 focus:outline-none focus:border-blue-500"
                        :placeholder="t('fileManager.toolbar.pathPlaceholder')" />
                </div>
                <button @click="refresh" class="p-1 hover:bg-gray-700 rounded text-gray-300"
                    :title="t('fileManager.toolbar.refresh')">
                    <RefreshCw class="w-4 h-4" />
                </button>
            </div>

            <!-- Action Buttons -->
            <div class="flex items-center space-x-2 border-t border-gray-700 pt-2">
                <button @click="createFile"
                    class="flex items-center space-x-1 px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded"
                    :title="t('fileManager.toolbar.newFile')">
                    <FilePlus class="w-3 h-3" />
                    <span>{{ t('fileManager.toolbar.newFile') }}</span>
                </button>
                <button @click="createFolder"
                    class="flex items-center space-x-1 px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded"
                    :title="t('fileManager.toolbar.newFolder')">
                    <FolderPlus class="w-3 h-3" />
                    <span>{{ t('fileManager.toolbar.newFolder') }}</span>
                </button>
                <div class="w-px h-4 bg-gray-700 mx-1"></div>
                <button @click="handleUpload"
                    class="flex items-center space-x-1 px-2 py-1 text-xs bg-blue-600 hover:bg-blue-500 rounded"
                    :title="t('fileManager.toolbar.uploadFile')">
                    <Upload class="w-3 h-3" />
                    <span>{{ t('fileManager.toolbar.uploadFile') }}</span>
                </button>
                <!-- Upload Directory placeholder -->
                <button @click="handleUploadDirectory"
                    class="flex items-center space-x-1 px-2 py-1 text-xs bg-blue-600 hover:bg-blue-500 rounded"
                    :title="t('fileManager.toolbar.uploadDirectory')">
                    <FolderPlus class="w-3 h-3" />
                    <span>{{ t('fileManager.toolbar.uploadDirectory') }}</span>
                </button>
            </div>
        </div>

        <!-- File List -->
        <div class="flex-1 overflow-y-auto border border-gray-800 rounded bg-gray-900/50"
            @dragover="handleNativeDragOver" @drop="handleNativeDrop" @contextmenu="handleContainerContextMenu">
            <!-- Header -->
            <div
                class="flex items-center p-2 text-xs text-gray-500 border-b border-gray-800 bg-gray-800/50 font-bold select-none">
                <div class="flex items-center px-2" :style="{ width: columnWidths.name + 'px' }">
                    <span>{{ t('fileManager.headers.name') }}</span>
                    <span class="w-1 h-6 ml-auto cursor-col-resize bg-gray-600 hover:bg-blue-500 transition-colors"
                        @mousedown.stop="startResize('name', $event)"></span>
                </div>
                <div class="flex items-center px-2" :style="{ width: columnWidths.size + 'px' }">
                    <span>{{ t('fileManager.headers.size') }}</span>
                    <span class="w-1 h-6 ml-auto cursor-col-resize bg-gray-600 hover:bg-blue-500 transition-colors"
                        @mousedown.stop="startResize('size', $event)"></span>
                </div>
                <div class="flex items-center px-2" :style="{ width: columnWidths.date + 'px' }">
                    <span>{{ t('fileManager.headers.dateModified') }}</span>
                    <span class="w-1 h-6 ml-auto cursor-col-resize bg-gray-600 hover:bg-blue-500 transition-colors"
                        @mousedown.stop="startResize('date', $event)"></span>
                </div>
                <div class="flex items-center px-2" :style="{ width: columnWidths.owner + 'px' }">
                    <span>{{ t('fileManager.headers.owner') }}</span>
                </div>
            </div>

            <!-- Flat View -->
            <template v-if="viewMode === 'flat'">
                <VirtualFileList
                    :items="files"
                    :view-mode="viewMode"
                    :selected-files="selectedFiles"
                    :selected-tree-paths="selectedTreePaths"
                    :column-widths="columnWidths"
                    :on-selection="handleSelection"
                    :on-navigate="navigate"
                    :on-context-menu="showContextMenu"
                    :on-drag-start="onDragStart"
                    :expanded-paths="expandedPaths"
                    :format-size="formatSize"
                    :format-date="formatDate"
                />
            </template>

            <!-- Tree View -->
            <template v-else>
                <VirtualFileList
                    :items="visibleTreeNodes"
                    :view-mode="viewMode"
                    :selected-files="selectedFiles"
                    :selected-tree-paths="selectedTreePaths"
                    :column-widths="columnWidths"
                    :on-selection="handleSelection"
                    :on-navigate="navigate"
                    :on-context-menu="showContextMenu"
                    :on-tree-selection="handleTreeSelection"
                    :on-open-tree-file="openTreeFile"
                    :on-tree-context-menu="showTreeContextMenu"
                    :on-toggle-directory="toggleDirectory"
                    :on-drag-start="onDragStart"
                    :expanded-paths="expandedPaths"
                    :format-size="formatSize"
                    :format-date="formatDate"
                />
            </template>

            <div v-if="files.length === 0" class="p-4 text-center text-gray-600 text-sm">
                {{ t('fileManager.emptyDirectory') }}
            </div>
        </div>

        <!-- Transfer List -->
        <TransferList />

        <!-- Opening File Indicator -->
        <div v-if="isOpeningFile"
            class="fixed bottom-4 right-4 bg-gray-800/90 text-gray-100 text-xs px-3 py-2 rounded shadow-lg border border-gray-700 z-50">
            正在打开...
        </div>

        <!-- Context Menu -->
        <div v-if="contextMenu.show" ref="contextMenuRef" :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
            class="fixed bg-gray-800 border border-gray-700 shadow-xl rounded z-50 py-1 min-w-[150px]">
            
            <template v-if="contextMenu.isBackground">
                <button @click.stop="refresh(); closeContextMenu()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
                    <RefreshCw class="w-4 h-4 mr-2 text-gray-400" />
                    {{ t('fileManager.toolbar.refresh') }}
                </button>
                <div class="border-t border-gray-700 my-1"></div>
                <button @click.stop="createFile(); closeContextMenu()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
                    <FilePlus class="w-4 h-4 mr-2 text-gray-400" />
                    {{ t('fileManager.toolbar.newFile') }}
                </button>
                <button @click.stop="createFolder(); closeContextMenu()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
                    <FolderPlus class="w-4 h-4 mr-2 text-gray-400" />
                    {{ t('fileManager.toolbar.newFolder') }}
                </button>
                <div class="border-t border-gray-700 my-1"></div>
                <button @click.stop="handleUpload(); closeContextMenu()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
                    <Upload class="w-4 h-4 mr-2 text-gray-400" />
                    {{ t('fileManager.toolbar.uploadFile') }}
                </button>
                <button @click.stop="handleUploadDirectory(); closeContextMenu()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
                    <FolderPlus class="w-4 h-4 mr-2 text-gray-400" />
                    {{ t('fileManager.toolbar.uploadDirectory') }}
                </button>
                <div class="border-t border-gray-700 my-1"></div>
                <button @click.stop="handleSetWorkspace()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center text-purple-400">
                    <Briefcase class="w-4 h-4 mr-2" />
                    Set as AI Workspace
                </button>
            </template>

            <template v-else>
                <button @click.stop="handleDownload()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
                    <span class="flex-1">{{
                        (!contextMenu.isTree && selectedFiles.size > 1)
                            ? t('fileManager.contextMenu.batchDownload')
                            : t('fileManager.contextMenu.download')
                    }}</span>
                    <span v-if="!contextMenu.isTree && selectedFiles.size > 1" class="text-xs text-gray-400">({{
                        selectedFiles.size
                        }})</span>
                </button>
                <button @click.stop="handleRename(contextMenu.file!)"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">{{ t('fileManager.contextMenu.rename')
                    }}</button>
                <button @click.stop="handleDelete(contextMenu.file!)"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 text-red-400">
                    {{ t('fileManager.contextMenu.delete') }} {{ selectedFiles.size > 1 ? `(${selectedFiles.size})` : '' }}
                </button>
                <div class="border-t border-gray-700 my-1"></div>
                <button v-if="contextMenu.file?.isDir" @click.stop="handleSetWorkspace()"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center text-purple-400">
                    <Briefcase class="w-4 h-4 mr-2" />
                    Set as AI Workspace
                </button>
                <button @click.stop="copyPath(contextMenu.file!)"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">{{
                        t('fileManager.contextMenu.copyPath') }}</button>
                <button @click.stop="copyName(contextMenu.file!)"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">{{
                        t('fileManager.contextMenu.copyName') }}</button>
                <button @click.stop="handleChangePermissions(contextMenu.file!)"
                    class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">{{
                        t('fileManager.contextMenu.changePermissions')
                    }}</button>
            </template>
        </div>
    </div>
</template>
