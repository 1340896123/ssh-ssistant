<script setup lang="ts">
import { ref, nextTick, computed, onMounted, onUnmounted } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useSessionStore } from '../stores/sessions';
import { invoke } from '@tauri-apps/api/core';
import { confirm } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { Send, Bot, User, TerminalSquare, Loader2, ChevronRight, ChevronDown, ClipboardPlus, Trash2, Square, Briefcase } from 'lucide-vue-next';
import MarkdownIt from 'markdown-it';

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true
});

const props = defineProps({
  sessionId: String,
  terminalContext: String
});

const emit = defineEmits(['refresh-context']);

const settingsStore = useSettingsStore();
const sessionStore = useSessionStore();

const activeWorkspace = computed(() => {
  const session = sessionStore.sessions.find(s => s.id === props.sessionId);
  return session?.activeWorkspace;
});

function renderMarkdown(content: string) {
  if (!content) return '';
  return md.render(content);
}

interface Message {
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  tool_call_id?: string;
  name?: string;
  tool_calls?: any[];
}

interface ContextPath {
  path: string;
  isDir: boolean;
}

const contextPaths = ref<ContextPath[]>([]);
const input = ref('');
const isLoading = ref(false);
const isDragOverInput = ref(false);
const messagesContainer = ref<HTMLElement | null>(null);
const toolStates = ref<Record<string, boolean>>({}); // Keep toolStates as an empty object by default
const toolRealTimeOutputs = ref<Record<string, string[]>>({});
let abortController = ref<AbortController | null>(null);

const initialMessage: Message = { role: 'assistant', content: 'Hello! I am your SSH AI Assistant. I can help you execute commands and manage your server. How can I help you today?' };

const messages = ref<Message[]>([
  { ...initialMessage }
]);

function clearSession() {
  messages.value = [{ ...initialMessage }];
  contextPaths.value = [];
  toolStates.value = {};
  toolRealTimeOutputs.value = {};
  scrollToBottom();
}

// --- Command Safety ---
const DANGEROUS_COMMANDS = [
  /^\s*rm\s+.*(-f|--force)\s+.*\//, //  rm with force flag on root dir
  new RegExp("^\\s*rm\\s+-rf\\s+\\/\\*?\\s*$"), // rm -rf / or rm -rf /*
  /^\s*dd\s+/,
  /^\s*mkfs\./,
  /^\s*fdisk\s+/,
  /^\s*wipefs\s+/,
  /^\s*mv\s+[^\s]+\s+\/dev\/null/, // mv to /dev/null
  />\s*\/dev\/sd/, // Redirecting output to a disk device
];
function isDangerous(command: string): boolean {
  return DANGEROUS_COMMANDS.some(regex => regex.test(command));
}

function toggleTool(id: string) {
  toolStates.value[id] = !toolStates.value[id];
}

const displayMessages = computed(() => {
  const result: any[] = [];
  const toolOutputMap = new Map<string, string>();

  // First pass: gather all tool outputs
  for (const msg of messages.value) {
    if (msg.role === 'tool' && msg.tool_call_id) {
      toolOutputMap.set(msg.tool_call_id, msg.content);
    }
  }

  // Second pass: build display list
  for (const msg of messages.value) {
    if (msg.role === 'tool') continue; // Skip tool messages as they are attached to assistant

    if (msg.role === 'assistant' && msg.tool_calls) {
      const toolExecutions = msg.tool_calls.map((tc: any) => {
        let command = 'Unknown command';
        try {
          command = JSON.parse(tc.function.arguments).command;
        } catch (e) { }

        return {
          id: tc.id,
          name: tc.function.name,
          args: tc.function.arguments,
          command,
          output: toolOutputMap.get(tc.id),
          isRunning: !toolOutputMap.has(tc.id),
          realTimeOutput: toolRealTimeOutputs.value[tc.id] || []
        };
      });

      result.push({
        ...msg,
        toolExecutions
      });
    } else {
      result.push(msg);
    }
  }
  return result;
});

async function scrollToBottom() {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  }
}

