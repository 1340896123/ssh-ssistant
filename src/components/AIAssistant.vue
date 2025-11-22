<script setup lang="ts">
import { ref, nextTick } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useSessionStore } from '../stores/sessions';
import { invoke } from '@tauri-apps/api/core';
import { Send, Bot, User, TerminalSquare, Loader2 } from 'lucide-vue-next';

const settingsStore = useSettingsStore();
const sessionStore = useSessionStore();

interface Message {
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  tool_call_id?: string;
  name?: string; 
}

const messages = ref<Message[]>([
  { role: 'assistant', content: 'Hello! I am your SSH AI Assistant. I can help you execute commands and manage your server. How can I help you today?' }
]);
const input = ref('');
const isLoading = ref(false);
const messagesContainer = ref<HTMLElement | null>(null);

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
      <div v-for="(msg, index) in messages" :key="index" class="flex flex-col space-y-1">
        
        <!-- System/Tool messages (Optional visibility) -->
        <div v-if="msg.role === 'tool'" class="flex items-start space-x-2 text-gray-400 text-xs pl-8">
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
             
             <!-- Tool Call Display -->
             <div v-if="(msg as any).tool_calls" class="mb-2 space-y-1">
                <div v-for="tc in (msg as any).tool_calls" :key="tc.id" class="text-xs text-gray-300 flex items-center bg-gray-900/50 p-1 rounded">
                    <TerminalSquare class="w-3 h-3 mr-1" />
                    <span class="font-mono">{{ JSON.parse(tc.function.arguments).command }}</span>
                </div>
             </div>

             <div class="whitespace-pre-wrap font-sans">{{ msg.content }}</div>
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
