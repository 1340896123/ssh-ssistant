<script setup lang="ts">
import { CheckCircle, XCircle, AlertTriangle, Info, X } from 'lucide-vue-next';
import { onMounted, onUnmounted, ref } from 'vue';

export type NotificationType = 'success' | 'error' | 'warning' | 'info';

const props = defineProps<{
  type?: NotificationType;
  title?: string;
  message: string;
  duration?: number; // ms, default 1000
  show: boolean;
}>();

const emit = defineEmits(['close']);

const visible = ref(props.show);
let timer: ReturnType<typeof setTimeout> | null = null;

function close() {
  visible.value = false;
  emit('close');
}

onMounted(() => {
  if (props.duration !== 0) {
    timer = setTimeout(() => {
      close();
    }, props.duration || 1000);
  }
});

onUnmounted(() => {
  if (timer) clearTimeout(timer);
});

// Icon mapping
const icons = {
  success: CheckCircle,
  error: XCircle,
  warning: AlertTriangle,
  info: Info
};

const colors = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500'
};
</script>

<template>
  <Transition
    enter-active-class="transition ease-out duration-300"
    enter-from-class="opacity-0 scale-90"
    enter-to-class="opacity-100 scale-100"
    leave-active-class="transition ease-in duration-200"
    leave-from-class="opacity-100 scale-100"
    leave-to-class="opacity-0 scale-90"
  >
    <div v-if="visible" class="fixed inset-0 z-[100] flex items-center justify-center pointer-events-none">
      <!-- Backdrop (optional, maybe transparent) -->
      
      <!-- Modal Content -->
      <div class="bg-gray-800 border border-gray-700 shadow-2xl rounded-lg p-6 min-w-[300px] max-w-md pointer-events-auto flex flex-col items-center space-y-4">
        
        <!-- Icon -->
        <component :is="icons[type || 'info']" class="w-12 h-12" :class="colors[type || 'info']" />

        <!-- Text Content -->
        <div class="text-center space-y-1">
          <h3 v-if="title" class="text-lg font-medium text-white">{{ title }}</h3>
          <p class="text-gray-400 text-sm">{{ message }}</p>
        </div>

        <!-- Close Button (Optional for manual close) -->
        <!-- <button @click="close" class="absolute top-2 right-2 text-gray-500 hover:text-white">
          <X class="w-4 h-4" />
        </button> -->
      </div>
    </div>
  </Transition>
</template>