const tools = [
  {
    type: "function",
    function: {
      name: "run_command",
      description: "Execute a shell command on the remote server. Use this to inspect files, check status, or perform actions.",
      parameters: {
        type: "object",
        properties: {
          command: {
            type: "string",
            description: "The shell command to execute"
          }
        },
        required: ["command"]
      }
    }
  },
  {
    type: "function",
    function: {
      name: "read_file",
      description: "Read the content of a remote file path that the user has shared or that you know.",
      parameters: {
        type: "object",
        properties: {
          path: {
            type: "string",
            description: "Remote file path to read. Prefer using paths provided by the user via drag-and-drop."
          },
          maxBytes: {
            type: "number",
            description: "Soft limit for number of bytes to return to avoid extremely large responses.",
            default: 16384
          }
        },
        required: ["path"]
      }
    }
  },
  {
    type: "function",
    function: {
      name: "write_file",
      description: "Write text content to a remote file (overwrite or append). Use with caution.",
      parameters: {
        type: "object",
        properties: {
          path: {
            type: "string",
            description: "Remote file path to write to. Prefer paths explicitly provided by the user."
          },
          content: {
            type: "string",
            description: "Full text content to write (UTF-8)."
          },
          mode: {
            type: "string",
            enum: ["overwrite", "append"],
            description: "Whether to overwrite the file or append to the end.",
            default: "overwrite"
          }
        },
        required: ["path", "content"]
      }
    }
  },
  {
    type: "function",
    function: {
      name: "search_files",
      description: "Search for a text pattern under a given remote directory using grep -n.",
      parameters: {
        type: "object",
        properties: {
          root: {
            type: "string",
            description: "Directory path to search under."
          },
          pattern: {
            type: "string",
            description: "Text pattern to search for (passed to grep)."
          },
          maxResults: {
            type: "number",
            description: "Maximum number of matching lines to return.",
            default: 200
          }
        },
        required: ["root", "pattern"]
      }
    }
  }
];

async function sendMessage() {
  if (!input.value.trim() || isLoading.value) return;

  abortController.value = new AbortController();
  const userMsg = input.value.trim();
  input.value = '';
  messages.value.push({ role: 'user', content: userMsg });
  scrollToBottom();

  await processChat();
}

function stopMessage() {
  if (abortController.value) {
    abortController.value.abort();
    
    // Cancel command execution on backend
    const runningCommandId = getCurrentRunningCommandId();
    if (runningCommandId) {
      invoke('cancel_command_execution', { commandId: runningCommandId }).catch(console.error);
    }
    
    isLoading.value = false;
    abortController.value = null;
    
    // Update running commands display to show they were stopped
    updateRunningCommandsStatus();
    
    messages.value.push({ role: 'assistant', content: `Request stopped by user.` });
    scrollToBottom();
  }
}

function getCurrentRunningCommandId(): string | null {
  // Find the most recent assistant message with tool calls
  for (let i = messages.value.length - 1; i >= 0; i--) {
    const msg = messages.value[i];
    if (msg.role === 'assistant' && msg.tool_calls) {
      for (const toolCall of msg.tool_calls) {
        // Check if this tool call has a corresponding tool output message
        const hasToolOutput = messages.value.some((toolMsg: any) => 
          toolMsg.role === 'tool' && toolMsg.tool_call_id === toolCall.id
        );
        
        if (!hasToolOutput) {
          return toolCall.id; // This tool call is still pending/running
        }
      }
    }
  }
  return null;
}

function updateRunningCommandsStatus() {
  // Find all messages with tool calls and update their status
  messages.value.forEach((msg: any) => {
    if (msg.tool_calls) {
      msg.tool_calls.forEach((tc: any) => {
        // Check if this tool call doesn't have a corresponding tool message yet
        const hasToolOutput = messages.value.some((toolMsg: any) => 
          toolMsg.role === 'tool' && toolMsg.tool_call_id === tc.id
        );
        
        if (!hasToolOutput) {
          // Add a tool message indicating the command was stopped
          messages.value.push({
            role: 'tool',
            tool_call_id: tc.id,
            name: tc.function.name,
            content: `Command execution stopped by user`
          });
        }
      });
    }
  });
}

