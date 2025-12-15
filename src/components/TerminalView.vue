<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { SearchAddon } from 'xterm-addon-search';
import * as Zmodem from 'zmodem.js';
import { save, open } from '@tauri-apps/plugin-dialog';
import { writeFile, readFile } from '@tauri-apps/plugin-fs';
import 'xterm/css/xterm.css';
import { Send, Sparkles, Terminal as TerminalIcon, Search, X, ArrowUp, ArrowDown, RotateCw, Unplug, Eraser } from 'lucide-vue-next';
import { useSettingsStore } from '../stores/settings';
import { useSessionStore } from '../stores/sessions';

const props = defineProps<{ sessionId: string }>();

function getContent(): string {
  if (!term) return '';

  const buffer = term.buffer.active;
  if (!buffer) return '';

  const numLines = buffer.length;
  const lines: string[] = [];
  // Let's grab up to 50 recent lines.
  const linesToGrab = Math.min(50, numLines);

  for (let i = numLines - linesToGrab; i < numLines; i++) {
    const line = buffer.getLine(i);
    if (line) {
      lines.push(line.translateToString());
    }
  }

  return lines.join('\n');
}

defineExpose({
  getContent,
});

const terminalContainer = ref<HTMLElement | null>(null);
const settingsStore = useSettingsStore();
const sessionStore = useSessionStore();

const currentSession = computed(() => sessionStore.sessions.find(s => s.id === props.sessionId));

function handleReconnect() {
  if (props.sessionId) {
    term?.clear();
    sessionStore.reconnectSession(props.sessionId);
  }
}

function handleDisconnect() {
  if (props.sessionId) {
    term?.write('\r\n\x1b[1;31m❌ Connection disconnected\x1b[0m\r\n');
    sessionStore.disconnectSession(props.sessionId);
  }
}

function handleClearLog() {
  term?.clear();
}

let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let searchAddon: SearchAddon | null = null;
let unlisten: (() => void) | null = null;
let zmodemSentry: any = null;

// Search State
const showSearch = ref(false);
const searchText = ref('');
const searchInputRef = ref<HTMLInputElement | null>(null);

// Input Box State
const commandInput = ref('');
const inputRef = ref<HTMLInputElement | null>(null);
const previewText = ref(''); // Gray preview text
const commandHistory = ref<string[]>([]);
const historyIndex = ref(-1);

// Completion State
const traditionalCompletions = ref<string[]>([]);
const selectedTraditionalIndex = ref(0);
const aiCompletions = ref<string[]>([]);
const selectedAiIndex = ref(0);
const showAiCompletions = ref(false);

const isAiLoading = ref(false);

// Context Tracking
const currentDir = ref('.');
const homeDir = ref('');
const remoteShell = ref('bash'); // Default to bash

interface FileEntry {
  name: string;
  is_dir: boolean;
}

