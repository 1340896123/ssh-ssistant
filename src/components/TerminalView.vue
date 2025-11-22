<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';

const props = defineProps<{ sessionId: string }>();
const terminalContainer = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let unlisten: (() => void) | null = null;

onMounted(async () => {
  if (!terminalContainer.value) return;

  term = new Terminal({
    cursorBlink: true,
    fontSize: 14,
    fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    theme: {
      background: '#000000',
    }
  });

  fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  term.open(terminalContainer.value);
  fitAddon.fit();

  // Listen to user input
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
    // Convert number[] back to Uint8Array
    const data = new Uint8Array(event.payload);
    term?.write(data);
  });
  
  // Also listen for exit
  const unlistenExit = await listen(`term-exit://${props.sessionId}`, () => {
    term?.write('\r\n[Process exited]\r\n');
  });
  
  // Add to cleanup
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

watch(() => props.sessionId, () => {
  // Handle session switch if component is reused
  // But usually Vue re-mounts component if key changes. 
  // Assuming key is handled by parent.
});

</script>

<template>
  <div class="h-full w-full bg-black p-1 overflow-hidden">
    <div ref="terminalContainer" class="h-full w-full"></div>
  </div>
</template>
