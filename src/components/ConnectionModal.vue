<script setup lang="ts">
import { ref, watch } from 'vue';
import type { Connection } from '../types';
import { Eye, EyeOff, Loader2, CheckCircle, XCircle } from 'lucide-vue-next';
import { useConnectionStore } from '../stores/connections';
import { useSshKeyStore } from '../stores/sshKeys';

const props = defineProps<{ show: boolean, connectionToEdit?: Connection | null }>();
const emit = defineEmits(['close', 'save']);
const connectionStore = useConnectionStore();
const sshKeyStore = useSshKeyStore();

const form = ref<Connection>({
  name: '',
  host: '',
  port: 22,
  username: '',
  password: '',
  authType: 'password',
  sshKeyId: null,
  jumpHost: '',
  jumpPort: 22,
  jumpUsername: '',
  jumpPassword: '',
  osType: 'Linux'
});

const showPassword = ref(false);
const showJumpPassword = ref(false);
const isTesting = ref(false);
const testResult = ref<{ success: boolean; message: string } | null>(null);

// Install Key State
const showInstallKeyModal = ref(false);
const isInstallingKey = ref(false);
const keyToInstall = ref<number | null>(null);
const installKeyResult = ref<{ success: boolean; message: string } | null>(null);

watch(() => props.show, (newVal) => {
  if (newVal) {
    showPassword.value = false;
    showJumpPassword.value = false;
    isTesting.value = false;
    testResult.value = null;
    sshKeyStore.loadKeys(); // Load keys when modal opens
    if (props.connectionToEdit) {
      form.value = { ...props.connectionToEdit };
      // Ensure optional fields are handled if undefined
      if (!form.value.jumpPort) form.value.jumpPort = 22;
      // Provide default OS type for backward compatibility
      if (!form.value.osType) form.value.osType = 'Linux';
      if (!form.value.authType) form.value.authType = 'password';
    } else {
      // Reset for new connection
      form.value = {
        name: '',
        host: '',
        port: 22,
        username: '',
        password: '',
        authType: 'password',
        sshKeyId: null,
        jumpHost: '',
        jumpPort: 22,
        jumpUsername: '',
        jumpPassword: '',
        groupId: null,
        osType: 'Linux'
      };
    }
  }
});

async function testConnection() {
  if (!form.value.host || !form.value.username) {
    testResult.value = { success: false, message: 'Host and Username are required' };
    return;
  }

  if (form.value.authType === 'key' && !form.value.sshKeyId) {
    testResult.value = { success: false, message: 'SSH Key is required for key authentication' };
    return;
  }

  isTesting.value = true;
  testResult.value = null;

  const payload = { ...form.value };
  payload.port = parseInt(payload.port.toString(), 10);
  if (payload.jumpPort) {
    payload.jumpPort = parseInt(payload.jumpPort.toString(), 10);
  }
  // Ensure osType is provided for backward compatibility
  if (!payload.osType) {
    payload.osType = 'Linux';
  }
  // Clear jump fields if host is empty
  if (!payload.jumpHost) {
    delete payload.jumpHost;
    delete payload.jumpPort;
    delete payload.jumpUsername;
    delete payload.jumpPassword;
  }

  try {
    await connectionStore.testConnection(payload);
    testResult.value = { success: true, message: 'Connection successful!' };
  } catch (e: any) {
    testResult.value = { success: false, message: e.toString() };
  } finally {
    isTesting.value = false;
  }
}

async function installKey() {
  if (!keyToInstall.value || !props.connectionToEdit?.id) return;
  isInstallingKey.value = true;
  installKeyResult.value = null;

  try {
    await sshKeyStore.installKey(props.connectionToEdit.id, keyToInstall.value);
    installKeyResult.value = { success: true, message: 'Key installed successfully!' };

    // Switch auth type and update form
    form.value.authType = 'key';
    form.value.sshKeyId = keyToInstall.value;

    setTimeout(() => {
      showInstallKeyModal.value = false;
      installKeyResult.value = null;
    }, 1500);

  } catch (e: any) {
    installKeyResult.value = { success: false, message: e.toString() };
  } finally {
    isInstallingKey.value = false;
  }
}

