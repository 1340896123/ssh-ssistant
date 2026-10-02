<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, shallowRef, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { X, Save, Loader2 } from "lucide-vue-next";
import { useNotificationStore } from "../stores/notifications";
import { useI18n } from "../composables/useI18n";
import { terminalTheme } from "../utils/terminalTheme";

type MonacoModule = typeof import("../monaco/monaco-lite");
type MonacoEditor = import("monaco-editor").editor.IStandaloneCodeEditor;
type MonarchLanguageModule = {
  conf: unknown;
  language: unknown;
};

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
const editor = shallowRef<MonacoEditor | null>(null);
const notificationStore = useNotificationStore();
const { t } = useI18n();
const isLoading = ref(false);
const isSaving = ref(false);
const isLanguageSupportLoading = ref(false);
const isInitialFileReady = ref(false);
const currentLoadedPath = ref<string | null>(null);
const originalContent = ref("");
const isDirty = ref(false);
const selectedLanguage = ref("plaintext");
let monacoModule: MonacoModule | null = null;
let languageLoadRequestId = 0;
let loadFileRequestId = 0;
let localEditVersion = 0;
let isApplyingLoadedContent = false;
const loadedLanguageModules = new Set<string>();
const registeredMonacoLanguages = new Set<string>();
const KEY_MOD_CTRL_CMD = 2048;

const jsonLanguageRegistration = {
  id: "json",
  extensions: [".json", ".bowerrc", ".jshintrc", ".jscsrc", ".eslintrc", ".babelrc", ".har"],
  aliases: ["JSON", "json"],
  mimetypes: ["application/json"],
};

