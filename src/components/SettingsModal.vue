<script setup lang="ts">
import { ref, watch } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useI18n } from '../composables/useI18n';
import { X } from 'lucide-vue-next';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits(['close']);
const store = useSettingsStore();
const { t } = useI18n();

const form = ref({
  theme: store.theme,
  language: store.language,
  ai: { ...store.ai }
});

watch(() => props.show, (val) => {
  if (val) {
    form.value = {
      theme: store.theme,
      language: store.language,
      ai: { ...store.ai }
    };
  }
});

function save() {
  store.saveSettings(form.value);
  emit('close');
}
</script>

<template>
  <div v-if="show" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
    <div class="bg-gray-800 rounded-lg shadow-xl w-[500px] border border-gray-700 flex flex-col max-h-[80vh]">
      <div class="flex items-center justify-between p-4 border-b border-gray-700">
        <h2 class="text-lg font-semibold text-white">{{ t('settings.title') }}</h2>
        <button @click="$emit('close')" class="text-gray-400 hover:text-white">
          <X class="w-5 h-5" />
        </button>
      </div>

      <div class="p-6 space-y-6 overflow-y-auto">
        <!-- Appearance -->
        <section>
          <h3 class="text-sm font-medium text-gray-400 uppercase mb-3">{{ t('settings.appearance') }}</h3>
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.theme') }}</label>
              <select v-model="form.theme" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                <option value="dark">{{ t('themes.dark') }}</option>
                <option value="light">{{ t('themes.light') }}</option>
              </select>
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.language') }}</label>
              <select v-model="form.language" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none">
                <option value="en">{{ t('languages.en') }}</option>
                <option value="zh">{{ t('languages.zh') }}</option>
              </select>
            </div>
          </div>
        </section>

        <!-- AI Configuration -->
        <section>
          <h3 class="text-sm font-medium text-gray-400 uppercase mb-3">{{ t('settings.aiAssistant') }}</h3>
          <div class="space-y-4">
             <div>
              <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.apiUrl') }}</label>
              <input v-model="form.ai.apiUrl" type="text" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" placeholder="https://api.openai.com/v1" />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.apiKey') }}</label>
              <input v-model="form.ai.apiKey" type="password" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" placeholder="sk-..." />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-300 mb-1">{{ t('settings.modelName') }}</label>
              <input v-model="form.ai.modelName" type="text" class="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white focus:border-blue-500 outline-none" placeholder="gpt-3.5-turbo" />
            </div>
          </div>
        </section>
      </div>

      <div class="p-4 border-t border-gray-700 flex justify-end space-x-3">
        <button @click="$emit('close')" class="px-4 py-2 text-sm text-gray-300 hover:text-white hover:bg-gray-700 rounded">{{ t('settings.cancel') }}</button>
        <button @click="save" class="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-500 text-white rounded">{{ t('settings.saveChanges') }}</button>
      </div>
    </div>
  </div>
</template>
