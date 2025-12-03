<script setup lang="ts">
import { ref, computed, nextTick } from 'vue';
import TerminalView from './TerminalView.vue';
import FileEditorModal from './FileEditorModal.vue';
import { Terminal, FileText, X } from 'lucide-vue-next';

defineProps<{
  sessionId: string;
}>();

const activeTab = ref<'terminal' | 'editor'>('terminal');
const editorFiles = ref<Array<{ id: string; name: string; path: string; fileName: string }>>([]);
const activeEditorId = ref<string | null>(null);
const terminalViewRef = ref<any>(null);
const fileEditorModalRef = ref<any>(null);

const activeEditorFile = computed(() => {
  if (!activeEditorId.value) return null;
  return editorFiles.value.find(f => f.id === activeEditorId.value) || null;
});

function openFileEditor(filePath: string, fileName: string) {
  const existingFile = editorFiles.value.find(f => f.path === filePath);
  if (existingFile) {
    activeEditorId.value = existingFile.id;
    activeTab.value = 'editor';
    return;
  }

  const newFile = {
    id: Date.now().toString(),
    name: fileName,
    path: filePath,
    fileName
  };

  editorFiles.value.push(newFile);
  activeEditorId.value = newFile.id;
  activeTab.value = 'editor';
}

async function closeEditor(fileId: string) {
  const file = editorFiles.value.find(f => f.id === fileId);
  if (!file) return;

  // Check for unsaved changes if the modal is available
  if (fileEditorModalRef.value) {
    const hasChanges = fileEditorModalRef.value.hasUnsavedChanges(file.path);
    if (hasChanges) {
      // Switch to this tab so the user can see the confirmation dialog
      activeTab.value = 'editor';
      activeEditorId.value = fileId;

      // Wait for Vue to update props
      await nextTick();

      // Trigger the close flow in the modal (which shows confirmation)
      fileEditorModalRef.value.triggerClose();
      return;
    }
  }

  // No unsaved changes, proceed to close
  const index = editorFiles.value.findIndex(f => f.id === fileId);
  if (index === -1) return;

  editorFiles.value.splice(index, 1);

  if (activeEditorId.value === fileId) {
    if (editorFiles.value.length > 0) {
      activeEditorId.value = editorFiles.value[editorFiles.value.length - 1].id;
    } else {
      activeTab.value = 'terminal';
      activeEditorId.value = null;
    }
  }
}

async function closeAllEditors() {
  // Check for unsaved changes in all files
  if (fileEditorModalRef.value) {
    for (const file of editorFiles.value) {
      if (fileEditorModalRef.value.hasUnsavedChanges(file.path)) {
        // Found a file with unsaved changes, switch to it and trigger close
        activeTab.value = 'editor';
        activeEditorId.value = file.id;
        await nextTick();
        fileEditorModalRef.value.triggerClose();
        return; // Stop here, user must handle this one first
      }
    }
  }

  // No unsaved changes, close all
  editorFiles.value = [];
  activeEditorId.value = null;
  activeTab.value = 'terminal';
}

function handleEditorClose() {
  // This is called when the modal emits 'close' (e.g. after saving or discarding)
  if (activeEditorId.value) {
    // We can directly remove it because the modal has already handled the confirmation
    const index = editorFiles.value.findIndex(f => f.id === activeEditorId.value);
    if (index !== -1) {
      editorFiles.value.splice(index, 1);

      if (editorFiles.value.length > 0) {
        activeEditorId.value = editorFiles.value[editorFiles.value.length - 1].id;
      } else {
        activeTab.value = 'terminal';
        activeEditorId.value = null;
      }
    }
  }
}

function handleEditorSave() {
  // Editor save logic is handled in FileEditorModal
  // This can be used for additional UI updates if needed
}

// Expose methods and refs for parent components
defineExpose({
  openFileEditor,
  closeAllEditors,
  terminalView: terminalViewRef,
  $refs: { terminalView: terminalViewRef }
});
</script>

<template>
  <div class="h-full w-full flex flex-col bg-gray-900">
    <!-- Tab Headers -->
    <div class="h-8 bg-gray-800 border-b border-gray-700 flex items-center flex-shrink-0">
      <!-- Terminal Tab -->
      <button
        @click="activeTab = 'terminal'"
        :class="[
          'flex items-center px-3 py-1 text-xs border-r border-gray-700 transition-colors',
          activeTab === 'terminal' 
            ? 'bg-gray-900 text-white' 
            : 'text-gray-400 hover:text-white hover:bg-gray-700'
        ]"
      >
        <Terminal class="w-3 h-3 mr-1" />
        Terminal
      </button>

      <!-- Editor Tabs -->
      <button
        v-for="file in editorFiles"
        :key="file.id"
        @click="activeTab = 'editor'; activeEditorId = file.id"
        :class="[
          'flex items-center px-2 py-1 text-xs border-r border-gray-700 transition-colors group',
          activeTab === 'editor' && activeEditorId === file.id
            ? 'bg-gray-900 text-white'
            : 'text-gray-400 hover:text-white hover:bg-gray-700'
        ]"
      >
        <FileText class="w-3 h-3 mr-1" />
        <span class="max-w-24 truncate">{{ file.name }}</span>
        <button
          @click.stop="closeEditor(file.id)"
          class="ml-1 p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-gray-600 transition-all"
          :class="activeTab === 'editor' && activeEditorId === file.id ? 'opacity-100' : ''"
        >
          <X class="w-3 h-3" />
        </button>
      </button>

      <!-- Close All Editors Button (visible when editors are open) -->
      <button
        v-if="editorFiles.length > 0"
        @click="closeAllEditors"
        class="ml-auto px-2 py-1 text-xs text-gray-400 hover:text-white hover:bg-gray-700 transition-colors mr-2"
        title="Close All Editors"
      >
        Close All
      </button>
    </div>

    <!-- Tab Content -->
    <div class="flex-1 overflow-hidden">
      <!-- Terminal View -->
      <div v-show="activeTab === 'terminal'" class="h-full w-full">
        <TerminalView ref="terminalViewRef" :sessionId="sessionId" />
      </div>

      <!-- File Editor View -->
      <div v-show="activeTab === 'editor' && activeEditorFile" class="h-full w-full">
        <FileEditorModal v-if="activeEditorFile" ref="fileEditorModalRef" :show="true" :sessionId="sessionId"
          :filePath="activeEditorFile.path" :fileName="activeEditorFile.fileName" @close="handleEditorClose"
          @save="handleEditorSave" />
      </div>
    </div>
  </div>
</template>