onMounted(async () => {
  if (!terminalContainer.value) return;

  const appearance = settingsStore.terminalAppearance;

  term = new Terminal({
    cursorBlink: true,
    fontSize: appearance.fontSize,
    fontFamily: appearance.fontFamily,
    cursorStyle: appearance.cursorStyle,
    lineHeight: appearance.lineHeight,
    theme: {
      background: '#000000',
    },
    allowProposedApi: true
  });

  fitAddon = new FitAddon();
  searchAddon = new SearchAddon();
  term.loadAddon(fitAddon);
  term.loadAddon(searchAddon);

  // Zmodem Integration
  zmodemSentry = new Zmodem.Sentry({
    to_terminal: (octets: any) => {
        // console.log('Zmodem sentry to_terminal', octets.byteLength || octets.length);
        term?.write(octets);
    },
    sender: (octets: any) => {
      // Send binary data to PTY
      // Convert octets (Array of numbers) to Uint8Array then to Array? 
      // Tauri invoke args need to be serializable. 
      // write_binary_to_pty expects Vec<u8>. 
      // We can pass number[] directly.
      invoke('write_binary_to_pty', {
        id: props.sessionId,
        data: Array.from(octets)
      });
    },
    on_detect: (detection: any) => {
      const zsession = detection.confirm();
      if (zsession.type === "receive") {
        handleZmodemDownload(zsession);
      } else {
        handleZmodemUpload(zsession);
      }
    },
    on_retract: () => {
      // console.log("Zmodem retracted");
    }
  });

  term.open(terminalContainer.value);
  fitAddon.fit();

  // OSC 7 Handler for CWD tracking
  term.parser.registerOscHandler(7, (data) => {
    try {
      const url = new URL(data);
      if (url.pathname) {
        let path = decodeURIComponent(url.pathname);
        currentDir.value = path;
      }
    } catch (e) {
      console.warn("Failed to parse OSC 7:", e);
    }
    return true;
  });

  // Listen to user input (direct terminal interaction)
  term.onData((data) => {
    if (currentSession.value?.status !== 'connected') return;
    invoke('write_to_pty', { id: props.sessionId, data });
  });

  // Listen to resize events
  window.addEventListener('resize', handleResize);
  term.onResize((size) => {
    invoke('resize_pty', {
      id: props.sessionId,
      rows: size.rows,
      cols: size.cols
    });
  });

  // Ctrl+F Handler
  term.attachCustomKeyEventHandler((arg) => {
    if (arg.ctrlKey && arg.code === "KeyF" && arg.type === "keydown") {
      showSearch.value = !showSearch.value;
      if (showSearch.value) {
        setTimeout(() => searchInputRef.value?.focus(), 100);
      } else {
        term?.focus();
      }
      return false;
    }
    return true;
  });

  // Initial resize
  setTimeout(() => {
    handleResize();
  }, 100);

  // Listen to backend data
  unlisten = await listen<number[]>(`term-data:${props.sessionId}`, (event) => {
    // console.log('term-data event received', props.sessionId, event.payload.length);
    const data = new Uint8Array(event.payload);
    // Feed through Zmodem Sentry
    zmodemSentry.consume(data);
  });

  const unlistenExit = await listen(`term-exit:${props.sessionId}`, () => {
    console.log('term-exit event received', props.sessionId);
    // Check if we are still connected before printing message to avoid duplicates
    // if handleDisconnect was called.
    if (currentSession.value?.status === 'connected') {
      console.log('term-exit: marking as disconnected');
      term?.write('\r\n\x1b[1;31m❌ Connection disconnected\x1b[0m\r\n');
      sessionStore.updateSessionStatus(props.sessionId, 'disconnected');
    } else {
      console.log('term-exit: already disconnected (or session not found)', currentSession.value?.status);
    }
  });

  // Watch for status changes to print message
  watch(
    () => currentSession.value?.status,
    (newStatus, oldStatus) => {
      if (newStatus === 'connected' && oldStatus === 'disconnected') {
        // Optional: Clear or print connected message
        term?.write('\r\n\x1b[1;32m✔ Connection established\x1b[0m\r\n');
      }
    }
  );

  // Ensure focus and initial resize
  term?.focus();
  setTimeout(() => {
    handleResize();
    term?.focus();
  }, 200);

  const oldUnlisten = unlisten;
  unlisten = () => {
    if (oldUnlisten) oldUnlisten();
    unlistenExit();
  };
  // Initialize Context (Non-blocking)
  (async () => {
    try {
      // 1. Get Home Directory
      const pwd = await invoke<string>('exec_command', {
        id: props.sessionId,
        command: 'pwd'
      });
      if (pwd) {
        homeDir.value = pwd.trim();
        currentDir.value = homeDir.value;
      }

      // 2. Detect Shell
      const shell = await invoke<string>('exec_command', {
        id: props.sessionId,
        command: 'echo $SHELL'
      });
      if (shell) {
        const shellName = shell.trim().split('/').pop();
        if (shellName) remoteShell.value = shellName;
      }
    } catch (e) {
      console.error("Failed to init context:", e);
    }
  })();
});

async function handleZmodemDownload(zsession: any) {
  zsession.on("offer", async (xfer: any) => {
    const offer = xfer.get_details();
    try {
      const savePath = await save({
        defaultPath: offer.name,
        title: 'Save Downloaded File'
      });

      if (savePath) {
        const fileBuffer: number[] = [];
        xfer.on("input", (payload: any) => {
          // accumulate payload
          for (let i = 0; i < payload.length; i++) {
            fileBuffer.push(payload[i]);
          }
          // update progress if needed
        });

        xfer.accept().then(() => {
          // Write to file
          const uint8 = new Uint8Array(fileBuffer);
          writeFile(savePath, uint8).then(() => {
            term?.write(`\r\nSaved to ${savePath}\r\n`);
          });
        });
      } else {
        xfer.skip();
      }
    } catch (e) {
      console.error("Download error:", e);
      xfer.skip();
    }
  });
  zsession.start();
}

