<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, shallowRef, nextTick } from "vue";
import * as monaco from "monaco-editor";
import { invoke } from "@tauri-apps/api/core";
import { X, Save, Loader2 } from "lucide-vue-next";
import { useNotificationStore } from "../stores/notifications";

const props = defineProps<{
  show: boolean;
  sessionId: string;
  filePath: string;
  fileName: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save"): void;
  (e: "close-with-unsaved-changes"): void;
}>();

const editorContainer = ref<HTMLElement | null>(null);
const editor = shallowRef<monaco.editor.IStandaloneCodeEditor | null>(null);
const notificationStore = useNotificationStore();
const isLoading = ref(false);
const isSaving = ref(false);
const originalContent = ref("");
const isDirty = ref(false);
const selectedLanguage = ref("plaintext");

// Cache to store edited content for each file
const fileContentCache = ref<Map<string, { content: string; originalContent: string; isDirty: boolean }>>(new Map());

// Confirmation dialog state
const showConfirmDialog = ref(false);

// Available languages for syntax highlighting
const availableLanguages = [
  { value: "plaintext", label: "Plain Text" },
  { value: "javascript", label: "JavaScript" },
  { value: "typescript", label: "TypeScript" },
  { value: "python", label: "Python" },
  { value: "rust", label: "Rust" },
  { value: "html", label: "HTML" },
  { value: "css", label: "CSS" },
  { value: "json", label: "JSON" },
  { value: "markdown", label: "Markdown" },
  { value: "shell", label: "Shell" },
  { value: "yaml", label: "YAML" },
  { value: "xml", label: "XML" },
  { value: "sql", label: "SQL" },
  { value: "go", label: "Go" },
  { value: "java", label: "Java" },
  { value: "cpp", label: "C/C++" },
  { value: "csharp", label: "C#" },
  { value: "php", label: "PHP" },
  { value: "dockerfile", label: "Dockerfile" },
  { value: "ini", label: "INI" },
  { value: "bat", label: "Batch" },
  { value: "powershell", label: "PowerShell" }
];

// Language detection based depreciated - now used for auto-selection only
function getLanguage(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "js":
      return "javascript";
    case "ts":
      return "typescript";
    case "py":
      return "python";
    case "rs":
      return "rust";
    case "html":
      return "html";
    case "css":
      return "css";
    case "json":
      return "json";
    case "md":
      return "markdown";
    case "vue":
      return "html"; // Monaco doesn't have vue out of box, html is close enough for basic
    case "sh":
      return "shell";
    case "yaml":
    case "yml":
      return "yaml";
    case "xml":
      return "xml";
    case "sql":
      return "sql";
    case "go":
      return "go";
    case "java":
      return "java";
    case "c":
    case "cpp":
    case "h":
      return "cpp";
    case "cs":
      return "csharp";
    case "php":
      return "php";
    default:
      return "plaintext";
  }
}

// Save current content to cache before switching files
function saveCurrentContentToCache() {
  if (!editor.value || !props.filePath) return;

  const currentContent = editor.value.getValue();
  const cacheKey = `${props.sessionId}:${props.filePath}`;

  fileContentCache.value.set(cacheKey, {
    content: currentContent,
    originalContent: originalContent.value,
    isDirty: currentContent !== originalContent.value
  });
}

// Get cached content for a file
function getCachedContent(filePath: string): { content: string; originalContent: string; isDirty: boolean } | null {
  const cacheKey = `${props.sessionId}:${filePath}`;
  return fileContentCache.value.get(cacheKey) || null;
}

// Handle close request with confirmation
function handleClose() {
  if (isDirty.value) {
    showConfirmDialog.value = true;
  } else {
    // Save current content to cache before closing
    saveCurrentContentToCache();
    // Dispose editor to free resources
    if (editor.value) {
      editor.value.dispose();
      editor.value = null;
    }
    emit("close");
  }
}

// Confirm closing without saving
function confirmCloseWithoutSave() {
  showConfirmDialog.value = false;
  // Save current content to cache even if not saved to remote
  saveCurrentContentToCache();
  // Dispose editor
  if (editor.value) {
    editor.value.dispose();
    editor.value = null;
  }
  emit("close");
}

// Save and then close
function saveAndClose() {
  showConfirmDialog.value = false;
  saveFile().then(() => {
    // Dispose editor
    if (editor.value) {
      editor.value.dispose();
      editor.value = null;
    }
    emit("close");
  });
}

