<script setup lang="ts">
import { ref, computed, nextTick } from 'vue';
import TerminalView from './TerminalView.vue';
import FileEditorModal from './FileEditorModal.vue';
import { Terminal, FileText, X } from 'lucide-vue-next';
import { useI18n } from '../composables/useI18n';

defineProps<{
  sessionId: string;
}>();

const activeTab = ref<'terminal' | 'editor'>('terminal');
const editorFiles = ref<Array<{ id: string; name: string; path: string; fileName: string }>>([]);
const activeEditorId = ref<string | null>(null);
const terminalViewRef = ref<any>(null);
const fileEditorModalRef = ref<any>(null);
const { t } = useI18n();

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

function activateEditor(fileId: string) {
  activeTab.value = 'editor';
  activeEditorId.value = fileId;
}

function handleEditorTabKeydown(event: KeyboardEvent, fileId: string) {
  if (event.key !== 'Enter' && event.key !== ' ') return;
  event.preventDefault();
  activateEditor(fileId);
}

async function switchToPath(path: string) {
  activeTab.value = 'terminal';
  await nextTick();
  if (terminalViewRef.value) {
    terminalViewRef.value.switchToPath(path);
  }
}

// Expose methods and refs for parent components
defineExpose({
  openFileEditor,
  closeAllEditors,
  switchToPath,
  terminalView: terminalViewRef,
  $refs: { terminalView: terminalViewRef }
});
</script>

<template>
  <div class="flex h-full w-full min-h-0 flex-col bg-bg-primary">
    <!-- Tab Headers -->
    <div class="flex shrink-0 items-start border-b border-subtle bg-bg-secondary">
      <!-- Wrappable Tab Container -->
      <div class="flex min-w-0 flex-1 flex-wrap items-center min-h-8">
        <!-- Terminal Tab -->
        <button @click="activeTab = 'terminal'" :class="[
          'flex items-center px-3 py-1 text-xs border-r border-subtle transition-all duration-normal whitespace-nowrap flex-shrink-0',
          activeTab === 'terminal'
            ? 'bg-bg-tertiary text-text-primary border-l border-l-primary'
            : 'text-text-muted hover:text-text-primary hover:bg-bg-tertiary'
        ]">
          <Terminal class="w-3 h-3 mr-1" />
          {{ t('terminalTabArea.terminalTab') }}
        </button>

        <!-- Editor Tabs -->
        <div
          v-for="file in editorFiles"
          :key="file.id"
          role="button"
          tabindex="0"
          @click="activateEditor(file.id)"
          @keydown="handleEditorTabKeydown($event, file.id)"
          :class="[
            'group flex max-w-[200px] flex-shrink-0 items-center border-r border-subtle px-2 py-1 text-xs transition-all duration-normal whitespace-nowrap focus:outline-none focus:ring-1 focus:ring-accent focus:ring-offset-0 focus:ring-offset-bg-secondary',
            activeTab === 'editor' && activeEditorId === file.id
              ? 'bg-bg-tertiary text-text-primary border-l border-l-primary'
              : 'text-text-muted hover:text-text-primary hover:bg-bg-tertiary'
          ]" :title="file.path">
          <FileText class="w-3 h-3 mr-1 flex-shrink-0" />
          <span class="truncate">{{ file.name }}</span>
          <button @click.stop="closeEditor(file.id)"
            class="ml-1 flex-shrink-0 rounded p-0.5 opacity-0 transition-all duration-normal group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-bg-elevated"
            :class="activeTab === 'editor' && activeEditorId === file.id ? 'opacity-100' : ''">
            <X class="w-3 h-3" />
          </button>
        </div>
      </div>

      <!-- Close All Editors Button (on the right, aligned to last row) -->
      <div v-if="editorFiles.length > 0" class="ml-auto shrink-0 border-l border-subtle">
        <button @click="closeAllEditors"
          class="px-2 py-1 text-xs text-text-muted hover:text-text-primary hover:bg-bg-tertiary transition-all duration-normal"
          :title="t('terminalTabArea.closeAllEditors')">
          {{ t('terminalTabArea.closeAllEditors') }}
        </button>
      </div>
    </div>

    <!-- Tab Content -->
    <div class="min-h-0 flex-1 overflow-hidden">
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