async function handleZmodemUpload(zsession: any) {
  try {
    const selected = await open({
      multiple: true,
      title: 'Select files to upload'
    });

    if (selected && selected.length > 0) {
      // Handle one by one or batch? Zmodem supports batch.
      // We need to construct file objects for zmodem.js
      const files = Array.isArray(selected) ? selected : [selected];

      const fileObjects = [];
      for (const path of files) {
        const name = path.split(/[\\/]/).pop() || "unknown";
        const data = await readFile(path);
        fileObjects.push({
          name: name,
          size: data.length,
          obj: data
        });
      }

      Zmodem.Browser.send_files(zsession, fileObjects, {
        on_offer_response(_obj: any, _xfer: any) {
          // console.log("offer response", xfer);
        },
        on_progress(_obj: any, _xfer: any, _buffer: any) {
          // console.log("progress", xfer.get_offset());
        },
        on_file_complete(_obj: any, _xfer: any) {
          // console.log("file complete");
        }
      }).then(() => {
        zsession.close();
      }).catch((e: any) => {
        console.error("Upload session error", e);
        zsession.close();
      });

    } else {
      zsession.close();
    }
  } catch (e) {
    console.error("Upload error:", e);
    zsession.close();
  }
}

function handleResize() {
  fitAddon?.fit();
}

function searchNext() {
  searchAddon?.findNext(searchText.value);
}

function searchPrev() {
  searchAddon?.findPrevious(searchText.value);
}

function closeSearch() {
  showSearch.value = false;
  term?.focus();
}

watch(searchText, (val) => {
  if (val) {
    searchAddon?.findNext(val);
  }
});

watch(
  () => settingsStore.terminalAppearance,
  (val) => {
    if (!term) return;
    term.options.fontSize = val.fontSize;
    term.options.fontFamily = val.fontFamily;
    term.options.cursorStyle = val.cursorStyle;
    term.options.lineHeight = val.lineHeight;
  },
  { deep: true }
);

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  if (unlisten) unlisten();
  term?.dispose();
});

// --- Input Box Logic ---

// Watch input for traditional completion
// Watch input for traditional completion
watch(commandInput, async (newValue) => {
  // Reset AI completions when typing
  if (showAiCompletions.value) {
    showAiCompletions.value = false;
    aiCompletions.value = [];
  }

  if (!newValue) {
    traditionalCompletions.value = [];
    previewText.value = '';
    return;
  }

  const words = newValue.split(' ');
  let lastWord = words[words.length - 1];

  // Determine if we are completing a command or a file
  // Simple heuristic: if it's the first word, it's a command.
  // If it's not the first word, it's likely a file/argument.
  // Exception: pipe |, semicolon ;, etc. start new commands.

  // For simplicity, let's assume > 1 word is file completion.
  const isCommand = words.length === 1;

  try {
    let matches: string[] = [];

    if (isCommand) {
      // Command Completion
      // Use shell-specific logic, running in the currentDir context
      const cmd = getShellCompletionCommand(remoteShell.value, lastWord, currentDir.value);
      if (cmd) {
        const result = await invoke<string>('exec_command', {
          id: props.sessionId,
          command: cmd
        });
        matches = result.split('\n').filter(line => line.trim() !== '');
      }
    } else {
      // File Completion using SFTP (More robust)
      // We need to determine the directory to list.
      // If lastWord is "/var/lo", dir is "/var", prefix is "lo".
      // If lastWord is "lo", dir is currentDir, prefix is "lo".

      let searchDir = currentDir.value;
      let filePrefix = lastWord;

      if (lastWord.includes('/')) {
        const lastSlash = lastWord.lastIndexOf('/');
        const dirPart = lastWord.substring(0, lastSlash);
        filePrefix = lastWord.substring(lastSlash + 1);

        // Resolve dirPart relative to currentDir
        searchDir = resolvePath(currentDir.value, dirPart, homeDir.value);
      }

      try {
        const files = await invoke<FileEntry[]>('list_files', {
          id: props.sessionId,
          path: searchDir
        });

        matches = files
          .filter(f => f.name.startsWith(filePrefix))
          .map(f => {
            // If we are completing a path, we want to append the name to the dirPart
            // But the UI expects the full replacement for the last word?
            // Or just the completion?
            // The UI replaces `lastWord` with `completion`.
            // So if lastWord is "/var/lo", completion should be "/var/log".

            if (lastWord.includes('/')) {
              const lastSlash = lastWord.lastIndexOf('/');
              const dirPart = lastWord.substring(0, lastSlash);
              return `${dirPart}/${f.name}${f.is_dir ? '/' : ''}`;
            }
            return f.name + (f.is_dir ? '/' : '');
          });
      } catch (e) {
        // SFTP failed (maybe permission denied or path invalid), ignore
      }
    }

    // Filter matches again just in case (for command completion)
    if (isCommand) {
      matches = matches.filter(l => l.startsWith(lastWord));
    }

    // Sort matches
    matches.sort((a, b) => {
      if (a === lastWord) return -1;
      if (b === lastWord) return 1;
      if (a.length !== b.length) return a.length - b.length;
      return a.localeCompare(b);
    });

    traditionalCompletions.value = matches;
    selectedTraditionalIndex.value = 0;

    if (matches.length > 0) {
      updatePreview(matches[0], lastWord);
    } else {
      previewText.value = '';
    }
  } catch (e) {
    console.error("Completion error:", e);
    traditionalCompletions.value = [];
    previewText.value = '';
  }
});

