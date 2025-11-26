<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { File, Folder, ArrowUp, RefreshCw, Upload, FilePlus, FolderPlus } from 'lucide-vue-next';
import { open, save, ask, message } from '@tauri-apps/plugin-dialog';
import { readDir } from '@tauri-apps/plugin-fs';
import type { FileEntry, FileManagerViewMode } from '../types';
import { useTransferStore } from '../stores/transfers';
import { useSettingsStore } from '../stores/settings';
import TransferList from './TransferList.vue';
import { useI18n } from '../composables/useI18n';
import draggable from 'vuedraggable';

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

    let x = e.clientX;
    let y = e.clientY;

    const menuWidth = 160;
    const menuHeight = 180;

    if (x + menuWidth > window.innerWidth) {
        x = window.innerWidth - menuWidth - 10;
    }

    if (y + menuHeight > window.innerHeight) {
        y = window.innerHeight - menuHeight - 10;
    }

    contextMenu.value = { show: true, x, y, file: node.entry, treePath: node.path, isTree: true };
}

const props = defineProps<{ sessionId: string }>();
const { t } = useI18n();
const settingsStore = useSettingsStore();
const viewMode = computed<FileManagerViewMode>(() => settingsStore.fileManager.viewMode);
const currentPath = ref('.');
const files = ref<FileEntry[]>([]);
const contextMenu = ref<{ show: boolean, x: number, y: number, file: FileEntry | null, treePath: string | null, isTree: boolean }>({ show: false, x: 0, y: 0, file: null, treePath: null, isTree: false });
const isEditingPath = ref(false);
const pathInput = ref('');
const selectedFiles = ref<Set<string>>(new Set());
const lastSelectedIndex = ref<number>(-1);
let unlistenDrop: (() => void) | null = null;
const transferStore = useTransferStore();
const treeRootPath = ref<string>('.');
const treeNodes = ref<Map<string, TreeNode>>(new Map());
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

const visibleTreeNodes = computed<TreeNode[]>(() => {
    const result: TreeNode[] = [];
    const nodes = treeNodes.value;

    const collectChildren = (parentPath: string | null, depth: number) => {
        const children: TreeNode[] = [];
        nodes.forEach((node) => {
            if (node.parentPath === parentPath) {
                children.push({ ...node, depth });
            }
        });

        children.sort((a, b) => {
            if (a.entry.isDir === b.entry.isDir) {
                return a.entry.name.localeCompare(b.entry.name);
            }
            return a.entry.isDir ? -1 : 1;
        });

        for (const child of children) {
            result.push(child);
            if (child.entry.isDir && expandedPaths.value.has(child.path)) {
                collectChildren(child.path, depth + 1);
            }
        }
    };

    collectChildren(treeRootPath.value === '.' ? null : treeRootPath.value, 0);
    collectChildren(treeRootPath.value === '.' ? null : treeRootPath.value, 0);
    return result;
});

const draggableTreeNodes = computed({
    get: () => visibleTreeNodes.value,
    set: () => { /* Read-only for drag source */ }
});

function cloneFile(element: FileEntry | TreeNode) {
    let entry: FileEntry;
    let path: string;

    if ('entry' in element) { // TreeNode
        entry = (element as TreeNode).entry;
        path = (element as TreeNode).path;
    } else { // FileEntry
        entry = element as FileEntry;
        path = currentPath.value === '.' ? entry.name : `${currentPath.value}/${entry.name}`;
    }

    return {
        path: path,
        isDir: entry.isDir
    };
}