function save() {
  const payload = { ...form.value };
  payload.port = parseInt(payload.port.toString(), 10);
  if (payload.jumpPort) {
    payload.jumpPort = parseInt(payload.jumpPort.toString(), 10);
  }
  // Ensure osType is provided for backward compatibility
  if (!payload.osType) {
    payload.osType = 'Linux';
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
          <label class="block text-xs text-gray-400 uppercase mb-1">Authentication Method</label>
          <div class="flex items-center justify-between">
            <div class="flex space-x-4">
              <label class="flex items-center space-x-2 cursor-pointer">
                <input type="radio" v-model="form.authType" value="password"
                  class="text-blue-600 focus:ring-blue-500 bg-gray-700 border-gray-600" />
                <span class="text-sm">Password</span>
              </label>
              <label class="flex items-center space-x-2 cursor-pointer">
                <input type="radio" v-model="form.authType" value="key"
                  class="text-blue-600 focus:ring-blue-500 bg-gray-700 border-gray-600" />
                <span class="text-sm">Private Key</span>
              </label>
            </div>
            <!-- Setup Key Auth Button -->
            <button v-if="connectionToEdit && connectionToEdit.id && form.authType === 'password'"
              @click="showInstallKeyModal = true" class="text-xs text-blue-400 hover:text-blue-300 underline">
              Setup Key Auth
            </button>
          </div>
        </div>

        <!-- Install Key Modal/Overlay -->
        <div v-if="showInstallKeyModal" class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center">
          <div class="bg-gray-800 p-6 rounded shadow-xl w-[400px] border border-gray-600">
            <h3 class="text-lg font-bold text-white mb-4">Install SSH Key</h3>
            <p class="text-sm text-gray-400 mb-4">
              This will install the public key to the server's <code>authorized_keys</code> and switch the connection to
              use Key authentication.
            </p>

            <div class="mb-4">
              <label class="block text-xs text-gray-400 uppercase mb-1">Select Key to Install</label>
              <select v-model="keyToInstall"
                class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none">
                <option :value="null" disabled>Select a key</option>
                <option v-for="key in sshKeyStore.keys" :key="key.id" :value="key.id">
                  {{ key.name }}
                </option>
              </select>
            </div>

            <div v-if="installKeyResult" class="mb-4 text-sm"
              :class="installKeyResult.success ? 'text-green-400' : 'text-red-400'">
              {{ installKeyResult.message }}
            </div>

            <div class="flex justify-end gap-2">
              <button @click="showInstallKeyModal = false" :disabled="isInstallingKey"
                class="px-3 py-1.5 text-sm text-gray-300 hover:text-white">Cancel</button>
              <button @click="installKey" :disabled="isInstallingKey || !keyToInstall"
                class="px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded flex items-center gap-2">
                <Loader2 v-if="isInstallingKey" class="w-3 h-3 animate-spin" />
                Install & Switch
              </button>
            </div>
          </div>
        </div>

        <div v-if="form.authType === 'password'">
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

        <div v-else>
          <label class="block text-xs text-gray-400 uppercase mb-1">SSH Key</label>
          <select v-model="form.sshKeyId"
            class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none">
            <option :value="null" disabled>Select a key</option>
            <option v-for="key in sshKeyStore.keys" :key="key.id" :value="key.id">
              {{ key.name }}
            </option>
          </select>
          <div v-if="sshKeyStore.keys.length === 0" class="text-xs text-yellow-500 mt-1">
            No keys found. Please add a key in Settings > SSH Keys.
          </div>
        </div>

        <div>
          <label class="block text-xs text-gray-400 uppercase mb-1">Operating System</label>
          <select v-model="form.osType"
            class="w-full p-2 bg-gray-700 text-white rounded border border-gray-600 focus:border-blue-500 outline-none">
            <option value="Linux">Linux</option>
            <option value="Windows">Windows</option>
            <option value="macOS">macOS</option>
          </select>
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

      <!-- Test Result Feedback -->
      <div v-if="testResult" class="mt-4 p-2 rounded text-sm flex items-center gap-2"
        :class="testResult.success ? 'bg-green-900/50 text-green-200' : 'bg-red-900/50 text-red-200'">
        <CheckCircle v-if="testResult.success" class="w-4 h-4" />
        <XCircle v-else class="w-4 h-4" />
        <span>{{ testResult.message }}</span>
      </div>

      <div class="mt-6 flex justify-between items-center">
        <button @click="testConnection" :disabled="isTesting"
          class="px-4 py-2 bg-yellow-600 hover:bg-yellow-500 text-white rounded cursor-pointer text-sm flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed">
          <Loader2 v-if="isTesting" class="w-4 h-4 animate-spin" />
          <span>Test Connection</span>
        </button>

        <div class="flex space-x-2">
          <button @click="$emit('close')"
            class="px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded cursor-pointer text-sm">Cancel</button>
          <button @click="save"
            class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded cursor-pointer text-sm">Save</button>
        </div>
      </div>
    </div>
  </div>
</template>