function updatePreview(completion: string, lastWord: string) {
  // Completion is the full word, e.g. "lsattr"
  // lastWord is "ls"
  // We want to show "attr" as preview
  if (completion.startsWith(lastWord)) {
    previewText.value = completion.slice(lastWord.length);
  } else {
    previewText.value = '';
  }
}

async function sendCommand() {
  if (!commandInput.value.trim()) return;

  const cmd = commandInput.value;

  // Optimistically track CD
  handleCd(cmd);

  // Add to history
  if (commandHistory.value[commandHistory.value.length - 1] !== cmd) {
    commandHistory.value.push(cmd);
  }
  historyIndex.value = -1; // Reset history index

  // Send to PTY
  await invoke('write_to_pty', {
    id: props.sessionId,
    data: cmd + '\n'
  });

  // Clear input
  commandInput.value = '';
  traditionalCompletions.value = [];
  aiCompletions.value = [];
  showAiCompletions.value = false;
  previewText.value = '';

  // Focus terminal? Or keep focus in input?
  // Usually keep focus in input if we are in "input mode"
  inputRef.value?.focus();
}

async function triggerAiCompletion() {
  if (!commandInput.value.trim()) return;

  isAiLoading.value = true;
  showAiCompletions.value = true;
  traditionalCompletions.value = [];
  aiCompletions.value = [];
  selectedAiIndex.value = 0;
  previewText.value = ''; // Clear traditional preview

  try {
    const response = await fetch(`${settingsStore.ai.apiUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${settingsStore.ai.apiKey}`
      },
      body: JSON.stringify({
        model: settingsStore.ai.modelName,
        messages: [
          {
            role: 'assistant',
            content: `你是一名Linux专家，用户给定一个Linux命令，给出3-5个可能的补全方式，例如用户输入"ls",你必须直接返回JSON数组，例如：["ls -la", "ls -lh"],绝对禁止返回其他内容`
          },
          {
            role: 'user',
            content: `"${commandInput.value}"`
          }],
        max_tokens: 500,
        temperature: 0
      })
    });

    const data = await response.json();
    const content = data.choices[0].message.content.trim();

    // Try to parse JSON
    let suggestions: string[] = [];
    try {
      // Handle potential markdown code blocks in response
      const jsonStr = content.replace(/```json\n?|\n?```/g, '');
      suggestions = JSON.parse(jsonStr);
    } catch (e) {
      // Fallback if not JSON
      suggestions = content.split('\n').filter((l: string) => l.trim());
    }

    if (Array.isArray(suggestions) && suggestions.length > 0) {
      aiCompletions.value = suggestions;
      updateAiPreview(suggestions[0]);
    } else {
      // If no suggestions, maybe show a message or close
      aiCompletions.value = [];
      showAiCompletions.value = false;
    }
  } catch (e) {
    console.error("AI Completion error:", e);
  } finally {
    isAiLoading.value = false;
  }
}