// Cancel close
function cancelClose() {
  showConfirmDialog.value = false;
}

async function loadFile() {
  if (!props.filePath) {
    console.warn('No file path provided');
    return;
  }

  console.log('Loading file:', props.filePath);

  // REMOVED: saveCurrentContentToCache(); - This was causing the bug!

  isLoading.value = true;
  try {
    // Check if we have cached content for this file
    const cached = getCachedContent(props.filePath);
    let content: string;

    if (cached) {
      // Use cached content and restore original content
      content = cached.content;
      originalContent.value = cached.originalContent;
      isDirty.value = cached.isDirty;
      console.log('Using cached content');
    } else {
      // Load from remote
      console.log('Loading from remote:', props.filePath);
      content = await invoke<string>("read_remote_file", {
        id: props.sessionId,
        path: props.filePath,
        maxBytes: 1024 * 1024 * 5, // 5MB limit for now
      });
      originalContent.value = content;
      isDirty.value = false;
      console.log('Remote content loaded, length:', content.length);
    }

    // Initialize editor if it doesn't exist
    if (!editor.value) {
      await nextTick();
      await initEditor();
    }

    // Wait a bit more for editor to be ready
    await nextTick();

    // Set selected language based on file detection
    selectedLanguage.value = getLanguage(props.fileName);

    if (editor.value) {
      const model = editor.value.getModel();
      if (model) {
        model.setValue(content);
        monaco.editor.setModelLanguage(model, selectedLanguage.value);
        console.log('Content set to existing model');
      } else {
        const newModel = monaco.editor.createModel(
          content,
          selectedLanguage.value
        );
        editor.value.setModel(newModel);
        console.log('New model created and set');
      }
      // Force layout update to ensure content is visible
      editor.value.layout();
    } else {
      console.error('Editor still not available after initialization');
    }
  } catch (e) {
    console.error('Failed to load file:', e);
    notificationStore.error(`Failed to load file: ${e}`);
    emit("close");
  } finally {
    isLoading.value = false;
  }
}

async function saveFile() {
  if (!editor.value || !props.filePath) return;

  isSaving.value = true;
  try {
    const content = editor.value.getValue();
    await invoke("write_remote_file", {
      id: props.sessionId,
      path: props.filePath,
      content: content,
    });

    // Update cache and original content after successful save
    originalContent.value = content;
    isDirty.value = false;

    const cacheKey = `${props.sessionId}:${props.filePath}`;
    fileContentCache.value.set(cacheKey, {
      content: content,
      originalContent: content,
      isDirty: false
    });

    notificationStore.success("File saved successfully");
    emit("save");
  } catch (e) {
    notificationStore.error(`Failed to save file: ${e}`);
  } finally {
    isSaving.value = false;
  }
}

async function initEditor() {
  if (!editorContainer.value) {
    console.error('Editor container not ready');
    return;
  }

  // 如果编辑器已存在，先销毁
  if (editor.value) {
    editor.value.dispose();
    editor.value = null;
  }

  try {
    editor.value = monaco.editor.create(editorContainer.value, {
      value: "",
      language: "plaintext",
      theme: "vs-dark",
      automaticLayout: true,
      minimap: { enabled: true },
      scrollBeyondLastLine: false,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    });

    editor.value.onDidChangeModelContent(() => {
      if (editor.value) {
        isDirty.value = editor.value.getValue() !== originalContent.value;
      }
    });

    // Add Ctrl+S command
    editor.value.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      saveFile();
    });

    // Force layout
    setTimeout(() => {
      editor.value?.layout();
    }, 100);

    console.log('Editor initialized successfully');
  } catch (e) {
    console.error('Failed to initialize editor:', e);
    notificationStore.error(`Failed to initialize editor: ${e}`);
  }
}

watch(
  () => props.show,
  async (newVal) => {
    if (newVal) {
      // Wait for DOM to be ready
      await nextTick();
      console.log('FileEditorModal show changed to true, initializing...');

      if (!editor.value) {
        await initEditor();
      }
      loadFile();
    }
  }
);

