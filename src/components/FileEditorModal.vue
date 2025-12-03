<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, shallowRef } from 'vue';
import * as monaco from 'monaco-editor';
import { invoke } from '@tauri-apps/api/core';
import { X, Save, Loader2 } from 'lucide-vue-next';
import { useNotificationStore } from '../stores/notifications';

const props = defineProps<{
    show: boolean;
    sessionId: string;
    filePath: string;
    fileName: string;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
    (e: 'save'): void;
}>();

const editorContainer = ref<HTMLElement | null>(null);
const editor = shallowRef<monaco.editor.IStandaloneCodeEditor | null>(null);
const notificationStore = useNotificationStore();
const isLoading = ref(false);
const isSaving = ref(false);
const originalContent = ref('');
const isDirty = ref(false);

// Language detection based on extension
function getLanguage(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase();
    switch (ext) {
        case 'js': return 'javascript';
        case 'ts': return 'typescript';
        case 'py': return 'python';
        case 'rs': return 'rust';
        case 'html': return 'html';
        case 'css': return 'css';
        case 'json': return 'json';
        case 'md': return 'markdown';
        case 'vue': return 'html'; // Monaco doesn't have vue out of box, html is close enough for basic
        case 'sh': return 'shell';
        case 'yaml':
        case 'yml': return 'yaml';
        case 'xml': return 'xml';
        case 'sql': return 'sql';
        case 'go': return 'go';
        case 'java': return 'java';
        case 'c':
        case 'cpp':
        case 'h': return 'cpp';
        case 'cs': return 'csharp';
        case 'php': return 'php';
        default: return 'plaintext';
    }
}

async function loadFile() {
    if (!props.filePath) return;
    
    isLoading.value = true;
    try {
        const content = await invoke<string>('read_remote_file', {
            id: props.sessionId,
            path: props.filePath,
            maxBytes: 1024 * 1024 * 5 // 5MB limit for now
        });
        
        originalContent.value = content;
        if (editor.value) {
            const model = editor.value.getModel();
            if (model) {
                model.setValue(content);
                monaco.editor.setModelLanguage(model, getLanguage(props.fileName));
            } else {
                const newModel = monaco.editor.createModel(content, getLanguage(props.fileName));
                editor.value.setModel(newModel);
            }
        }
        isDirty.value = false;
    } catch (e) {
        notificationStore.error(`Failed to load file: ${e}`);
        emit('close');
    } finally {
        isLoading.value = false;
    }
}

async function saveFile() {
    if (!editor.value || !props.filePath) return;
    
    isSaving.value = true;
    try {
        const content = editor.value.getValue();
        await invoke('write_remote_file', {
            id: props.sessionId,
            path: props.filePath,
            content: content
        });
        originalContent.value = content;
        isDirty.value = false;
        notificationStore.success('File saved successfully');
        emit('save');
    } catch (e) {
        notificationStore.error(`Failed to save file: ${e}`);
    } finally {
        isSaving.value = false;
    }
}

function initEditor() {
    if (!editorContainer.value) return;

    editor.value = monaco.editor.create(editorContainer.value, {
        value: '',
        language: 'plaintext',
        theme: 'vs-dark',
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
}

watch(() => props.show, (newVal) => {
    if (newVal) {
        // Wait for DOM to be ready
        setTimeout(() => {
            if (!editor.value) {
                initEditor();
            }
            loadFile();
        }, 100);
    }
});

onMounted(() => {
    if (props.show) {
        initEditor();
        loadFile();
    }
});

onUnmounted(() => {
    if (editor.value) {
        editor.value.dispose();
    }
});
</script>

<template>
    <div v-if="show" class="fixed inset-0 z-50 flex flex-col bg-gray-900 text-white">
        <!-- Header -->
        <div class="h-12 border-b border-gray-700 flex items-center justify-between px-4 bg-gray-800">
            <div class="flex items-center space-x-4">
                <span class="font-bold text-sm text-gray-300">{{ fileName }}</span>
                <span v-if="isDirty" class="text-xs text-yellow-500 italic">(Modified)</span>
                <span class="text-xs text-gray-500">{{ filePath }}</span>
            </div>
            <div class="flex items-center space-x-2">
                <button @click="saveFile" :disabled="isSaving || !isDirty"
                    class="flex items-center px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed rounded transition-colors">
                    <Loader2 v-if="isSaving" class="w-4 h-4 mr-2 animate-spin" />
                    <Save v-else class="w-4 h-4 mr-2" />
                    Save
                </button>
                <button @click="$emit('close')" class="p-1.5 hover:bg-gray-700 rounded text-gray-400 hover:text-white transition-colors">
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
    </div>
</template>