async function loadFiles(path: string) {
    try {
        files.value = await invoke<FileEntry[]>('list_files', { id: props.sessionId, path });
        currentPath.value = path;
        pathInput.value = path;
        selectedFiles.value.clear();
        lastSelectedIndex.value = -1;

        if (viewMode.value === 'tree') {
            treeRootPath.value = path;
            treeNodes.value = new Map();
            expandedPaths.value = new Set();
            selectedTreePaths.value = new Set();
            const parentPath = path === '.' ? null : path;
            for (const entry of files.value) {
                const fullPath = path === '.' ? entry.name : `${path}/${entry.name}`;
                treeNodes.value.set(fullPath, {
                    entry,
                    path: fullPath,
                    depth: 0,
                    parentPath,
                    childrenLoaded: false,
                    loading: false,
                });
            }
        }
    } catch (e) {
        console.error(e);
        files.value = [];
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

    try {
        const children = await invoke<FileEntry[]>('list_files', { id: props.sessionId, path: node.path });
        for (const child of children) {
            const childPath = `${node.path}/${child.name}`;
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
        }
        existing.childrenLoaded = true;
        treeNodes.value.set(node.path, existing);
    } catch (e) {
        console.error(e);
    } finally {
        const updated = treeNodes.value.get(node.path);
        if (updated) {
            updated.loading = false;
            treeNodes.value.set(node.path, updated);
        }
    }
}

async function openTreeFile(node: TreeNode) {
    if (node.entry.isDir) {
        await toggleDirectory(node);
        return;
    }
    try {
        await invoke('edit_remote_file', {
            id: props.sessionId,
            remotePath: node.path,
            remoteName: node.entry.name,
        });
    } catch (e) {
        alert("Failed to open file: " + e);
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

function handleMouseMove(event: MouseEvent) {
    if (!resizingColumn.value) return;
    const delta = event.clientX - resizeStartX.value;
    const newWidth = Math.max(80, resizeStartWidth.value + delta);
    columnWidths.value[resizingColumn.value] = newWidth;
}

function stopResize() {
    resizingColumn.value = null;
}

onMounted(async () => {
    loadFiles('.');
    transferStore.initListeners();
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', stopResize);
    window.addEventListener('keydown', handleKeyDown);
    unlistenDrop = await listen('tauri://drag-drop', async (event) => {
        const payload = event.payload as { paths: string[] };
        const paths = payload.paths || (Array.isArray(payload) ? payload : []);

        if (!paths || paths.length === 0) return;

        for (const localPath of paths) {
            // Handle both slash and backslash, remove empty parts
            const parts = localPath.split(/[\\/]/).filter(p => p);
            const name = parts.pop() || 'uploaded';
            const remotePath = currentPath.value === '.' ? name : `${currentPath.value}/${name}`;

            const transferId = typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);

            transferStore.addTransfer({
                id: transferId,
                type: 'upload',
                name,
                localPath,
                remotePath,
                size: 0, // Backend calculates
                transferred: 0,
                progress: 0,
                status: 'pending',
                sessionId: props.sessionId
            });
        }
        loadFiles(currentPath.value);
    });
});

watch(viewMode, (mode) => {
    if (mode === 'tree') {
        loadFiles(currentPath.value);
    }
});

onUnmounted(() => {
    if (unlistenDrop) {
        unlistenDrop();
    }
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', stopResize);
    window.removeEventListener('keydown', handleKeyDown);
});

async function navigate(entry: FileEntry) {
    if (entry.isDir) {
        const newPath = currentPath.value === '.' ? entry.name : `${currentPath.value}/${entry.name}`;
        loadFiles(newPath);
    } else {
        // Edit remote file
        try {
            await invoke('edit_remote_file', {
                id: props.sessionId,
                remotePath: `${currentPath.value}/${entry.name}`,
                remoteName: entry.name
            });
        } catch (e) {
            alert("Failed to open file: " + e);
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

    // Calculate position to avoid overflow
    let x = e.clientX;
    let y = e.clientY;

    const menuWidth = 160; // Approximate width
    const menuHeight = 180; // Approximate height

    if (x + menuWidth > window.innerWidth) {
        x = window.innerWidth - menuWidth - 10; // 10px padding
    }

    if (y + menuHeight > window.innerHeight) {
        y = window.innerHeight - menuHeight - 10;
    }

    contextMenu.value = { show: true, x, y, file, treePath: null, isTree: false };
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
            const remotePath = currentPath.value === '.' ? name : `${currentPath.value}/${name}`;

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
        alert("Upload failed: " + e);
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
            const remotePath = currentPath.value === '.' ? name : `${currentPath.value}/${name}`;

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
        alert("Upload directory failed: " + e);
    }
}

async function handleDownload(file: FileEntry) {
    if (!file) return;
    try {
        const savePath = await save({
            defaultPath: file.name,
            title: 'Save file as'
        });
        if (savePath) {
            const remotePath = contextMenu.value.isTree && contextMenu.value.treePath
                ? contextMenu.value.treePath
                : (currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`);

            transferStore.addTransfer({
                id: crypto.randomUUID(),
                type: 'download',
                name: file.name,
                localPath: savePath,
                remotePath,
                size: file.size,
                transferred: 0,
                progress: 0,
                status: 'pending',
                sessionId: props.sessionId
            });
        }
    } catch (e) {
        console.error(e);
        alert("Download failed: " + e);
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
            alert("Failed to change permissions: " + e);
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
        await message("Delete failed: " + e, { title: 'Error', kind: 'error' });
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
        alert("Rename failed: " + e);
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
        alert("Create folder failed: " + e);
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
        alert("Create file failed: " + e);
    }
}

function copyPath(file: FileEntry) {
    const remotePath = contextMenu.value.isTree && contextMenu.value.treePath
        ? contextMenu.value.treePath
        : (currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`);
    navigator.clipboard.writeText(remotePath);
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
    <div class="h-full bg-gray-900 text-white p-2 flex flex-col" @click="closeContextMenu">
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
        <div class="flex-1 overflow-y-auto border border-gray-800 rounded bg-gray-900/50">
            <!-- Header -->
            <div
                class="flex items-center p-2 text-xs text-gray-500 border-b border-gray-800 bg-gray-800/50 font-bold select-none">
                <div class="flex items-center" :style="{ width: columnWidths.name + 'px' }">
                    <span class="mr-1">{{ t('fileManager.headers.name') }}</span>
                    <span class="w-1 h-6 ml-auto cursor-col-resize"
                        @mousedown.stop="startResize('name', $event)"></span>
                </div>
                <div class="text-right" :style="{ width: columnWidths.size + 'px' }">
                    <span>{{ t('fileManager.headers.size') }}</span>
                    <span class="inline-block w-1 h-6 ml-1 cursor-col-resize align-middle"
                        @mousedown.stop="startResize('size', $event)"></span>
                </div>
                <div :style="{ width: columnWidths.date + 'px' }">
                    <span>{{ t('fileManager.headers.dateModified') }}</span>
                    <span class="inline-block w-1 h-6 ml-1 cursor-col-resize align-middle"
                        @mousedown.stop="startResize('date', $event)"></span>
                </div>
                <div :style="{ width: columnWidths.owner + 'px' }">
                    <span>{{ t('fileManager.headers.owner') }}</span>
                    <span class="inline-block w-1 h-6 ml-1 cursor-col-resize align-middle"
                        @mousedown.stop="startResize('owner', $event)"></span>
                </div>
            </div>

            <!-- Flat View -->
            <template v-if="viewMode === 'flat'">
                <draggable v-model="files" item-key="name" :group="{ name: 'files', pull: 'clone', put: false }"
                    :sort="false" :clone="cloneFile">
                    <template #item="{ element: file, index }">
                        <div class="flex items-center p-2 cursor-pointer border-b border-gray-800/50 transition-colors select-none"
                            :class="{ 'bg-blue-900/50': selectedFiles.has(file.name), 'hover:bg-gray-800': !selectedFiles.has(file.name) }"
                            @click="handleSelection($event, file, index)" @dblclick="navigate(file)"
                            @contextmenu="showContextMenu($event, file)">
                            <div class="flex items-center min-w-0" :style="{ width: columnWidths.name + 'px' }">
                                <Folder v-if="file.isDir" class="w-4 h-4 mr-2 text-yellow-400 flex-shrink-0" />
                                <File v-else class="w-4 h-4 mr-2 text-blue-400 flex-shrink-0" />
                                <span class="text-sm truncate">{{ file.name }}</span>
                            </div>
                            <span class="text-xs text-gray-500 font-mono text-right"
                                :style="{ width: columnWidths.size + 'px' }">{{
                                    file.isDir ? '' : formatSize(file.size) }}</span>
                            <span class="text-xs text-gray-500 truncate" :style="{ width: columnWidths.date + 'px' }">{{
                                formatDate(file.mtime) }}</span>
                            <span class="text-xs text-gray-500 truncate"
                                :style="{ width: columnWidths.owner + 'px' }">{{
                                    file.owner
                                }}</span>
                        </div>
                    </template>
                </draggable>
            </template>

            <!-- Tree View: multi-level with lazy loading -->
            <template v-else>
                <draggable v-model="draggableTreeNodes" item-key="path"
                    :group="{ name: 'files', pull: 'clone', put: false }" :sort="false" :clone="cloneFile">
                    <template #item="{ element: node }">
                        <div class="flex items-center p-2 cursor-pointer border-b border-gray-800/50 transition-colors select-none"
                            :class="{ 'bg-blue-900/50': selectedTreePaths.has(node.path), 'hover:bg-gray-800': !selectedTreePaths.has(node.path) }"
                            @click.stop="handleTreeSelection(node)" @dblclick.stop="openTreeFile(node)"
                            @contextmenu.stop.prevent="showTreeContextMenu($event, node)">
                            <div class="flex items-center min-w-0"
                                :style="{ width: columnWidths.name + 'px', paddingLeft: (node.depth * 16) + 'px' }">
                                <button v-if="node.entry.isDir"
                                    class="mr-1 w-3 h-3 flex items-center justify-center text-xs text-gray-400"
                                    @click.stop="toggleDirectory(node)">
                                    <span v-if="expandedPaths.has(node.path)">-</span>
                                    <span v-else>+</span>
                                </button>
                                <span v-else class="mr-4"></span>
                                <Folder v-if="node.entry.isDir" class="w-4 h-4 mr-2 text-yellow-400 flex-shrink-0" />
                                <File v-else class="w-4 h-4 mr-2 text-blue-400 flex-shrink-0" />
                                <span class="text-sm truncate">{{ node.entry.name }}</span>
                            </div>
                            <span class="text-xs text-gray-500 font-mono text-right"
                                :style="{ width: columnWidths.size + 'px' }">{{
                                    node.entry.isDir ? '' : formatSize(node.entry.size) }}</span>
                            <span class="text-xs text-gray-500 truncate" :style="{ width: columnWidths.date + 'px' }">{{
                                formatDate(node.entry.mtime) }}</span>
                            <span class="text-xs text-gray-500 truncate"
                                :style="{ width: columnWidths.owner + 'px' }">{{
                                    node.entry.owner
                                }}</span>
                        </div>
                    </template>
                </draggable>
            </template>

            <div v-if="files.length === 0" class="p-4 text-center text-gray-600 text-sm">
                {{ t('fileManager.emptyDirectory') }}
            </div>
        </div>

        <!-- Transfer List -->
        <TransferList />

        <!-- Context Menu -->
        <div v-if="contextMenu.show" :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
            class="fixed bg-gray-800 border border-gray-700 shadow-xl rounded z-50 py-1 min-w-[150px]">
            <button @click.stop="handleDownload(contextMenu.file!)"
                class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
                <span class="flex-1">{{ t('fileManager.contextMenu.download') }}</span>
            </button>
            <button @click.stop="handleRename(contextMenu.file!)"
                class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">{{ t('fileManager.contextMenu.rename')
                }}</button>
            <button @click.stop="handleDelete(contextMenu.file!)"
                class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 text-red-400">
                {{ t('fileManager.contextMenu.delete') }} {{ selectedFiles.size > 1 ? `(${selectedFiles.size})` : '' }}
            </button>
            <div class="border-t border-gray-700 my-1"></div>
            <button @click.stop="copyPath(contextMenu.file!)"
                class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">{{
                    t('fileManager.contextMenu.copyPath') }}</button>
            <button @click.stop="handleChangePermissions(contextMenu.file!)"
                class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">{{
                    t('fileManager.contextMenu.changePermissions')
                }}</button>
        </div>
    </div>
</template>