async function processChat() {
  isLoading.value = true;

  // Clone messages for API to avoid mutating UI state, and inject context
  const apiMessages = JSON.parse(JSON.stringify(messages.value));

  // Inject System Prompt with Workspace Context
  let systemContent = "You are an intelligent SSH DevOps assistant. Use tools to manage the server.";

  // Add OS Context
  const currentSession = sessionStore.sessions.find(s => s.id === props.sessionId);
  if (currentSession?.os) {
    systemContent += `\n\nTARGET OS: ${currentSession.os}`;
    if (currentSession.os === 'Windows') {
      systemContent += "\nNOTE: The remote system is Windows. Use PowerShell or CMD syntax as appropriate.";
    }
  }

  if (activeWorkspace.value) {
    systemContent += `\n\n== CURRENT WORKSPACE ==
PATH: ${activeWorkspace.value.path}

INSTRUCTIONS:
1. All commands are executed in a fresh shell. 
2. I will automatically prepend 'cd ${activeWorkspace.value.path}' to your 'run_command' calls if they are relative.
3. Prioritize files shown in the PROJECT STRUCTURE below.

PROJECT STRUCTURE:
${activeWorkspace.value.fileTree}

KEY CONTEXT:
${activeWorkspace.value.context}
`;
  }

  // Prepend system message
  apiMessages.unshift({ role: 'system', content: systemContent });

  if (props.terminalContext && apiMessages.length > 0 && apiMessages[apiMessages.length - 1].role === 'user') {
    const lastMsg = apiMessages[apiMessages.length - 1];
    let contextText = lastMsg.content;
    if (contextPaths.value.length > 0) {
      const list = contextPaths.value.map((c) => `${c.path}${c.isDir ? '/' : ''}`).join('\n');
      contextText = `Here are the remote paths I am working with (from the file manager drag-and-drop):\n\n${list}\n\nMy request is: ${contextText}`;
    }
    lastMsg.content = `Here is the current terminal output for context:\n\n---\n${props.terminalContext}\n---\n\n${contextText}`;
  }


  try {
    const response = await fetch(`${settingsStore.ai.apiUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${settingsStore.ai.apiKey}`
      },
      body: JSON.stringify({
        model: settingsStore.ai.modelName,
        messages: apiMessages,
        tools: tools,
        tool_choice: "auto"
      }),
      signal: abortController.value?.signal
    });

    if (!response.ok) {
      const errText = await response.text();
      throw new Error(`API Error: ${response.status} - ${errText}`);
    }

    const data = await response.json();
    const choice = data.choices[0];
    const message = choice.message;

    if (message.tool_calls) {
      messages.value.push({
        role: 'assistant',
        content: message.content || '',
        ...message
      });
      scrollToBottom();

      // Handle tool calls
      for (const toolCall of message.tool_calls) {
        // Check if aborted before executing tool
        if (abortController.value?.signal.aborted) {
          throw new DOMException('Aborted', 'AbortError');
        }

        const name = toolCall.function.name;
        if (name === 'run_command') {
          const args = JSON.parse(toolCall.function.arguments);
          let cmd = args.command;

          // Auto-CD into workspace if active
          if (activeWorkspace.value && !cmd.trim().startsWith('cd ')) {
            // Use a safe way to cd, escape path quotes if needed (simplified here)
            // We assume path is safe-ish or we wrap in quotes
            const wsPath = activeWorkspace.value.path.replace(/'/g, "'\\''");
            cmd = `cd '${wsPath}' && ${cmd}`;
          }

          // --- DANGER ZONE ---
          if (isDangerous(cmd)) {
            const confirmed = await confirm(`DANGEROUS COMMAND DETECTED!\n\nAre you sure you want to execute:\n\n${cmd}`);
            if (!confirmed) {
              messages.value.push({
                role: 'tool',
                tool_call_id: toolCall.id,
                name: toolCall.function.name,
                content: `Command execution cancelled by user.`
              });
              continue; // Skip to next tool call
            }
          }

          try {
            // Initialize real-time output storage for this tool call
            toolRealTimeOutputs.value[toolCall.id] = [];
            
            // Start listening for real-time output events
            const unlisten = await listen(`command-output-${props.sessionId}-${toolCall.id}`, (event: any) => {
              const output = event.payload;
              if (output && output.data) {
                toolRealTimeOutputs.value[toolCall.id].push(output.data);
                // Force UI update to show new output
                scrollToBottom();
              }
            });
            
            let result = '';
            
            // Check if aborted before executing command
            if (abortController.value?.signal.aborted) {
              unlisten();
              throw new DOMException('Aborted', 'AbortError');
            }
            
            result = await invoke<string>('exec_command', {
              id: props.sessionId,
              command: cmd,
              toolCallId: toolCall.id
            });
            
            unlisten();
            
            // Check if aborted after command completed
            if (abortController.value?.signal.aborted) {
              throw new DOMException('Aborted', 'AbortError');
            }

            messages.value.push({
              role: 'tool',
              tool_call_id: toolCall.id,
              name: toolCall.function.name,
              content: result || "(No output)"
            });
          } catch (e: any) {
            if (e.name === 'AbortError' || e.message?.includes('Aborted')) {
              messages.value.push({
                role: 'tool',
                tool_call_id: toolCall.id,
                name: toolCall.function.name,
                content: `Command execution stopped by user.`
              });
            } else {
              messages.value.push({
                role: 'tool',
                tool_call_id: toolCall.id,
                name: toolCall.function.name,
                content: `Error executing command: ${e}`
              });
            }
          }
        } else if (name === 'read_file') {
          const args = JSON.parse(toolCall.function.arguments) as { path: string; maxBytes?: number };
          try {
            const result = await invoke<string>('read_remote_file', {
              id: props.sessionId,
              path: args.path,
              maxBytes: args.maxBytes ?? 16384,
            });
            messages.value.push({
              role: 'tool',
              tool_call_id: toolCall.id,
              name,
              content: result || '(Empty file or no data)'
            });
          } catch (e) {
            messages.value.push({
              role: 'tool',
              tool_call_id: toolCall.id,
              name,
              content: `Error reading file: ${e}`
            });
          }
        } else if (name === 'write_file') {
          const args = JSON.parse(toolCall.function.arguments) as { path: string; content: string; mode?: 'overwrite' | 'append' };
          try {
            await invoke('write_remote_file', {
              id: props.sessionId,
              path: args.path,
              content: args.content,
              mode: args.mode ?? 'overwrite',
            });
            messages.value.push({
              role: 'tool',
              tool_call_id: toolCall.id,
              name,
              content: `Write to ${args.path} completed (mode=${args.mode ?? 'overwrite'}).`
            });
          } catch (e) {
            messages.value.push({
              role: 'tool',
              tool_call_id: toolCall.id,
              name,
              content: `Error writing file: ${e}`
            });
          }
        } else if (name === 'search_files') {
          const args = JSON.parse(toolCall.function.arguments) as { root: string; pattern: string; maxResults?: number };
          try {
            const result = await invoke<string>('search_remote_files', {
              id: props.sessionId,
              root: args.root,
              pattern: args.pattern,
              maxResults: args.maxResults ?? 200,
            });
            messages.value.push({
              role: 'tool',
              tool_call_id: toolCall.id,
              name,
              content: result || '(No matches)'
            });
          } catch (e) {
            messages.value.push({
              role: 'tool',
              tool_call_id: toolCall.id,
              name,
              content: `Error searching files: ${e}`
            });
          }
        }
      }

      // Check if aborted before recursive call
      if (abortController.value?.signal.aborted) {
        throw new DOMException('Aborted', 'AbortError');
      }

      // Recursive call to process tool outputs
      await processChat();

    } else {
      messages.value.push({ role: 'assistant', content: message.content });
      scrollToBottom();
    }

  } catch (e: any) {
    if (e.name === 'AbortError') {
      console.log('Fetch aborted by user');
    } else {
      console.error(e);
      messages.value.push({ role: 'assistant', content: `Error: ${e}` });
    }
  } finally {
    isLoading.value = false;
    abortController.value = null;
    scrollToBottom();
  }
}

