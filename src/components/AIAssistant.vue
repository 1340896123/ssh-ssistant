<script setup lang="ts">
import { ref, nextTick, computed } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useSessionStore } from '../stores/sessions';
import { invoke } from '@tauri-apps/api/core';
import { Send, Bot, User, TerminalSquare, Loader2, ChevronRight, ChevronDown } from 'lucide-vue-next';
import MarkdownIt from 'markdown-it';

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true
});

const settingsStore = useSettingsStore();
const sessionStore = useSessionStore();

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

const messages = ref<Message[]>([
  { role: 'assistant', content: 'Hello! I am your SSH AI Assistant. I can help you execute commands and manage your server. How can I help you today?' }
]);
const input = ref('');
const isLoading = ref(false);
const messagesContainer = ref<HTMLElement | null>(null);
const toolStates = ref<Record<string, boolean>>({});

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
        } catch (e) {}
        
        return {
          id: tc.id,
          name: tc.function.name,
          args: tc.function.arguments,
          command,
          output: toolOutputMap.get(tc.id)
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
  }
];

async function sendMessage() {
  if (!input.value.trim() || isLoading.value) return;

  const userMsg = input.value.trim();
  input.value = '';
  messages.value.push({ role: 'user', content: userMsg });
  scrollToBottom();

  await processChat();
}

async function processChat() {
  isLoading.value = true;
  
  // Prepare payload
  // Filter messages for API (exclude some internal UI state if any, currently 1:1)
  // Note: OpenAI expects 'tool_calls' in assistant message if present, handled by logic below?
  // For simplicity, we reconstruct the conversation for the API.
  // However, keeping the exact structure is better.
  
  // We need to handle the conversation loop
  try {
    const response = await fetch(`${settingsStore.ai.apiUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${settingsStore.ai.apiKey}`
      },
      body: JSON.stringify({
        model: settingsStore.ai.modelName,
        messages: messages.value.map(m => ({
            role: m.role,
            content: m.content,
            tool_call_id: m.tool_call_id,
            name: m.name,
            // tool_calls needs to be added if this message had them. 
            // Currently our local Message struct is simplified. 
            // We'll improve local storage to handle tool_calls.
        })),
        tools: tools,
        tool_choice: "auto"
      })
    });

    if (!response.ok) {
      const errText = await response.text();
      throw new Error(`API Error: ${response.status} - ${errText}`);
    }

    const data = await response.json();
    const choice = data.choices[0];
    const message = choice.message;

    // Push assistant message
    // If it has tool_calls, we need to store them properly
    if (message.tool_calls) {
        messages.value.push({
            role: 'assistant',
            content: message.content || '',
            // We need to store tool_calls to send back in next turn
            // Extending our Message interface locally or just storing raw object?
            // Let's store raw extended properties in the ref array (it's flexible)
            ...message 
        });
        scrollToBottom();

        // Handle tool calls
        for (const toolCall of message.tool_calls) {
            if (toolCall.function.name === 'run_command') {
                const args = JSON.parse(toolCall.function.arguments);
                const cmd = args.command;
                
                // Feedback UI
                // messages.value.push({ role: 'system', content: `Executing: ${cmd}` }); 
                
                try {
                    const result = await invoke<string>('exec_command', { 
                        id: sessionStore.activeSessionId, 
                        command: cmd 
                    });
                    
                    messages.value.push({
                        role: 'tool',
                        tool_call_id: toolCall.id,
                        name: toolCall.function.name,
                        content: result || "(No output)"
                    });
                } catch (e) {
                    messages.value.push({
                        role: 'tool',
                        tool_call_id: toolCall.id,
                        name: toolCall.function.name,
                        content: `Error executing command: ${e}`
                    });
                }
            }
        }
        
        // Recursive call to process tool outputs
        await processChat();

    } else {
        messages.value.push({ role: 'assistant', content: message.content });
        scrollToBottom();
    }

  } catch (e) {
    console.error(e);
    messages.value.push({ role: 'assistant', content: `Error: ${e}` });
  } finally {
    isLoading.value = false;
    scrollToBottom();
  }
}

</script>

<template>
  <div class="flex flex-col h-full bg-gray-900 text-white">
    <!-- Messages Area -->
    <div ref="messagesContainer" class="flex-1 overflow-y-auto p-4 space-y-4">
      <div v-for="(msg, index) in displayMessages" :key="index" class="flex flex-col space-y-1">
        
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
          
          <div class="max-w-[85%] rounded-lg p-3 text-sm"
               :class="msg.role === 'user' ? 'bg-blue-700' : 'bg-gray-800'">
             
             <!-- Tool Call Display (Collapsible) -->
             <div v-if="msg.toolExecutions" class="mb-2 space-y-2">
                <div v-for="exec in msg.toolExecutions" :key="exec.id" class="bg-gray-900/50 rounded border border-gray-700 overflow-hidden">
                   <div @click="toggleTool(exec.id)" class="flex items-center p-2 cursor-pointer hover:bg-gray-800 text-xs transition-colors">
                      <component :is="toolStates[exec.id] ? ChevronDown : ChevronRight" class="w-4 h-4 text-gray-400 mr-1" />
                      <TerminalSquare class="w-3 h-3 mr-2 text-purple-400" />
                      <span class="font-mono flex-1 truncate text-gray-300">{{ exec.command }}</span>
                      <span v-if="!exec.output" class="flex items-center text-yellow-500 ml-2">
                        <Loader2 class="w-3 h-3 animate-spin mr-1" />
                        Running
                      </span>
                      <span v-else class="text-green-500 ml-2 text-[10px] uppercase">Done</span>
                   </div>
                   <div v-if="toolStates[exec.id] && exec.output" class="p-2 border-t border-gray-700 bg-black/30">
                      <pre class="text-xs text-gray-300 whitespace-pre-wrap overflow-x-auto font-mono">{{ exec.output }}</pre>
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
    <div class="p-4 bg-gray-800 border-t border-gray-700">
      <div class="relative flex items-center">
        <textarea 
            v-model="input" 
            @keydown.enter.exact.prevent="sendMessage"
            class="w-full bg-gray-900 border border-gray-700 rounded-lg pl-4 pr-12 py-3 text-sm text-white focus:outline-none focus:border-blue-500 resize-none"
            placeholder="Ask AI to help..."
            rows="1"
            :disabled="isLoading"
        ></textarea>
        <button 
            @click="sendMessage" 
            :disabled="isLoading || !input.trim()"
            class="absolute right-2 p-2 text-blue-500 hover:text-blue-400 disabled:opacity-50 disabled:cursor-not-allowed rounded-full hover:bg-gray-800"
        >
            <Send class="w-5 h-5" />
        </button>
      </div>
      <div class="mt-2 text-xs text-gray-500 text-center">
        AI can execute commands. Be careful.
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
  background-color: #111827; /* gray-900 */
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
  color: #e5e7eb; /* gray-200 */
}

:deep(.markdown-content ul), :deep(.markdown-content ol) {
  margin-left: 1.2em;
  margin-bottom: 0.5em;
  list-style-type: disc;
}

:deep(.markdown-content ol) {
  list-style-type: decimal;
}

:deep(.markdown-content a) {
  color: #60a5fa; /* blue-400 */
  text-decoration: underline;
}
</style>
