<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { File, Folder, ArrowUp, RefreshCw, Upload, FilePlus, FolderPlus } from 'lucide-vue-next';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { FileEntry } from '../types';

const props = defineProps<{ sessionId: string }>();
const currentPath = ref('.');
const files = ref<FileEntry[]>([]);
const contextMenu = ref<{ show: boolean, x: number, y: number, file: FileEntry | null }>({ show: false, x: 0, y: 0, file: null });
const isEditingPath = ref(false);
const pathInput = ref('');

async function loadFiles(path: string) {
  try {
    files.value = await invoke<FileEntry[]>('list_files', { id: props.sessionId, path });
    currentPath.value = path;
    pathInput.value = path;
  } catch (e) {
    console.error(e);
    files.value = [];
  }
}

onMounted(() => loadFiles('.'));

async function navigate(entry: FileEntry) {
    if (entry.isDir) {
        const newPath = currentPath.value === '.' ? entry.name : `${currentPath.value}/${entry.name}`;
        loadFiles(newPath);
    } else {
        // Download and open temp
        try {
            const remotePath = currentPath.value === '.' ? entry.name : `${currentPath.value}/${entry.name}`;
            await invoke('download_temp_and_open', { id: props.sessionId, remotePath, remoteName: entry.name });
        } catch (e) {
            console.error("Failed to open file", e);
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

function showContextMenu(e: MouseEvent, file: FileEntry) {
    e.preventDefault();
    contextMenu.value = { show: true, x: e.clientX, y: e.clientY, file };
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
        if (selected && typeof selected === 'string') { // plugin-dialog v2 returns string or null (or array)
             // Extract filename
             // Windows path? or Unix? Local is Windows.
             const name = selected.split(/[\\/]/).pop() || 'uploaded_file';
             const remotePath = currentPath.value === '.' ? name : `${currentPath.value}/${name}`;
             await invoke('upload_file', { id: props.sessionId, localPath: selected, remotePath });
             await loadFiles(currentPath.value);
        }
    } catch(e) {
        console.error(e);
        alert("Upload failed: " + e);
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
             const remotePath = currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`;
             await invoke('download_file', { id: props.sessionId, remotePath, localPath: savePath });
             alert("Download finished!");
        }
    } catch(e) {
         console.error(e);
         alert("Download failed: " + e);
    }
    closeContextMenu();
}

async function handleDelete(file: FileEntry) {
     if (!file || !confirm(`Delete ${file.name}?`)) return;
     try {
         const remotePath = currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`;
         await invoke('delete_item', { id: props.sessionId, path: remotePath, isDir: file.isDir });
         await loadFiles(currentPath.value);
     } catch(e) {
         alert("Delete failed: " + e);
     }
     closeContextMenu();
}

async function handleRename(file: FileEntry) {
    const newName = prompt("New name:", file.name);
    if (!newName || newName === file.name) return;
    try {
         const oldPath = currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`;
         const newPath = currentPath.value === '.' ? newName : `${currentPath.value}/${newName}`;
         await invoke('rename_item', { id: props.sessionId, oldPath, newPath });
         await loadFiles(currentPath.value);
    } catch(e) {
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
    } catch(e) {
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
    } catch(e) {
        alert("Create file failed: " + e);
    }
}

function copyPath(file: FileEntry) {
     const remotePath = currentPath.value === '.' ? file.name : `${currentPath.value}/${file.name}`;
     navigator.clipboard.writeText(remotePath);
     closeContextMenu();
}

</script>

<template>
  <div class="h-full bg-gray-900 text-white p-2 flex flex-col" @click="closeContextMenu">
    <!-- Toolbar -->
    <div class="flex flex-col space-y-2 mb-2 bg-gray-800 p-2 rounded">
        <!-- Path Bar -->
        <div class="flex items-center space-x-2">
            <button @click="goUp" class="p-1 hover:bg-gray-700 rounded text-gray-300" title="Up level">
                <ArrowUp class="w-4 h-4" />
            </button>
            <div class="flex-1 relative">
                <input 
                    v-model="pathInput"
                    @keydown.enter="handlePathSubmit"
                    class="w-full bg-gray-900 border border-gray-700 rounded px-2 py-1 text-sm font-mono text-gray-300 focus:outline-none focus:border-blue-500"
                    placeholder="Path..."
                />
            </div>
             <button @click="refresh" class="p-1 hover:bg-gray-700 rounded text-gray-300" title="Refresh">
                <RefreshCw class="w-4 h-4" />
            </button>
        </div>
        
        <!-- Action Buttons -->
        <div class="flex items-center space-x-2 border-t border-gray-700 pt-2">
            <button @click="createFile" class="flex items-center space-x-1 px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded" title="New File">
                <FilePlus class="w-3 h-3" />
                <span>New File</span>
            </button>
            <button @click="createFolder" class="flex items-center space-x-1 px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded" title="New Folder">
                <FolderPlus class="w-3 h-3" />
                <span>New Folder</span>
            </button>
            <div class="w-px h-4 bg-gray-700 mx-1"></div>
            <button @click="handleUpload" class="flex items-center space-x-1 px-2 py-1 text-xs bg-blue-600 hover:bg-blue-500 rounded" title="Upload File">
                <Upload class="w-3 h-3" />
                <span>Upload File</span>
            </button>
            <!-- Upload Directory placeholder -->
             <button class="flex items-center space-x-1 px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded opacity-50 cursor-not-allowed" title="Upload Directory (Not Implemented)">
                <FolderPlus class="w-3 h-3" />
                <span>Upload Dir</span>
            </button>
        </div>
    </div>
    
    <!-- File List -->
    <div class="flex-1 overflow-y-auto border border-gray-800 rounded bg-gray-900/50">
        <!-- Header -->
        <div class="flex items-center p-2 text-xs text-gray-500 border-b border-gray-800 bg-gray-800/50 font-bold">
             <span class="flex-1">Name</span>
             <span class="w-20 text-right">Size</span>
        </div>
        
        <div v-for="file in files" :key="file.name" 
             class="flex items-center p-2 hover:bg-gray-800 cursor-pointer border-b border-gray-800/50 transition-colors"
             @dblclick="navigate(file)"
             @contextmenu="showContextMenu($event, file)">
            <Folder v-if="file.isDir" class="w-4 h-4 mr-2 text-yellow-400" />
            <File v-else class="w-4 h-4 mr-2 text-blue-400" />
            <span class="text-sm flex-1 truncate select-none">{{ file.name }}</span>
            <span class="text-xs text-gray-500 w-20 text-right font-mono">{{ file.size }}</span>
        </div>
        
        <div v-if="files.length === 0" class="p-4 text-center text-gray-600 text-sm">
            Empty directory
        </div>
    </div>

    <!-- Context Menu -->
    <div v-if="contextMenu.show" 
         :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }"
         class="fixed bg-gray-800 border border-gray-700 shadow-xl rounded z-50 py-1 min-w-[150px]">
        <button @click="handleDownload(contextMenu.file!)" class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 flex items-center">
             <span class="flex-1">Download</span>
        </button>
        <button @click="handleRename(contextMenu.file!)" class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">Rename</button>
        <button @click="handleDelete(contextMenu.file!)" class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700 text-red-400">Delete</button>
        <div class="border-t border-gray-700 my-1"></div>
        <button @click="copyPath(contextMenu.file!)" class="w-full text-left px-4 py-2 text-sm hover:bg-gray-700">Copy Path</button>
    </div>
  </div>
</template>