const containerRef = ref<HTMLElement | null>(null);
let unlistenDrop: (() => void) | null = null;

onMounted(async () => {
  scrollToBottom();

  unlistenDrop = await listen('tauri://drag-drop', (event) => {
    const payload = event.payload as { paths: string[], position: { x: number, y: number } };
    if (containerRef.value) {
      const rect = containerRef.value.getBoundingClientRect();
      const x = payload.position.x;
      const y = payload.position.y;

      // Check if drop is within this component
      if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) {
        if (payload.paths && payload.paths.length > 0) {
          for (const path of payload.paths) {
            const exists = contextPaths.value.some(c => c.path === path);
            if (!exists) {
              contextPaths.value.push({ path, isDir: false });
            }
          }
        }
      }
    }
  });
});

function onInputDragOver(event: DragEvent) {
  event.preventDefault();
  isDragOverInput.value = true;
}
function onInputDragLeave(_: DragEvent) {
  isDragOverInput.value = false;
}

function onInputDrop(event: DragEvent) {
  event.preventDefault();
  isDragOverInput.value = false;
  const data = event.dataTransfer?.getData('application/json');
  if (data) {
    try {
      const item = JSON.parse(data);
      if (item.path) {
        const exists = contextPaths.value.some(c => c.path === item.path);
        if (!exists) {
          contextPaths.value.push(item);
        }
      }
    } catch (e) {
      console.error("Failed to parse drop data", e);
    }
  }
}