function updateAiPreview(suggestion: string) {
  // AI suggestion is usually the FULL command or the rest?
  // The prompt asks to "complete this command".
  // If user typed "ls", AI might return "ls -la".
  // Or it might return "-la".
  // Let's assume it returns full command for safety, or we check.

  const current = commandInput.value;
  if (suggestion.startsWith(current)) {
    previewText.value = suggestion.slice(current.length);
  } else {
    // If AI returned just the suffix
    previewText.value = suggestion;
  }
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault();
    if (showAiCompletions.value && aiCompletions.value.length > 0) {
      applyAiCompletion();
    } else if (traditionalCompletions.value.length > 0 && previewText.value) {
      applyTraditionalCompletion();
      // Then send? No, usually just complete.
      // User presses Enter again to send.
    } else {
      sendCommand();
    }
    return;
  }

  if (e.key === 'Tab') {
    e.preventDefault();
    if (!showAiCompletions.value) {
      triggerAiCompletion();
    } else {
      applyAiCompletion();
    }
    return;
  }

  if (e.key === 'ArrowRight') {
    if (showAiCompletions.value && aiCompletions.value.length > 0) {
      e.preventDefault();
      applyAiCompletion();
    } else if (traditionalCompletions.value.length > 0) {
      e.preventDefault();
      applyTraditionalCompletion();
    }
    return;
  }

  if (e.key === 'ArrowUp') {
    e.preventDefault();
    if (showAiCompletions.value) {
      selectedAiIndex.value = Math.max(0, selectedAiIndex.value - 1);
      updateAiPreview(aiCompletions.value[selectedAiIndex.value]);
    } else if (traditionalCompletions.value.length > 0) {
      selectedTraditionalIndex.value = Math.max(0, selectedTraditionalIndex.value - 1);
      const words = commandInput.value.split(' ');
      const lastWord = words[words.length - 1];
      updatePreview(traditionalCompletions.value[selectedTraditionalIndex.value], lastWord);
    } else if (!commandInput.value || historyIndex.value !== -1) {
      // History navigation
      // If input is empty OR we are already navigating history
      if (historyIndex.value < commandHistory.value.length - 1) {
        historyIndex.value++;
        const idx = commandHistory.value.length - 1 - historyIndex.value;
        if (idx >= 0) {
          commandInput.value = commandHistory.value[idx];
          // Clear completions when navigating history
          traditionalCompletions.value = [];
          previewText.value = '';
        }
      }
    }
    return;
  }

  if (e.key === 'ArrowDown') {
    e.preventDefault();
    if (showAiCompletions.value) {
      selectedAiIndex.value = Math.min(aiCompletions.value.length - 1, selectedAiIndex.value + 1);
      updateAiPreview(aiCompletions.value[selectedAiIndex.value]);
    } else if (traditionalCompletions.value.length > 0) {
      selectedTraditionalIndex.value = Math.min(traditionalCompletions.value.length - 1, selectedTraditionalIndex.value + 1);
      const words = commandInput.value.split(' ');
      const lastWord = words[words.length - 1];
      updatePreview(traditionalCompletions.value[selectedTraditionalIndex.value], lastWord);
    } else if (!commandInput.value || historyIndex.value !== -1) {
      // History navigation
      if (historyIndex.value > -1) {
        historyIndex.value--;
        if (historyIndex.value === -1) {
          commandInput.value = '';
          traditionalCompletions.value = [];
          previewText.value = '';
        } else {
          const idx = commandHistory.value.length - 1 - historyIndex.value;
          commandInput.value = commandHistory.value[idx];
          traditionalCompletions.value = [];
          previewText.value = '';
        }
      }
    }
    return;
  }

  if (e.key === 'Escape') {
    showAiCompletions.value = false;
    traditionalCompletions.value = [];
    previewText.value = '';
  }
}

function applyTraditionalCompletion() {
  const completion = traditionalCompletions.value[selectedTraditionalIndex.value];
  if (!completion) return;

  const words = commandInput.value.split(' ');
  words.pop(); // Remove partial word
  words.push(completion); // Add completed word
  commandInput.value = words.join(' ') + ' '; // Add space?

  traditionalCompletions.value = [];
  previewText.value = '';
}