watch(
  () => props.filePath,
  async (newPath, oldPath) => {
    if (props.show) {
      await nextTick();
      console.log('File path changed from', oldPath, 'to', newPath);

      // 1. Save content to old path cache if applicable
      if (oldPath && editor.value) {
        const currentContent = editor.value.getValue();
        const cacheKey = `${props.sessionId}:${oldPath}`;
        // Only save if we have a valid old path
        fileContentCache.value.set(cacheKey, {
          content: currentContent,
          originalContent: originalContent.value,
          isDirty: currentContent !== originalContent.value
        });
        console.log('Saved cache for old path:', oldPath);
      }

      if (!editor.value) {
        await initEditor();
      }
      loadFile();
    }
  }
);

watch(
  () => props.fileName,
  () => {
    if (props.show && editor.value) {
      // Auto-update language selection when file changes
      selectedLanguage.value = getLanguage(props.fileName);
      handleLanguageChange();
    }
  }
);

// Check if a file has unsaved changes
function hasUnsavedChanges(filePath: string): boolean {
  // Check current file
  if (props.filePath === filePath) {
    return isDirty.value;
  }
  // Check cache
  const cached = getCachedContent(filePath);
  return cached ? cached.isDirty : false;
}

// Handle language change
function handleLanguageChange() {
  if (editor.value) {
    const model = editor.value.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, selectedLanguage.value);
    }
  }
}

// Trigger close flow (for parent component to call)
function triggerClose() {
  handleClose();
}

onMounted(async () => {
  // Initialize editor when component mounts if it's supposed to be shown
  if (props.show) {
    await nextTick();
    console.log('FileEditorModal mounted, initializing...');
    await initEditor();
    loadFile();
  }
});

onUnmounted(() => {
  if (editor.value) {
    editor.value.dispose();
  }
});

defineExpose({
  hasUnsavedChanges,
  triggerClose
});
</script>

<template>
  <div v-if="show" class="h-full w-full flex flex-col bg-gray-900 text-white">
    <!-- Header -->
    <div class="h-12 border-b border-gray-700 flex items-center justify-between px-4 bg-gray-800 flex-shrink-0">
      <div class="flex items-center space-x-4">
        <span class="font-bold text-sm text-gray-300">{{ fileName }}</span>
        <span v-if="isDirty" class="text-xs text-yellow-500 italic">(Modified)</span>
        <span class="text-xs text-gray-500">{{ filePath }}</span>
      </div>
      <div class="flex items-center space-x-2">
        <!-- Language Selector -->
        <select v-model="selectedLanguage" @change="handleLanguageChange"
          class="px-3 py-1.5 text-sm bg-gray-700 text-white border border-gray-600 rounded focus:outline-none focus:border-blue-500">
          <option v-for="lang in availableLanguages" :key="lang.value" :value="lang.value">
            {{ lang.label }}
          </option>
        </select>
        <button @click="saveFile" :disabled="isSaving || !isDirty"
          class="flex items-center px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed rounded transition-colors">
          <Loader2 v-if="isSaving" class="w-4 h-4 mr-2 animate-spin" />
          <Save v-else class="w-4 h-4 mr-2" />
          Save
        </button>
        <button @click="handleClose"
          class="p-1.5 hover:bg-gray-700 rounded text-gray-400 hover:text-white transition-colors">
          <X class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Editor Body -->
    <div class="flex-1 relative overflow-hidden">
      <div v-if="isLoading" class="absolute inset-0 flex items-center justify-center bg-gray-900 z-10">
        <Loader2 class="w-8 h-8 text-blue-500 animate-spin" />
      </div>
      <div ref="editorContainer" class="w-full h-full"></div>
    </div>

    <!-- Confirmation Dialog -->
    <div v-if="showConfirmDialog" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="bg-gray-800 text-white rounded-lg p-6 max-w-md mx-4">
        <h3 class="text-lg font-semibold mb-4">Unsaved Changes</h3>
        <p class="text-gray-300 mb-6">
          Do you want to save the changes to "{{ fileName }}" before closing?
        </p>
        <div class="flex justify-end space-x-3">
          <button @click="confirmCloseWithoutSave"
            class="px-4 py-2 text-sm bg-gray-600 hover:bg-gray-500 rounded transition-colors">
            Don't Save
          </button>
          <button @click="cancelClose"
            class="px-4 py-2 text-sm bg-gray-600 hover:bg-gray-500 rounded transition-colors">
            Cancel
          </button>
          <button @click="saveAndClose" :disabled="isSaving"
            class="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed rounded transition-colors flex items-center">
            <Loader2 v-if="isSaving" class="w-4 h-4 mr-2 animate-spin" />
            Save & Close
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
