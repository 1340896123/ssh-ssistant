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
  <div v-if="show" class="fixed inset-0 bg-bg-overlay flex items-center justify-center z-50">
    <div class="bg-bg-elevated p-6 rounded w-[500px] text-text-primary max-h-[90vh] overflow-y-auto border border-border-primary">
      <h2 class="text-xl mb-4 font-bold text-text-primary">{{ connectionToEdit ? 'Edit Connection' : 'New Connection' }}</h2>
      <div class="space-y-4">
        <div>
          <label class="block text-xs text-text-secondary uppercase mb-1">Name</label>
          <input v-model="form.name"
            class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
            placeholder="My Server" />
        </div>
        <div class="grid grid-cols-4 gap-4">
          <div class="col-span-3">
            <label class="block text-xs text-text-secondary uppercase mb-1">Host</label>
            <input v-model="form.host"
              class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
              placeholder="192.168.1.1" />
          </div>
          <div>
            <label class="block text-xs text-text-secondary uppercase mb-1">Port</label>
            <input v-model.number="form.port" type="number"
              class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
              placeholder="22" />
          </div>
        </div>
        <div>
          <label class="block text-xs text-text-secondary uppercase mb-1">Username</label>
          <input v-model="form.username"
            class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
            placeholder="root" />
        </div>

        <div>
          <label class="block text-xs text-text-secondary uppercase mb-1">Authentication Method</label>
          <div class="flex items-center justify-between">
            <div class="flex space-x-4">
              <label class="flex items-center space-x-2 cursor-pointer">
                <input type="radio" v-model="form.authType" value="password"
                  class="text-accent focus:ring-accent bg-bg-tertiary border-border-primary" />
                <span class="text-sm">Password</span>
              </label>
              <label class="flex items-center space-x-2 cursor-pointer">
                <input type="radio" v-model="form.authType" value="key"
                  class="text-accent focus:ring-accent bg-bg-tertiary border-border-primary" />
                <span class="text-sm">Private Key</span>
              </label>
            </div>
            <!-- Setup Key Auth Button -->
            <button v-if="connectionToEdit && connectionToEdit.id && form.authType === 'password'"
              @click="showInstallKeyModal = true" class="text-xs text-accent hover:text-accent/80 underline">
              Setup Key Auth
            </button>
          </div>
        </div>

        <!-- Install Key Modal/Overlay -->
        <div v-if="showInstallKeyModal" class="fixed inset-0 bg-bg-overlay/80 z-50 flex items-center justify-center">
          <div class="bg-bg-elevated p-6 rounded w-[400px] border border-border-primary">
            <h3 class="text-lg font-bold text-text-primary mb-4">Install SSH Key</h3>
            <p class="text-sm text-text-secondary mb-4">
              This will install the public key to the server's <code>authorized_keys</code> and switch the connection to
              use Key authentication.
            </p>

            <div class="mb-4">
              <label class="block text-xs text-text-secondary uppercase mb-1">Select Key to Install</label>
              <select v-model="keyToInstall"
                class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none">
                <option :value="null" disabled>Select a key</option>
                <option v-for="key in sshKeyStore.keys" :key="key.id" :value="key.id">
                  {{ key.name }}
                </option>
              </select>
            </div>

            <div v-if="installKeyResult" class="mb-4 text-sm"
              :class="installKeyResult.success ? 'text-success' : 'text-error'">
              {{ installKeyResult.message }}
            </div>

            <div class="flex justify-end gap-2">
              <button @click="showInstallKeyModal = false" :disabled="isInstallingKey"
                class="px-3 py-1.5 text-sm text-text-secondary hover:text-text-primary">Cancel</button>
              <button @click="installKey" :disabled="isInstallingKey || !keyToInstall"
                class="px-3 py-1.5 text-sm bg-accent hover:bg-accent/80 text-text-primary rounded flex items-center gap-2">
                <Loader2 v-if="isInstallingKey" class="w-3 h-3 animate-spin" />
                Install & Switch
              </button>
            </div>
          </div>
        </div>

        <div v-if="form.authType === 'password'">
          <label class="block text-xs text-text-secondary uppercase mb-1">Password</label>
          <div class="relative">
            <input v-model="form.password" :type="showPassword ? 'text' : 'password'"
              class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none pr-10"
              placeholder="••••••" />
            <button @click="showPassword = !showPassword" class="absolute right-2 top-2 text-text-secondary hover:text-text-primary">
              <Eye v-if="!showPassword" class="w-5 h-5" />
              <EyeOff v-else class="w-5 h-5" />
            </button>
          </div>
        </div>

        <div v-else>
          <label class="block text-xs text-text-secondary uppercase mb-1">SSH Key</label>
          <select v-model="form.sshKeyId"
            class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none">
            <option :value="null" disabled>Select a key</option>
            <option v-for="key in sshKeyStore.keys" :key="key.id" :value="key.id">
              {{ key.name }}
            </option>
          </select>
          <div v-if="sshKeyStore.keys.length === 0" class="text-xs text-warning mt-1">
            No keys found. Please add a key in Settings > SSH Keys.
          </div>
        </div>

        <div>
          <label class="block text-xs text-text-secondary uppercase mb-1">Operating System</label>
          <select v-model="form.osType"
            class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none">
            <option value="Linux">Linux</option>
            <option value="Windows">Windows</option>
            <option value="macOS">macOS</option>
          </select>
        </div>

        <div>
          <label class="block text-xs text-text-secondary uppercase mb-1">Group</label>
          <select v-model="form.groupId"
            class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none">
            <option :value="null">None</option>
            <option v-for="group in connectionStore.groups" :key="group.id" :value="group.id">
              {{ group.name }}
            </option>
          </select>
        </div>

        <!-- Proxy Jump Section -->
        <div class="border-t border-border-primary pt-4 mt-2">
          <h3 class="text-sm font-semibold text-text-primary mb-2">Proxy Jump (Optional)</h3>
          <div class="space-y-3 pl-2 border-l-2 border-border-primary">
            <div class="grid grid-cols-4 gap-4">
              <div class="col-span-3">
                <label class="block text-xs text-text-tertiary uppercase mb-1">Jump Host</label>
                <input v-model="form.jumpHost"
                  class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                  placeholder="jump.example.com" />
              </div>
              <div>
                <label class="block text-xs text-text-tertiary uppercase mb-1">Port</label>
                <input v-model.number="form.jumpPort" type="number"
                  class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                  placeholder="22" />
              </div>
            </div>
            <div>
              <label class="block text-xs text-gray-500 uppercase mb-1">Jump Username</label>
              <input v-model="form.jumpUsername"
                class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none"
                placeholder="jumpuser" />
            </div>
            <div>
              <label class="block text-xs text-gray-500 uppercase mb-1">Jump Password</label>
              <div class="relative">
                <input v-model="form.jumpPassword" :type="showJumpPassword ? 'text' : 'password'"
                  class="w-full p-2 bg-bg-tertiary text-text-primary rounded border border-border-primary focus:border-accent outline-none pr-10"
                  placeholder="••••••" />
                <button @click="showJumpPassword = !showJumpPassword"
                  class="absolute right-2 top-2 text-text-secondary hover:text-text-primary">
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
        :class="testResult.success ? 'bg-success/20 text-success' : 'bg-error/20 text-error'">
        <CheckCircle v-if="testResult.success" class="w-4 h-4" />
        <XCircle v-else class="w-4 h-4" />
        <span>{{ testResult.message }}</span>
      </div>

      <div class="mt-6 flex justify-between items-center">
        <button @click="testConnection" :disabled="isTesting"
          class="px-4 py-2 bg-warning hover:bg-warning/80 text-text-primary rounded cursor-pointer text-sm flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed">
          <Loader2 v-if="isTesting" class="w-4 h-4 animate-spin" />
          <span>Test Connection</span>
        </button>

        <div class="flex space-x-2">
          <button @click="$emit('close')"
            class="px-4 py-2 bg-bg-tertiary hover:bg-bg-elevated text-text-primary rounded cursor-pointer text-sm">Cancel</button>
          <button @click="save"
            class="px-4 py-2 bg-accent hover:bg-accent/80 text-text-primary rounded cursor-pointer text-sm">Save</button>
        </div>
      </div>
    </div>
  </div>
</template>