function applyAiCompletion() {
  const suggestion = aiCompletions.value[selectedAiIndex.value];
  if (!suggestion) return;

  // If suggestion is full command
  if (suggestion.startsWith(commandInput.value)) {
    commandInput.value = suggestion;
  } else {
    // Append
    commandInput.value += suggestion;
  }

  showAiCompletions.value = false;
  previewText.value = '';
}


function handleCd(cmd: string) {
  const trimmed = cmd.trim();
  if (trimmed.startsWith('cd ')) {
    const args = trimmed.split(/\s+/);
    if (args.length >= 2) {
      const target = args[1];
      currentDir.value = resolvePath(currentDir.value, target, homeDir.value);
    }
  } else if (trimmed === 'cd') {
    currentDir.value = homeDir.value;
  }
}

function resolvePath(current: string, target: string, home: string): string {
  if (target.startsWith('/')) return target;
  if (target === '~' || target.startsWith('~/')) {
    return target.replace('~', home);
  }

  // Handle .. and .
  const parts = current.split('/').filter(p => p);
  const targetParts = target.split('/').filter(p => p);

  for (const part of targetParts) {
    if (part === '.') continue;
    if (part === '..') {
      parts.pop();
    } else {
      parts.push(part);
    }
  }

  const result = '/' + parts.join('/');
  return result || '/';
}

function getShellCompletionCommand(shell: string, word: string, cwd: string): string {
  // We wrap in cd to ensure context
  const cdPrefix = `cd "${cwd}" && `;

  if (shell === 'bash' || shell === 'sh') {
    return `${cdPrefix}compgen -c ${word} | head -20`;
  } else if (shell === 'zsh') {
    // Zsh is complex, but we can try to use bash compgen if available, or just list binaries
    // Fallback to bash compgen if possible
    return `${cdPrefix}bash -c "compgen -c ${word}" 2>/dev/null || echo ""`;
  } else {
    // Generic fallback
    return `${cdPrefix}compgen -c ${word} 2>/dev/null || echo ""`;
  }
}

</script>

