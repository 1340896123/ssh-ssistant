<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';
import { Send, Sparkles, Terminal as TerminalIcon } from 'lucide-vue-next';
import { useSettingsStore } from '../stores/settings';

const props = defineProps<{ sessionId: string }>();
const terminalContainer = ref<HTMLElement | null>(null);
const settingsStore = useSettingsStore();

let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let unlisten: (() => void) | null = null;

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

onMounted(async () => {
  if (!terminalContainer.value) return;

  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    theme: {
      background: '#000000',
    },
    allowProposedApi: true
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(terminalContainer.value);
  fitAddon.fit();

  // Listen to user input (direct terminal interaction)
  term.onData((data) => {
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

  // Initial resize
  setTimeout(() => {
    handleResize();
  }, 100);

  // Listen to backend data
  unlisten = await listen<number[]>(`term-data://${props.sessionId}`, (event) => {
    const data = new Uint8Array(event.payload);
    term?.write(data);
  });

  const unlistenExit = await listen(`term-exit://${props.sessionId}`, () => {
    term?.write('\r\n[Process exited]\r\n');
  });

  const oldUnlisten = unlisten;
  unlisten = () => {
    oldUnlisten();
    unlistenExit();
  };
});

function handleResize() {
  fitAddon?.fit();
}

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  if (unlisten) unlisten();
  term?.dispose();
});

// --- Input Box Logic ---

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

  // If we are just navigating history, don't trigger completion immediately? 
  // Actually, standard behavior is to complete what's there.

  const words = newValue.split(' ');
  let lastWord = words[words.length - 1];

  // If last char is space, lastWord is empty. We might want to complete files in current dir.
  // But be careful not to be too annoying.
  // Let's try: if lastWord is empty, we try to complete files?
  // Or maybe only if explicit trigger? 
  // User said: "ls -l" (no space at end) -> lastWord is "-l".
  // "ls -l " (space at end) -> lastWord is "".

  let commandToRun = '';
  if (lastWord && lastWord.length > 0) {
    // Try command completion first
    commandToRun = `compgen -c ${lastWord} | head -10`;
  } else if (words.length > 1) {
    // If we have previous words, and last word is empty, maybe complete files?
    // e.g. "ls " -> complete files.
    commandToRun = `compgen -f | head -10`;
    lastWord = ''; // For matching
  }

  if (commandToRun) {
    try {
      let result = await invoke<string>('exec_command', {
        id: props.sessionId,
        command: commandToRun
      });

      if (showAiCompletions.value) return;

      let lines = result.split('\n').filter(line => line.trim() !== '');

      // If no command matches, and we were trying command completion, try file completion
      if (lines.length === 0 && lastWord && lastWord.length > 0) {
        result = await invoke<string>('exec_command', {
          id: props.sessionId,
          command: `compgen -f ${lastWord} | head -10`
        });
        
        if (showAiCompletions.value) return;

        lines = result.split('\n').filter(line => line.trim() !== '');
      }

      // Filter to only those starting with lastWord to be sure
      let matches = lines.filter(l => l.startsWith(lastWord));

      // Sort matches: exact match first, then shortest length, then alphabetical
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
  } else {
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

</script>

<template>
  <div class="h-full w-full flex flex-col bg-black overflow-hidden">
    <!-- Terminal Area -->
    <div class="flex-1 relative overflow-hidden p-1">
      <div ref="terminalContainer" class="h-full w-full"></div>
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