function removeContextPath(pathToRemove: string) {
  contextPaths.value = contextPaths.value.filter((c) => c.path !== pathToRemove);
}

function copyCommand(command: string) {
  navigator.clipboard.writeText(command).then(() => {
    // Optional: Show a brief success message
    console.log('Command copied to clipboard');
  }).catch(err => {
    console.error('Failed to copy command:', err);
  });
}

function rerunCommand(command: string) {
  input.value = command;
  // Auto-focus and send the command
  nextTick(() => {
    sendMessage();
  });
}

onUnmounted(() => {
  if (unlistenDrop) {
    unlistenDrop();
  }
});
</script>

<template>
  <div class="flex flex-col h-full bg-gray-900 text-white" ref="containerRef">
    <!-- Header -->
    <div class="flex flex-col bg-gray-800 border-b border-gray-700">
      <div class="flex items-center justify-between px-4 py-2">
        <div class="flex items-center space-x-2">
          <Bot class="w-5 h-5 text-purple-400" />
          <span class="font-medium">AI Assistant</span>
        </div>
        <div class="flex items-center space-x-1">
          <button @click="clearSession"
            class="text-gray-400 hover:text-red-400 transition-colors p-1 rounded hover:bg-gray-700"
            title="Clear Session">
            <Trash2 class="w-4 h-4" />
          </button>
        </div>
      </div>
      <!-- Workspace Status Bar -->
      <div v-if="activeWorkspace"
        class="px-4 py-1 bg-gray-900/50 border-t border-gray-700 flex items-center text-xs text-gray-400">
        <Briefcase class="w-3 h-3 mr-1.5 text-blue-400" />
        <span class="font-mono text-blue-300 mr-2">{{ activeWorkspace.name }}</span>
        <span class="truncate opacity-60">{{ activeWorkspace.path }}</span>
        <div class="flex-1"></div>
        <span v-if="activeWorkspace.isIndexed" class="text-green-500">Indexed</span>
        <span v-else class="text-yellow-500 flex items-center">
          <Loader2 class="w-3 h-3 animate-spin mr-1" /> Indexing
        </span>
      </div>
    </div>

    <!-- Messages Area -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4" ref="messagesContainer">
      <div v-for="(msg, index) in displayMessages" :key="index" class="space-y-1">

        <!-- System messages (Optional visibility) -->
        <div v-if="msg.role === 'system'" class="flex items-start space-x-2 text-gray-400 text-xs pl-8">
          <TerminalSquare class="w-3 h-3 mt-0.5" />
          <pre class="whitespace-pre-wrap bg-gray-800 p-1 rounded flex-1 overflow-x-auto">{{ msg.content }}</pre>
        </div>

        <!-- User/Assistant messages -->
        <div v-else class="flex space-x-3" :class="msg.role === 'user' ? 'flex-row-reverse space-x-reverse' : ''">
          <div class="w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0"
            :class="msg.role === 'user' ? 'bg-blue-600' : 'bg-purple-600'">
            <User v-if="msg.role === 'user'" class="w-5 h-5" />
            <Bot v-else class="w-5 h-5" />
          </div>

          <div class="max-w-[85%] rounded-lg p-3 text-sm" :class="msg.role === 'user' ? 'bg-blue-700' : 'bg-gray-800'">

            <!-- Tool Call Display (Collapsible) -->
            <div v-if="msg.toolExecutions" class="mb-2 space-y-2">
              <div v-for="exec in msg.toolExecutions" :key="exec.id"
                class="bg-gray-900/50 rounded border border-gray-700 overflow-hidden">
                <div @click="toggleTool(exec.id)"
                  class="flex items-center p-2 cursor-pointer hover:bg-gray-800 text-xs transition-colors">
                  <component :is="toolStates[exec.id] ? ChevronDown : ChevronRight"
                    class="w-4 h-4 text-gray-400 mr-1" />
                  <TerminalSquare class="w-3 h-3 mr-2 text-purple-400" />
                  <span class="font-mono flex-1 truncate text-gray-300">{{ exec.command }}</span>
                  
                  <!-- Status indicator -->
                  <span v-if="!exec.output" class="flex items-center text-yellow-500 ml-2">
                    <Loader2 class="w-3 h-3 animate-spin mr-1" />
                    Running
                  </span>
                  <span v-else-if="exec.output === 'Command execution stopped by user'" class="text-red-500 ml-2 text-[10px] uppercase">
                    Stopped
                  </span>
                  <span v-else class="text-green-500 ml-2 text-[10px] uppercase">Done</span>
                  
                  <!-- Action buttons -->
                  <div class="flex items-center gap-1 ml-2">
                    <!-- Copy command button -->
                    <button @click.stop="copyCommand(exec.command)"
                      class="p-1 text-gray-400 hover:text-blue-400 hover:bg-gray-700 rounded transition-colors"
                      title="Copy command">
                      <ClipboardPlus class="w-3 h-3" />
                    </button>
                    
                    <!-- Rerun button (only for completed commands) -->
                    <button v-if="exec.output" 
                      @click.stop="rerunCommand(exec.command)"
                      class="p-1 text-gray-400 hover:text-green-400 hover:bg-gray-700 rounded transition-colors"
                      title="Rerun command">
                      <Loader2 class="w-3 h-3" />
                    </button>
                    
                    <!-- Stop button (only for running commands) -->
                    <button v-if="!exec.output && isLoading" 
                      @click.stop="stopMessage()"
                      class="p-1 text-red-500 hover:text-red-400 hover:bg-gray-700 rounded transition-colors"
                      title="Stop command">
                      <Square class="w-3 h-3 fill-current" />
                    </button>
                  </div>
                </div>
                <div v-if="toolStates[exec.id]" class="p-2 border-t border-gray-700 bg-black/30 overflow-y-auto max-h-64">
                  <pre
                    class="text-xs text-gray-300 whitespace-pre-wrap overflow-x-auto font-mono">
                    <!-- Show real-time output if running, otherwise show final output -->
                    <template v-if="exec.isRunning && exec.realTimeOutput && exec.realTimeOutput.length > 0">
                      {{ exec.realTimeOutput.join('') }}
                    </template>
                    <template v-else-if="exec.output">
                      {{ exec.output }}
                    </template>
                    <template v-else-if="exec.isRunning">
                      <!-- Empty placeholder for running commands without output yet -->
                    </template>
                  </pre>
                </div>
              </div>
            </div>

            <div class="markdown-content font-sans" v-html="renderMarkdown(msg.content)"></div>
          </div>
        </div>
      </div>
      <div v-if="isLoading" class="flex items-center space-x-2 text-gray-500 text-sm pl-12">
        <Loader2 class="w-4 h-4 animate-spin" />
        <span>AI is thinking...</span>
      </div>
    </div>

    <!-- Input Area -->
    <div class="p-4 bg-gray-800 border-t border-gray-700" @dragover="onInputDragOver" @dragleave="onInputDragLeave"
      @drop="onInputDrop">
      <div class="w-full" :class="{ 'opacity-50 border-2 border-dashed border-blue-500 rounded-lg': isDragOverInput }">
        <div class="flex flex-col space-y-2">
          <!-- Context Chips -->
          <div v-if="contextPaths.length > 0" class="flex flex-wrap gap-2 px-1">
            <div v-for="c in contextPaths" :key="c.path"
              class="flex items-center bg-blue-900/50 border border-blue-700/50 rounded px-2 py-1 text-xs text-blue-200 max-w-full">
              <span class="truncate font-mono mr-2">{{ c.isDir ? '[DIR]' : '' }} {{ c.path }}</span>
              <button @click="removeContextPath(c.path)" class="text-blue-400 hover:text-red-400">
                &times;
              </button>
            </div>
          </div>

          <div class="relative flex items-center">
            <button @click="emit('refresh-context')"
              class="absolute left-3 top-1/2 -translate-y-1/2 p-2 text-gray-400 hover:text-white transition-colors"
              title="Import terminal context">
              <ClipboardPlus class="w-5 h-5" />
            </button>
            <textarea v-model="input" @keydown.enter.exact.prevent="sendMessage"
              class="w-full bg-gray-900 border border-gray-700 rounded-lg pl-12 pr-12 py-3 text-sm text-white focus:outline-none focus:border-blue-500 resize-none"
              placeholder="Ask AI to help..." rows="1" :disabled="isLoading"></textarea>
            <button @click="isLoading ? stopMessage() : sendMessage()" :disabled="!isLoading && !input.trim()"
              class="absolute right-2 top-1/2 -translate-y-1/2 p-2 text-blue-500 hover:text-blue-400 disabled:opacity-50 disabled:cursor-not-allowed rounded-full transition-colors"
              :class="{ 'hover:text-red-400 text-red-500': isLoading }" :title="isLoading ? 'Stop' : 'Send'">
              <Square v-if="isLoading" class="w-5 h-5 fill-current" />
              <Send v-else class="w-5 h-5" />
            </button>
          </div>
        </div>
      </div>
      <div class="mt-2 text-xs text-gray-500 text-center">
        AI can execute commands. Exercise caution.
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Styles for markdown content */
:deep(.markdown-content) {
  line-height: 1.5;
}

:deep(.markdown-content p) {
  margin-bottom: 0.5em;
}

:deep(.markdown-content p:last-child) {
  margin-bottom: 0;
}

:deep(.markdown-content pre) {
  background-color: #111827;
  /* gray-900 */
  padding: 0.5rem;
  border-radius: 0.375rem;
  overflow-x: auto;
  margin-top: 0.5em;
  margin-bottom: 0.5em;
}

:deep(.markdown-content code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  background-color: rgba(0, 0, 0, 0.3);
  padding: 0.1rem 0.3rem;
  border-radius: 0.25rem;
  font-size: 0.9em;
}

:deep(.markdown-content pre code) {
  background-color: transparent;
  padding: 0;
  font-size: 0.9em;
  color: #e5e7eb;
  /* gray-200 */
}

:deep(.markdown-content ul),
:deep(.markdown-content ol) {
  margin-left: 1.2em;
  margin-bottom: 0.5em;
  list-style-type: disc;
}

:deep(.markdown-content ol) {
  list-style-type: decimal;
}

:deep(.markdown-content a) {
  color: #60a5fa;
  /* blue-400 */
  text-decoration: underline;
}
</style>