<template>
  <div class="h-full w-full flex flex-col bg-black overflow-hidden">
    <!-- Toolbar -->
    <div class="h-8 bg-gray-800 border-b border-gray-700 flex items-center px-2 space-x-2 flex-shrink-0">
      <button v-if="currentSession && currentSession.status === 'disconnected'" @click="handleReconnect"
        class="flex items-center px-2 py-1 text-xs text-green-400 hover:bg-gray-700 rounded transition-colors"
        title="Reconnect">
        <RotateCw class="w-3 h-3 mr-1" />
        Reconnect
      </button>

      <button v-if="currentSession && currentSession.status === 'connected'" @click="handleDisconnect"
        class="flex items-center px-2 py-1 text-xs text-red-400 hover:bg-gray-700 rounded transition-colors"
        title="Disconnect">
        <Unplug class="w-3 h-3 mr-1" />
        Disconnect
      </button>

      <div class="flex-1"></div>

      <button @click="handleClearLog"
        class="flex items-center px-2 py-1 text-xs text-gray-400 hover:text-white hover:bg-gray-700 rounded transition-colors"
        title="Clear Log">
        <Eraser class="w-3 h-3 mr-1" />
        Clear Log
      </button>
    </div>

    <!-- Terminal Area -->
    <div class="flex-1 relative overflow-hidden p-1">
      <div ref="terminalContainer" class="h-full w-full"></div>

      <!-- Search Bar -->
      <div v-if="showSearch"
        class="absolute top-2 right-4 z-10 flex items-center bg-gray-800 border border-gray-600 rounded shadow-lg p-1 animate-fade-in">
        <div class="flex items-center bg-gray-900 rounded border border-gray-700 mr-1">
          <Search class="w-3 h-3 text-gray-500 ml-2" />
          <input ref="searchInputRef" v-model="searchText" type="text"
            class="bg-transparent text-white text-xs px-2 py-1 focus:outline-none w-40 font-mono" placeholder="Find..."
            @keydown.enter="searchNext" @keydown.esc="closeSearch" />
        </div>
        <div class="flex items-center border-l border-gray-700 pl-1">
          <button @click="searchPrev" class="p-1 text-gray-400 hover:text-white hover:bg-gray-700 rounded"
            title="Previous (Shift+Enter)">
            <ArrowUp class="w-4 h-4" />
          </button>
          <button @click="searchNext" class="p-1 text-gray-400 hover:text-white hover:bg-gray-700 rounded"
            title="Next (Enter)">
            <ArrowDown class="w-4 h-4" />
          </button>
          <button @click="closeSearch" class="p-1 text-gray-400 hover:text-white hover:bg-gray-700 rounded ml-1"
            title="Close (Esc)">
            <X class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- Input Area -->
    <div class="bg-gray-800 border-t border-gray-700 p-2 relative">

      <!-- Completions Popup -->
      <div v-if="traditionalCompletions.length > 0 && !showAiCompletions"
        class="absolute bottom-full left-0 mb-1 bg-gray-800 border border-gray-600 rounded shadow-lg max-h-40 overflow-y-auto min-w-[200px] z-20">
        <div v-for="(item, index) in traditionalCompletions" :key="item"
          class="px-3 py-1 text-sm cursor-pointer flex items-center"
          :class="index === selectedTraditionalIndex ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-gray-700'"
          @click="selectedTraditionalIndex = index; applyTraditionalCompletion()">
          <TerminalIcon class="w-3 h-3 mr-2 opacity-50" />
          {{ item }}
        </div>
      </div>

      <!-- AI Completions Popup -->
      <div v-if="showAiCompletions"
        class="absolute bottom-full left-0 mb-1 bg-gray-800 border border-purple-500 rounded shadow-lg max-h-60 overflow-y-auto min-w-[300px] z-20">
        <div v-if="isAiLoading" class="p-2 text-gray-400 text-sm flex items-center">
          <Sparkles class="w-3 h-3 mr-2 animate-pulse" />
          AI Thinking...
        </div>
        <div v-else v-for="(item, index) in aiCompletions" :key="index"
          class="px-3 py-2 text-sm cursor-pointer border-b border-gray-700 last:border-0"
          :class="index === selectedAiIndex ? 'bg-purple-900/50 text-white' : 'text-gray-300 hover:bg-gray-700'"
          @click="selectedAiIndex = index; applyAiCompletion()">
          <div class="flex items-center justify-between w-full">
            <div class="flex items-center">
              <Sparkles class="w-3 h-3 mr-2 text-purple-400" />
              <span>{{ item }}</span>
            </div>
            <span
              class="text-[10px] text-purple-400 bg-purple-900/30 px-1 rounded border border-purple-500/30">AI推荐</span>
          </div>
        </div>
      </div>

      <!-- Input Box -->
      <div class="relative flex items-center">
        <div class="absolute left-3 text-green-500 font-mono text-sm select-none">➜</div>

        <!-- Input with Preview -->
        <div class="relative flex-1">
          <!-- Invisible text to size the preview? No, we overlay preview on top or right? -->
          <!-- Easier: Input background transparent, preview behind it? -->
          <!-- Or: Input text color normal, preview text appended? -->
          <!-- We can use a span for the input part and a span for the preview part in a container, 
                     and a transparent input on top. -->

          <div
            class="absolute inset-0 flex items-center pl-8 pr-12 pointer-events-none overflow-hidden whitespace-pre font-mono text-sm">
            <span class="text-transparent">{{ commandInput }}</span>
            <span class="text-gray-500">{{ previewText }}</span>
          </div>

          <input ref="inputRef" v-model="commandInput" type="text"
            class="w-full bg-gray-900/50 border border-gray-600 rounded pl-8 pr-12 py-2 text-sm text-white font-mono focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
            placeholder="Type a command..." @keydown="handleKeyDown" spellcheck="false" autocomplete="off" />
        </div>

        <button @click="sendCommand" class="absolute right-2 p-1 text-gray-400 hover:text-white transition-colors"
          title="Send Command">
          <Send class="w-4 h-4" />
        </button>
      </div>

      <!-- Helper Text -->
      <div class="mt-1 flex justify-between text-[10px] text-gray-500 px-1">
        <span>Tab: AI Completion</span>
        <span>↑/↓: History/Select</span>
      </div>
    </div>
  </div>
</template>
