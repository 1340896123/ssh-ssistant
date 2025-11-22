<script setup lang="ts">
import { ref } from 'vue';
import type { Connection } from '../types';

defineProps<{ show: boolean }>();
const emit = defineEmits(['close', 'save']);

const form = ref<Connection>({
  name: '',
  host: '',
  port: 22,
  username: '',
  password: ''
});

function save() {
  const payload = { ...form.value };
  payload.port = parseInt(payload.port.toString(), 10);
  emit('save', payload);
  form.value = { name: '', host: '', port: 22, username: '', password: '' };
}
</script>

<template>
  <div v-if="show" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-gray-800 p-6 rounded shadow-lg w-96 text-white">
      <h2 class="text-xl mb-4">New Connection</h2>
      <div class="space-y-4">
        <div>
          <label class="block text-sm text-gray-400">Name</label>
          <input v-model="form.name" class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600" />
        </div>
        <div>
          <label class="block text-sm text-gray-400">Host</label>
          <input v-model="form.host" class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600" />
        </div>
        <div>
          <label class="block text-sm text-gray-400">Port</label>
          <input v-model.number="form.port" type="number" class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600" />
        </div>
        <div>
          <label class="block text-sm text-gray-400">Username</label>
          <input v-model="form.username" class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600" />
        </div>
        <div>
          <label class="block text-sm text-gray-400">Password</label>
          <input v-model="form.password" type="password" class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600" />
        </div>
      </div>
      <div class="mt-6 flex justify-end space-x-2">
        <button @click="$emit('close')" class="px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded cursor-pointer">Cancel</button>
        <button @click="save" class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded cursor-pointer">Save</button>
      </div>
    </div>
  </div>
</template>