const jsonLanguageConfiguration = {
  wordPattern: /(-?\d*\.\d\w*)|([^\[\{\]\}\:\"\,\s]+)/g,
  comments: {
    lineComment: "//",
    blockComment: ["/*", "*/"] as [string, string],
  },
  brackets: [
    ["{", "}"] as [string, string],
    ["[", "]"] as [string, string],
  ],
  autoClosingPairs: [
    { open: "{", close: "}", notIn: ["string"] },
    { open: "[", close: "]", notIn: ["string"] },
    { open: '"', close: '"', notIn: ["string"] },
  ],
};

const monarchLanguageDefinitions: Record<
  string,
  {
    id: string;
    aliases?: string[];
    extensions?: string[];
    mimetypes?: string[];
    firstLine?: string;
    loader: () => Promise<unknown>;
  }
> = {
  javascript: {
    id: "javascript",
    aliases: ["JavaScript", "js"],
    extensions: [".js", ".es6", ".jsx", ".mjs", ".cjs"],
    loader: () =>
      import("monaco-editor/esm/vs/basic-languages/javascript/javascript.js"),
  },
  typescript: {
    id: "typescript",
    aliases: ["TypeScript", "ts"],
    extensions: [".ts", ".tsx", ".mts", ".cts"],
    loader: () =>
      import("monaco-editor/esm/vs/basic-languages/typescript/typescript.js"),
  },
  html: {
    id: "html",
    aliases: ["HTML", "htm"],
    extensions: [".html", ".htm", ".xhtml", ".vue"],
    loader: () =>
      import("monaco-editor/esm/vs/basic-languages/html/html.js"),
  },
  css: {
    id: "css",
    aliases: ["CSS", "css"],
    extensions: [".css"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/css/css.js"),
  },
  python: {
    id: "python",
    aliases: ["Python", "py"],
    extensions: [".py", ".rpy", ".pyw", ".cpy", ".gyp", ".gypi"],
    firstLine: "^#!/.*\\bpython[0-9.-]*\\b",
    loader: () => import("monaco-editor/esm/vs/basic-languages/python/python.js"),
  },
  rust: {
    id: "rust",
    aliases: ["Rust", "rs"],
    extensions: [".rs"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/rust/rust.js"),
  },
  markdown: {
    id: "markdown",
    aliases: ["Markdown", "md"],
    extensions: [".md", ".markdown"],
    loader: () =>
      import("monaco-editor/esm/vs/basic-languages/markdown/markdown.js"),
  },
  shell: {
    id: "shell",
    aliases: ["Shell", "sh"],
    extensions: [".sh", ".bash"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/shell/shell.js"),
  },
  yaml: {
    id: "yaml",
    aliases: ["YAML", "yaml"],
    extensions: [".yaml", ".yml"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/yaml/yaml.js"),
  },
  xml: {
    id: "xml",
    aliases: ["XML", "xml"],
    extensions: [".xml", ".xsd", ".dtd", ".ascx", ".csproj", ".config", ".wxi", ".wxl", ".wxs", ".xaml", ".svg", ".svgz", ".opf", ".xslt", ".xsl"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/xml/xml.js"),
  },
  sql: {
    id: "sql",
    aliases: ["SQL"],
    extensions: [".sql"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/sql/sql.js"),
  },
  go: {
    id: "go",
    aliases: ["Go"],
    extensions: [".go"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/go/go.js"),
  },
  java: {
    id: "java",
    aliases: ["Java", "java"],
    extensions: [".java", ".jav"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/java/java.js"),
  },
  cpp: {
    id: "cpp",
    aliases: ["C++", "Cpp", "cpp"],
    extensions: [".cpp", ".cc", ".cxx", ".hpp", ".hh", ".hxx", ".c", ".h"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/cpp/cpp.js"),
  },
  csharp: {
    id: "csharp",
    aliases: ["C#", "csharp"],
    extensions: [".cs", ".csx", ".cake"],
    loader: () =>
      import("monaco-editor/esm/vs/basic-languages/csharp/csharp.js"),
  },
  php: {
    id: "php",
    aliases: ["PHP", "php"],
    extensions: [".php", ".php4", ".php5", ".phtml", ".ctp"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/php/php.js"),
  },
  dockerfile: {
    id: "dockerfile",
    aliases: ["Dockerfile"],
    extensions: [".dockerfile"],
    loader: () =>
      import("monaco-editor/esm/vs/basic-languages/dockerfile/dockerfile.js"),
  },
  ini: {
    id: "ini",
    aliases: ["Ini", "ini"],
    extensions: [".ini", ".properties", ".gitconfig"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/ini/ini.js"),
  },
  bat: {
    id: "bat",
    aliases: ["Batch", "bat"],
    extensions: [".bat", ".cmd"],
    loader: () => import("monaco-editor/esm/vs/basic-languages/bat/bat.js"),
  },
  powershell: {
    id: "powershell",
    aliases: ["Powershell", "ps", "ps1"],
    extensions: [".ps1", ".psm1", ".psd1"],
    loader: () =>
      import("monaco-editor/esm/vs/basic-languages/powershell/powershell.js"),
  },
};

// Cache to store edited content for each file
const fileContentCache = ref<Map<string, { content: string; originalContent: string; isDirty: boolean }>>(new Map());

// Confirmation dialog state
const showConfirmDialog = ref(false);

async function getMonaco() {
  if (!monacoModule) {
    monacoModule = (await import("../monaco/monaco-lite")) as MonacoModule;
  }
  return monacoModule;
}

async function registerMonarchLanguage(languageKey: string) {
  const definition = monarchLanguageDefinitions[languageKey];
  if (!definition) return false;
  if (registeredMonacoLanguages.has(definition.id)) return true;

  const monaco = await getMonaco();
  const languages = await monaco.loadMonacoLanguages();
  languages.register({
    id: definition.id,
    aliases: definition.aliases,
    extensions: definition.extensions,
    mimetypes: definition.mimetypes,
    firstLine: definition.firstLine,
  });

  const languageModule = (await definition.loader()) as MonarchLanguageModule;
  languages.setLanguageConfiguration(
    definition.id,
    languageModule.conf as Parameters<
      Awaited<ReturnType<MonacoModule["loadMonacoLanguages"]>>["setLanguageConfiguration"]
    >[1],
  );
  languages.setMonarchTokensProvider(
    definition.id,
    languageModule.language as Parameters<
      Awaited<ReturnType<MonacoModule["loadMonacoLanguages"]>>["setMonarchTokensProvider"]
    >[1],
  );
  registeredMonacoLanguages.add(definition.id);
  return true;
}

async function registerJsonLanguage() {
  if (registeredMonacoLanguages.has(jsonLanguageRegistration.id)) return;

  const monaco = await getMonaco();
  const languages = await monaco.loadMonacoLanguages();
  const { createTokenizationSupport } = await import(
    "monaco-editor/esm/vs/language/json/tokenization.js"
  );

  languages.register(jsonLanguageRegistration);
  languages.setLanguageConfiguration(
    jsonLanguageRegistration.id,
    jsonLanguageConfiguration as Parameters<
      Awaited<ReturnType<MonacoModule["loadMonacoLanguages"]>>["setLanguageConfiguration"]
    >[1],
  );
  languages.setTokensProvider(
    jsonLanguageRegistration.id,
    createTokenizationSupport(true) as unknown as Parameters<
      Awaited<ReturnType<MonacoModule["loadMonacoLanguages"]>>["setTokensProvider"]
    >[1],
  );
  registeredMonacoLanguages.add(jsonLanguageRegistration.id);
}

async function ensureLanguageSupport(language: string) {
  const normalized = language.trim().toLowerCase();
  const loadKey =
    normalized === "javascript" || normalized === "typescript"
      ? "typescript-family"
      : normalized;

  if (loadedLanguageModules.has(loadKey)) {
    return;
  }

  switch (loadKey) {
    case "plaintext":
      break;
    case "typescript-family":
      await registerMonarchLanguage("javascript");
      await registerMonarchLanguage("typescript");
      break;
    case "html":
      await registerMonarchLanguage("html");
      break;
    case "css":
      await registerMonarchLanguage("css");
      break;
    case "json":
      await registerJsonLanguage();
      break;
    case "python":
      await registerMonarchLanguage("python");
      break;
    case "rust":
      await registerMonarchLanguage("rust");
      break;
    case "markdown":
      await registerMonarchLanguage("markdown");
      break;
    case "shell":
      await registerMonarchLanguage("shell");
      break;
    case "yaml":
      await registerMonarchLanguage("yaml");
      break;
    case "xml":
      await registerMonarchLanguage("xml");
      break;
    case "sql":
      await registerMonarchLanguage("sql");
      break;
    case "go":
      await registerMonarchLanguage("go");
      break;
    case "java":
      await registerMonarchLanguage("java");
      break;
    case "cpp":
      await registerMonarchLanguage("cpp");
      break;
    case "csharp":
      await registerMonarchLanguage("csharp");
      break;
    case "php":
      await registerMonarchLanguage("php");
      break;
    case "dockerfile":
      await registerMonarchLanguage("dockerfile");
      break;
    case "ini":
      await registerMonarchLanguage("ini");
      break;
    case "bat":
      await registerMonarchLanguage("bat");
      break;
    case "powershell":
      await registerMonarchLanguage("powershell");
      break;
    default:
      break;
  }

  loadedLanguageModules.add(loadKey);
}

async function applyLanguageToModel(language: string) {
  const requestId = ++languageLoadRequestId;
  const normalized = language.trim().toLowerCase() || "plaintext";

  if (normalized === "plaintext") {
    isLanguageSupportLoading.value = false;
    const monaco = await getMonaco();
    const model = editor.value?.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, "plaintext");
    }
    return;
  }

  isLanguageSupportLoading.value = true;
  try {
    await ensureLanguageSupport(normalized);
    if (requestId !== languageLoadRequestId) return;

    const monaco = await getMonaco();
    const model = editor.value?.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, normalized);
    }
  } finally {
    if (requestId === languageLoadRequestId) {
      isLanguageSupportLoading.value = false;
    }
  }
}

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

  const requestId = ++loadFileRequestId;
  const requestEditVersion = localEditVersion;

  console.log('Loading file:', props.filePath);

  // REMOVED: saveCurrentContentToCache(); - This was causing the bug!

  isLoading.value = true;
  isInitialFileReady.value = false;
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
    } else if (
      currentLoadedPath.value === props.filePath &&
      editor.value &&
      isDirty.value
    ) {
      content = editor.value.getValue();
      console.log('Preserving dirty content for same file reload');
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

    if (requestId !== loadFileRequestId) return;

    // Initialize editor if it doesn't exist
    if (!editor.value) {
      await nextTick();
      await initEditor();
    }

    // Wait a bit more for editor to be ready
    await nextTick();
    if (requestId !== loadFileRequestId) return;

    // Show content immediately and hydrate language support asynchronously.
    selectedLanguage.value = getLanguage(props.fileName);
    const monaco = await getMonaco();
    if (requestId !== loadFileRequestId) return;
    if (
      currentLoadedPath.value === props.filePath &&
      localEditVersion !== requestEditVersion &&
      editor.value &&
      isDirty.value
    ) {
      console.log('Skipping stale load because local edits exist');
      isInitialFileReady.value = true;
      return;
    }

    if (editor.value) {
      const model = editor.value.getModel();
      if (model) {
        isApplyingLoadedContent = true;
        model.setValue(content);
        monaco.editor.setModelLanguage(model, "plaintext");
        isApplyingLoadedContent = false;
        console.log('Content set to existing model');
      } else {
        const newModel = monaco.editor.createModel(
          content,
          "plaintext"
        );
        editor.value.setModel(newModel);
        console.log('New model created and set');
      }
      // Force layout update to ensure content is visible
      editor.value.layout();
      void applyLanguageToModel(selectedLanguage.value);
      currentLoadedPath.value = props.filePath;
      isInitialFileReady.value = true;
    } else {
      console.error('Editor still not available after initialization');
    }
  } catch (e) {
    console.error('Failed to load file:', e);
    notificationStore.error(t("fileEditor.notifications.loadFailed", { error: e }));
    emit("close");
  } finally {
    if (requestId === loadFileRequestId) {
      isLoading.value = false;
    }
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

    notificationStore.success(t("fileEditor.notifications.saveSuccess"));
    emit("save");
  } catch (e) {
    notificationStore.error(t("fileEditor.notifications.saveFailed", { error: e }));
  } finally {
    isSaving.value = false;
  }
}

async function initEditor() {
  if (!editorContainer.value) {
    console.error('Editor container not ready');
    return;
  }
  const monaco = await getMonaco();

  // 如果编辑器已存在，先销毁
  if (editor.value) {
    editor.value.dispose();
    editor.value = null;
  }

  try {
    const palette = terminalTheme();
    monaco.editor.defineTheme("graphite-console", {
      base: "vs-dark",
      inherit: true,
      rules: [],
      colors: {
        "editor.background": palette.background,
        "editor.foreground": palette.foreground,
        "editorCursor.foreground": palette.cursor,
        "editor.selectionBackground": palette.selectionBackground,
        "editorLineNumber.foreground": palette.brightBlack,
        "editorLineNumber.activeForeground": palette.foreground,
        "editorWidget.background": palette.background,
        "editorWidget.border": palette.black,
      },
    });
    const createdEditor = monaco.editor.create(editorContainer.value, {
      value: "",
      language: "plaintext",
      theme: "graphite-console",
      automaticLayout: true,
      minimap: { enabled: true },
      scrollBeyondLastLine: false,
      fontSize: 14,
      fontFamily: getComputedStyle(document.documentElement).getPropertyValue('--font-mono').trim(),
    });
    editor.value = createdEditor;
    syncCodexTestApi();

    createdEditor.onDidChangeModelContent(() => {
      if (!isApplyingLoadedContent) {
        localEditVersion += 1;
      }
      if (editor.value) {
        isDirty.value = editor.value.getValue() !== originalContent.value;
      }
    });

    // Add Ctrl+S command
    createdEditor.addCommand(KEY_MOD_CTRL_CMD | 49, () => {
      saveFile();
    });

    // Force layout
    setTimeout(() => {
      editor.value?.layout();
    }, 100);

    console.log('Editor initialized successfully');
  } catch (e) {
    console.error('Failed to initialize editor:', e);
    notificationStore.error(t("fileEditor.notifications.initFailed", { error: e }));
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
async function handleLanguageChange() {
  if (editor.value) {
    await applyLanguageToModel(selectedLanguage.value);
  }
}

// Trigger close flow (for parent component to call)
function triggerClose() {
  handleClose();
}

async function __codexSetEditorContent(content: string) {
  for (let attempt = 0; attempt < 200; attempt++) {
    if (!editor.value) {
      await nextTick();
      await initEditor();
    }

    if (!isInitialFileReady.value) {
      await new Promise((resolve) => setTimeout(resolve, 50));
      continue;
    }

    const currentEditor = editor.value;
    const model = currentEditor?.getModel();
    if (currentEditor && model) {
      model.setValue(content);
      localEditVersion += 1;
      isDirty.value = content !== originalContent.value;
      if (props.filePath) {
        const cacheKey = `${props.sessionId}:${props.filePath}`;
        fileContentCache.value.set(cacheKey, {
          content,
          originalContent: originalContent.value,
          isDirty: isDirty.value,
        });
      }
      return 'ok';
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  if (!editor.value) return 'no-editor';
  if (!editor.value.getModel()) return 'no-model';
  return 'unknown';
}

function syncCodexTestApi() {
  if (typeof window === 'undefined') return;
  const testWindow = window as any;
  if (!testWindow.__CODEX_FILE_EDITOR_TEST__) return;
  testWindow.__codexFileEditorTestApi = {
    __codexSetEditorContent,
    isInitialFileReady: () => isInitialFileReady.value,
    hasUnsavedChanges,
    triggerClose,
  };
}

onMounted(async () => {
  // Initialize editor when component mounts if it's supposed to be shown
  syncCodexTestApi();
  if (props.show) {
    await nextTick();
    console.log('FileEditorModal mounted, initializing...');
    await initEditor();
    loadFile();
  }
});

onUnmounted(() => {
  if (typeof window !== 'undefined') {
    const testWindow = window as any;
    if (testWindow.__codexFileEditorTestApi?.__codexSetEditorContent === __codexSetEditorContent) {
      testWindow.__codexFileEditorTestApi = null;
    }
  }
  if (editor.value) {
    editor.value.dispose();
  }
});

defineExpose({
  hasUnsavedChanges,
  triggerClose,
  __codexSetEditorContent
});
</script>

<template>
  <div v-if="show" class="flex h-full w-full min-h-0 flex-col bg-bg-primary text-text-primary" data-testid="file-editor-root">
    <!-- Header -->
    <div class="min-h-12 border-b border-border-primary flex flex-wrap items-center justify-between gap-2 px-4 py-2 bg-bg-elevated flex-shrink-0" data-testid="file-editor-header">
      <div class="flex min-w-0 flex-wrap items-center gap-2">
        <span class="font-bold text-sm text-text-primary" data-testid="file-editor-name">{{ fileName }}</span>
        <span v-if="isDirty" class="text-xs text-warning italic" data-testid="file-editor-dirty">({{ t('fileEditor.modified') }})</span>
        <span v-if="isLanguageSupportLoading" class="text-xs text-text-muted" data-testid="file-editor-syntax-loading">
          Loading syntax...
        </span>
        <span class="max-w-[18rem] truncate font-mono text-xs text-text-muted" :title="filePath" data-testid="file-editor-path">{{ filePath }}</span>
      </div>
      <div class="flex items-center space-x-2">
        <!-- Language Selector -->
        <select v-model="selectedLanguage" @change="handleLanguageChange" data-testid="file-editor-language"
          class="px-3 py-1.5 text-sm bg-bg-tertiary text-text-primary border border-border-primary rounded focus:outline-none focus:border-accent">
          <option v-for="lang in availableLanguages" :key="lang.value" :value="lang.value">
            {{ lang.label }}
          </option>
        </select>
        <button @click="saveFile" :disabled="isSaving || !isDirty" data-testid="file-editor-save"
          class="flex items-center px-3 py-1.5 text-sm bg-accent hover:bg-accent/80 disabled:opacity-50 disabled:cursor-not-allowed rounded transition-colors text-text-on-accent">
          <Loader2 v-if="isSaving" class="w-4 h-4 mr-2 animate-spin" />
          <Save v-else class="w-4 h-4 mr-2" />
          {{ t('fileEditor.save') }}
        </button>
        <button @click="handleClose" data-testid="file-editor-close"
          class="p-1.5 hover:bg-bg-tertiary rounded text-text-muted hover:text-text-primary transition-colors">
          <X class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Editor Body -->
    <div class="relative min-h-0 flex-1 overflow-hidden" data-testid="file-editor-body">
      <div v-if="isLoading" class="absolute inset-0 flex items-center justify-center bg-bg-primary z-10" data-testid="file-editor-loading">
        <Loader2 class="w-8 h-8 text-accent animate-spin" />
      </div>
      <div ref="editorContainer" class="w-full h-full" data-testid="file-editor-container"></div>
    </div>

    <!-- Confirmation Dialog -->
    <div v-if="showConfirmDialog" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" data-testid="file-editor-confirm-overlay">
      <div class="bg-bg-elevated text-text-primary rounded-lg p-6 max-w-md mx-4" data-testid="file-editor-confirm-dialog">
        <h3 class="text-lg font-semibold mb-4">{{ t('fileEditor.unsaved.title') }}</h3>
        <p class="text-text-secondary mb-6">
          {{ t('fileEditor.unsaved.description', { name: fileName }) }}
        </p>
        <div class="flex justify-end space-x-3">
          <button @click="confirmCloseWithoutSave" data-testid="file-editor-confirm-discard"
            class="px-4 py-2 text-sm bg-bg-tertiary hover:bg-bg-tertiary/80 rounded transition-colors">
            {{ t('fileEditor.unsaved.discard') }}
          </button>
          <button @click="cancelClose" data-testid="file-editor-confirm-cancel"
            class="px-4 py-2 text-sm bg-bg-tertiary hover:bg-bg-tertiary/80 rounded transition-colors">
            {{ t('fileEditor.unsaved.cancel') }}
          </button>
          <button @click="saveAndClose" :disabled="isSaving" data-testid="file-editor-confirm-save-close"
            class="px-4 py-2 text-sm bg-accent hover:bg-accent/80 disabled:opacity-50 disabled:cursor-not-allowed rounded transition-colors flex items-center text-text-on-accent">
            <Loader2 v-if="isSaving" class="w-4 h-4 mr-2 animate-spin" />
            {{ t('fileEditor.unsaved.saveAndClose') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
