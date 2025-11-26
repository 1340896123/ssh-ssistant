<script setup lang="ts">
import { ref, watch } from 'vue';
import type { Connection } from '../types';
import { Eye, EyeOff } from 'lucide-vue-next';
import { useConnectionStore } from '../stores/connections';

const props = defineProps<{ show: boolean, connectionToEdit?: Connection | null }>();
const emit = defineEmits(['close', 'save']);
const connectionStore = useConnectionStore();

const form = ref<Connection>({
  name: '',
  host: '',
  port: 22,
  username: '',
  password: '',
  jumpHost: '',
  jumpPort: 22,
  jumpUsername: '',
  jumpPassword: ''
});

const showPassword = ref(false);
const showJumpPassword = ref(false);

watch(() => props.show, (newVal) => {
  if (newVal) {
    showPassword.value = false;
    showJumpPassword.value = false;
    if (props.connectionToEdit) {
      form.value = { ...props.connectionToEdit };
      // Ensure optional fields are handled if undefined
      if (!form.value.jumpPort) form.value.jumpPort = 22;
    } else {
      // Reset for new connection
      form.value = {
        name: '',
        host: '',
        port: 22,
        username: '',
        password: '',
        jumpHost: '',
        jumpPort: 22,
        jumpUsername: '',
        jumpPassword: '',
        groupId: null
      };
    }
  }
});

function save() {
  const payload = { ...form.value };
  payload.port = parseInt(payload.port.toString(), 10);
  if (payload.jumpPort) {
    payload.jumpPort = parseInt(payload.jumpPort.toString(), 10);
  }
  // Clear jump fields if host is empty to avoid sending empty strings as Some("")
  if (!payload.jumpHost) {
    delete payload.jumpHost;
    delete payload.jumpPort;
    delete payload.jumpUsername;
    delete payload.jumpPassword;
  }
  emit('save', payload);
  // Reset handled by watch or next open
}
</script>

<template>
  <div v-if="show" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-gray-800 p-6 rounded shadow-lg w-[500px] text-white max-h-[90vh] overflow-y-auto">
      <h2 class="text-xl mb-4 font-bold">{{ connectionToEdit ? 'Edit Connection' : 'New Connection' }}</h2>
      <div class="space-y-4">
        <div>
          <label class="block text-xs text-gray-400 uppercase mb-1">Name</label>
          <input v-model="form.name"
            class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none"
            placeholder="My Server" />
        </div>
        <div class="grid grid-cols-4 gap-4">
          <div class="col-span-3">
            <label class="block text-xs text-gray-400 uppercase mb-1">Host</label>
            <input v-model="form.host"
              class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none"
              placeholder="192.168.1.1" />
          </div>
          <div>
            <label class="block text-xs text-gray-400 uppercase mb-1">Port</label>
            <input v-model.number="form.port" type="number"
              class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none"
              placeholder="22" />
          </div>
        </div>
        <div>
          <label class="block text-xs text-gray-400 uppercase mb-1">Username</label>
          <input v-model="form.username"
            class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none"
            placeholder="root" />
        </div>
        <div>
          <label class="block text-xs text-gray-400 uppercase mb-1">Password</label>
          <div class="relative">
            <input v-model="form.password" :type="showPassword ? 'text' : 'password'"
              class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none pr-10"
              placeholder="••••••" />
            <button @click="showPassword = !showPassword" class="absolute right-2 top-2 text-gray-400 hover:text-white">
              <Eye v-if="!showPassword" class="w-5 h-5" />
              <EyeOff v-else class="w-5 h-5" />
            </button>
          </div>
        </div>

        <div>
          <label class="block text-xs text-gray-400 uppercase mb-1">Group</label>
          <select v-model="form.groupId"
            class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none">
            <option :value="null">None</option>
            <option v-for="group in connectionStore.groups" :key="group.id" :value="group.id">
              {{ group.name }}
            </option>
          </select>
        </div>

        <!-- Proxy Jump Section -->
        <div class="border-t border-gray-700 pt-4 mt-2">
          <h3 class="text-sm font-semibold text-gray-300 mb-2">Proxy Jump (Optional)</h3>
          <div class="space-y-3 pl-2 border-l-2 border-gray-700">
            <div class="grid grid-cols-4 gap-4">
              <div class="col-span-3">
                <label class="block text-xs text-gray-500 uppercase mb-1">Jump Host</label>
                <input v-model="form.jumpHost"
                  class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none"
                  placeholder="jump.example.com" />
              </div>
              <div>
                <label class="block text-xs text-gray-500 uppercase mb-1">Port</label>
                <input v-model.number="form.jumpPort" type="number"
                  class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none"
                  placeholder="22" />
              </div>
            </div>
            <div>
              <label class="block text-xs text-gray-500 uppercase mb-1">Jump Username</label>
              <input v-model="form.jumpUsername"
                class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none"
                placeholder="jumpuser" />
            </div>
            <div>
              <label class="block text-xs text-gray-500 uppercase mb-1">Jump Password</label>
              <div class="relative">
                <input v-model="form.jumpPassword" :type="showJumpPassword ? 'text' : 'password'"
                  class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none pr-10"
                  placeholder="••••••" />
                <button @click="showJumpPassword = !showJumpPassword"
                  class="absolute right-2 top-2 text-gray-400 hover:text-white">
                  <Eye v-if="!showJumpPassword" class="w-5 h-5" />
                  <EyeOff v-else class="w-5 h-5" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="mt-6 flex justify-end space-x-2">
        <button @click="$emit('close')"
          class="px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded cursor-pointer text-sm">Cancel</button>
        <button @click="save"
          class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded cursor-pointer text-sm">Save</button>
      </div>
    </div>
  </div>
</template>
