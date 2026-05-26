<script setup lang="ts">
import { ref, watch } from "vue";
import type {
  AccessEndpoint,
  CredentialRef,
  CredentialKind,
  HostAsset,
} from "../types";
import { Eye, EyeOff, Loader2, CheckCircle, XCircle } from "lucide-vue-next";
import { useAssetStore } from "../stores/assets";
import { useSshKeyStore } from "../stores/sshKeys";
import { sessionService } from "../services";
import { useI18n } from "../composables/useI18n";

const props = defineProps<{
  show: boolean;
  assetToEdit?: HostAsset | null;
  endpointToEdit?: AccessEndpoint | null;
  credentialRefToEdit?: CredentialRef | null;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (
    e: "save",
    payload: {
      asset: HostAsset;
      endpoint: AccessEndpoint;
      credentialRef?: CredentialRef | null;
    },
  ): void;
}>();

const assetStore = useAssetStore();
const sshKeyStore = useSshKeyStore();
const { t } = useI18n();

const formAsset = ref<HostAsset>({
  name: "",
  host: "",
  port: 22,
  platform: "Linux",
  folderId: null,
  envId: null,
  labels: [],
  owner: "",
  criticality: "medium",
  defaultWorkspacePath: "",
  bastionChainId: "",
  accessEndpointId: null,
  healthSummary: "",
  isFavorite: false,
});

const formEndpoint = ref<AccessEndpoint>({
  assetId: 0,
  name: "",
  host: "",
  port: 22,
  username: "root",
  authType: "password",
  credentialRefId: null,
  sshKeyId: null,
  jumpHost: null,
  jumpPort: null,
  jumpUsername: null,
  jumpPassword: null,
});

const formCredentialRef = ref<CredentialRef | null>({
  id: undefined,
  name: "",
  credentialKind: "password",
  username: "root",
  secret: "",
  sshKeyId: null,
  assetId: null,
  createdAt: 0,
  updatedAt: 0,
});

const labelsInput = ref("");
const showPassword = ref(false);
const showJumpPassword = ref(false);
const isTesting = ref(false);
const testResult = ref<{ success: boolean; message: string } | null>(null);

function resetForms() {
  formAsset.value = {
    id: props.assetToEdit?.id,
    name: props.assetToEdit?.name ?? "",
    host: props.assetToEdit?.host ?? "",
    port: props.assetToEdit?.port ?? 22,
    platform: props.assetToEdit?.platform ?? "Linux",
    folderId: props.assetToEdit?.folderId ?? props.assetToEdit?.groupId ?? null,
    envId: props.assetToEdit?.envId ?? null,
    labels: props.assetToEdit?.labels ?? [],
    owner: props.assetToEdit?.owner ?? "",
    criticality: props.assetToEdit?.criticality ?? "medium",
    defaultWorkspacePath: props.assetToEdit?.defaultWorkspacePath ?? "",
    bastionChainId: props.assetToEdit?.bastionChainId ?? "",
    accessEndpointId: props.assetToEdit?.accessEndpointId ?? null,
    healthSummary: props.assetToEdit?.healthSummary ?? "",
    isFavorite: props.assetToEdit?.isFavorite ?? false,
  };

  formEndpoint.value = {
    id: props.endpointToEdit?.id,
    assetId: props.endpointToEdit?.assetId ?? props.assetToEdit?.id ?? 0,
    name:
      props.endpointToEdit?.name ??
      (props.assetToEdit?.name
        ? `${props.assetToEdit.name} default endpoint`
        : t("assetCenter.fields.defaultEndpoint")),
    host: props.endpointToEdit?.host ?? props.assetToEdit?.host ?? "",
    port: props.endpointToEdit?.port ?? props.assetToEdit?.port ?? 22,
    username:
      props.endpointToEdit?.username ??
      props.credentialRefToEdit?.username ??
      "root",
    authType:
      props.endpointToEdit?.authType ??
      (props.credentialRefToEdit?.credentialKind === "sshKey" ? "key" : "password"),
    credentialRefId: props.endpointToEdit?.credentialRefId ?? props.credentialRefToEdit?.id ?? null,
    sshKeyId: props.endpointToEdit?.sshKeyId ?? props.credentialRefToEdit?.sshKeyId ?? null,
    jumpHost: props.endpointToEdit?.jumpHost ?? null,
    jumpPort: props.endpointToEdit?.jumpPort ?? 22,
    jumpUsername: props.endpointToEdit?.jumpUsername ?? null,
    jumpPassword: null,
  };

  formCredentialRef.value = {
    id: props.credentialRefToEdit?.id,
    name:
      props.credentialRefToEdit?.name ??
      (props.assetToEdit?.name
        ? `${props.assetToEdit.name} credential`
        : `${t("connectionModal.editTitle")} credential`),
    credentialKind:
      props.credentialRefToEdit?.credentialKind ??
      (formEndpoint.value.authType === "key" ? "sshKey" : "password"),
    username:
      props.credentialRefToEdit?.username ??
      formEndpoint.value.username,
    secret:
      props.credentialRefToEdit?.credentialKind === "password"
        ? props.credentialRefToEdit?.secret ?? ""
        : "",
    sshKeyId:
      props.credentialRefToEdit?.sshKeyId ??
      formEndpoint.value.sshKeyId ??
      null,
    assetId: props.assetToEdit?.id ?? null,
    createdAt: props.credentialRefToEdit?.createdAt ?? 0,
    updatedAt: props.credentialRefToEdit?.updatedAt ?? 0,
  };

  labelsInput.value = (formAsset.value.labels ?? []).join(", ");
  showPassword.value = false;
  showJumpPassword.value = false;
  isTesting.value = false;
  testResult.value = null;
}

watch(
  () => props.show,
  async (show) => {
    if (!show) return;
    await Promise.all([assetStore.loadAssets(), sshKeyStore.loadKeys()]);
    resetForms();
  },
  { immediate: true },
);

watch(
  () => formEndpoint.value.authType,
  (authType) => {
    if (!formCredentialRef.value) return;
    formCredentialRef.value.credentialKind = authType === "key" ? "sshKey" : "password";
    if (authType === "password") {
      formEndpoint.value.sshKeyId = null;
      formCredentialRef.value.sshKeyId = null;
    } else {
      formCredentialRef.value.secret = null;
    }
  },
);

function buildPayload() {
  const asset: HostAsset = {
    ...formAsset.value,
    folderId: formAsset.value.folderId ?? null,
    groupId: formAsset.value.folderId ?? null,
    labels: labelsInput.value
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean),
    criticality: formAsset.value.criticality ?? "medium",
    platform: formAsset.value.platform ?? "Linux",
    owner: formAsset.value.owner?.trim() || "",
    defaultWorkspacePath: formAsset.value.defaultWorkspacePath?.trim() || "",
    bastionChainId: formAsset.value.bastionChainId?.trim() || "",
    healthSummary: formAsset.value.healthSummary?.trim() || "",
  };

  const endpoint: AccessEndpoint = {
    ...formEndpoint.value,
    assetId: asset.id ?? 0,
    host: formEndpoint.value.host.trim() || asset.host,
    port: Number(formEndpoint.value.port || asset.port || 22),
    username: formEndpoint.value.username.trim(),
    name:
      formEndpoint.value.name.trim() ||
      `${asset.name || t("connectionModal.newTitle")} ${t("assetCenter.fields.defaultEndpoint")}`,
    authType: formEndpoint.value.authType ?? "password",
    jumpHost: formEndpoint.value.jumpHost?.trim() || null,
    jumpPort: formEndpoint.value.jumpHost ? Number(formEndpoint.value.jumpPort || 22) : null,
    jumpUsername: formEndpoint.value.jumpUsername?.trim() || null,
    jumpPassword: formEndpoint.value.jumpHost
      ? (() => {
          const trimmed = formEndpoint.value.jumpPassword?.trim();
          if (trimmed) return trimmed;
          if (formEndpoint.value.id) return undefined;
          return null;
        })()
      : null,
  };

  const credentialRef =
    formCredentialRef.value && endpoint.authType
      ? {
          ...formCredentialRef.value,
          name:
            formCredentialRef.value.name.trim() ||
            `${asset.name || t("connectionModal.newTitle")} credential`,
          credentialKind: (endpoint.authType === "key" ? "sshKey" : "password") as CredentialKind,
          username: endpoint.username,
          secret:
            endpoint.authType === "password"
              ? formCredentialRef.value.secret?.trim() || ""
              : null,
          sshKeyId:
            endpoint.authType === "key"
              ? formCredentialRef.value.sshKeyId ?? endpoint.sshKeyId ?? null
              : null,
          assetId: asset.id ?? null,
          createdAt: formCredentialRef.value.createdAt || Date.now(),
          updatedAt: Date.now(),
        }
      : null;

  endpoint.credentialRefId = credentialRef?.id ?? endpoint.credentialRefId ?? null;
  endpoint.sshKeyId =
    endpoint.authType === "key"
      ? credentialRef?.sshKeyId ?? endpoint.sshKeyId ?? null
      : null;

  return { asset, endpoint, credentialRef };
}

async function testConnection() {
  const payload = buildPayload();
  if (!payload.asset.host.trim() || !payload.endpoint.username.trim()) {
    testResult.value = {
      success: false,
      message: t("connectionModal.testResult.hostRequired"),
    };
    return;
  }

  if (
    payload.endpoint.authType === "password" &&
    !payload.credentialRef?.secret
  ) {
    testResult.value = {
      success: false,
      message: t("connectionModal.testResult.passwordRequired"),
    };
    return;
  }

  if (
    payload.endpoint.authType === "key" &&
    !payload.credentialRef?.sshKeyId
  ) {
    testResult.value = {
      success: false,
      message: t("connectionModal.testResult.sshKeyRequired"),
    };
    return;
  }

  isTesting.value = true;
  testResult.value = null;

  try {
    await sessionService.testConnection({
      id: payload.asset.id,
      name: payload.asset.name,
      host: payload.endpoint.host,
      port: payload.endpoint.port,
      username: payload.endpoint.username,
      password:
        payload.endpoint.authType === "password"
          ? payload.credentialRef?.secret ?? undefined
          : undefined,
      authType: payload.endpoint.authType,
      sshKeyId: payload.credentialRef?.sshKeyId ?? payload.endpoint.sshKeyId ?? null,
      jumpHost: payload.endpoint.jumpHost ?? undefined,
      jumpPort: payload.endpoint.jumpPort ?? undefined,
      jumpUsername: payload.endpoint.jumpUsername ?? undefined,
      jumpPassword: payload.endpoint.jumpPassword ?? undefined,
      groupId: payload.asset.folderId ?? null,
      osType: payload.asset.platform ?? "Linux",
      platform: payload.asset.platform ?? "Linux",
    });
    testResult.value = { success: true, message: t("connectionModal.testResult.success") };
  } catch (error: any) {
    testResult.value = { success: false, message: error?.toString() ?? t("connectionModal.testResult.failed") };
  } finally {
    isTesting.value = false;
  }
}

function save() {
  const payload = buildPayload();
  emit("save", payload);
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 flex items-center justify-center bg-bg-overlay"
  >
    <div
      class="max-h-[90vh] w-[680px] overflow-y-auto rounded border border-border-primary bg-bg-elevated p-6 text-text-primary"
    >
      <h2 class="mb-4 text-xl font-bold text-text-primary">
        {{ assetToEdit ? t('connectionModal.editTitle') : t('connectionModal.newTitle') }}
      </h2>

      <div class="space-y-5">
        <section class="space-y-4">
          <div class="text-xs font-semibold uppercase tracking-wide text-text-secondary">
            {{ t('connectionModal.sections.assetInfo') }}
          </div>

          <div>
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.name') }}</label>
            <input
              v-model="formAsset.name"
              class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              :placeholder="t('connectionModal.placeholders.name')"
            />
          </div>

          <div class="grid grid-cols-4 gap-4">
            <div class="col-span-3">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.host') }}</label>
              <input
                v-model="formAsset.host"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.host')"
              />
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.port') }}</label>
              <input
                v-model.number="formAsset.port"
                type="number"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.port')"
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.platform') }}</label>
              <select
                v-model="formAsset.platform"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              >
                <option value="Linux">{{ t('connectionModal.platformOptions.linux') }}</option>
                <option value="Windows">{{ t('connectionModal.platformOptions.windows') }}</option>
                <option value="macOS">{{ t('connectionModal.platformOptions.macos') }}</option>
              </select>
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.criticality') }}</label>
              <select
                v-model="formAsset.criticality"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              >
                <option value="low">{{ t('connectionModal.criticalityOptions.low') }}</option>
                <option value="medium">{{ t('connectionModal.criticalityOptions.medium') }}</option>
                <option value="high">{{ t('connectionModal.criticalityOptions.high') }}</option>
                <option value="critical">{{ t('connectionModal.criticalityOptions.critical') }}</option>
              </select>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.folder') }}</label>
              <select
                v-model="formAsset.folderId"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              >
                <option :value="null">{{ t('connectionModal.noneOption') }}</option>
                <option v-for="folder in assetStore.folders" :key="folder.id" :value="folder.id">
                  {{ folder.name }}
                </option>
              </select>
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.environment') }}</label>
              <select
                v-model="formAsset.envId"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              >
                <option :value="null">{{ t('connectionModal.noneOption') }}</option>
                <option v-for="env in assetStore.environments" :key="env.id" :value="env.id">
                  {{ env.name }}
                </option>
              </select>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.owner') }}</label>
              <input
                v-model="formAsset.owner"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.owner')"
              />
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.labels') }}</label>
              <input
                v-model="labelsInput"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.labels')"
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.defaultWorkspace') }}</label>
              <input
                v-model="formAsset.defaultWorkspacePath"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.defaultWorkspace')"
              />
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.bastionChainId') }}</label>
              <input
                v-model="formAsset.bastionChainId"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.bastionChainId')"
              />
            </div>
          </div>

          <div>
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.healthSummary') }}</label>
            <input
              v-model="formAsset.healthSummary"
              class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              :placeholder="t('connectionModal.placeholders.healthSummary')"
            />
          </div>
        </section>

        <section class="space-y-4 border-t border-border-primary pt-4">
          <div class="text-xs font-semibold uppercase tracking-wide text-text-secondary">
            {{ t('connectionModal.sections.defaultAccessEndpoint') }}
          </div>

          <div>
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.endpointUsername') }}</label>
            <input
              v-model="formEndpoint.username"
              class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              :placeholder="t('connectionModal.placeholders.endpointUsername')"
            />
          </div>

          <div>
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.accessMethod') }}</label>
            <div class="flex items-center gap-4">
              <label class="flex cursor-pointer items-center gap-2">
                <input
                  v-model="formEndpoint.authType"
                  type="radio"
                  value="password"
                  class="border-border-primary bg-bg-tertiary text-accent focus:ring-accent"
                />
                <span class="text-sm">{{ t('connectionModal.labels.password') }}</span>
              </label>
              <label class="flex cursor-pointer items-center gap-2">
                <input
                  v-model="formEndpoint.authType"
                  type="radio"
                  value="key"
                  class="border-border-primary bg-bg-tertiary text-accent focus:ring-accent"
                />
                <span class="text-sm">{{ t('connectionModal.labels.sshKey') }}</span>
              </label>
            </div>
          </div>

          <div v-if="formEndpoint.authType === 'password'">
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.password') }}</label>
            <div class="relative">
              <input
                v-model="formCredentialRef!.secret"
                :type="showPassword ? 'text' : 'password'"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 pr-10 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.password')"
              />
              <button
                class="absolute right-2 top-2 text-text-secondary hover:text-text-primary"
                @click="showPassword = !showPassword"
              >
                <Eye v-if="!showPassword" class="h-5 w-5" />
                <EyeOff v-else class="h-5 w-5" />
              </button>
            </div>
          </div>

          <div v-else>
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.sshKey') }}</label>
            <select
              v-model="formCredentialRef!.sshKeyId"
              class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
            >
              <option :value="null" disabled>{{ t('connectionModal.selectKey') }}</option>
              <option v-for="key in sshKeyStore.keys" :key="key.id" :value="key.id">
                {{ key.name }}
              </option>
            </select>
          </div>

          <div class="grid grid-cols-4 gap-4">
            <div class="col-span-3">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.jumpHost') }}</label>
              <input
                v-model="formEndpoint.jumpHost"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.jumpHost')"
              />
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.jumpPort') }}</label>
              <input
                v-model.number="formEndpoint.jumpPort"
                type="number"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.port')"
              />
            </div>
          </div>

          <div>
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.jumpUsername') }}</label>
            <input
              v-model="formEndpoint.jumpUsername"
              class="w-full rounded border border-border-primary bg-bg-tertiary p-2 text-text-primary outline-none focus:border-accent"
              :placeholder="t('connectionModal.placeholders.jumpUsername')"
            />
          </div>

          <div>
            <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('connectionModal.labels.jumpPassword') }}</label>
            <div class="relative">
              <input
                v-model="formEndpoint.jumpPassword"
                :type="showJumpPassword ? 'text' : 'password'"
                class="w-full rounded border border-border-primary bg-bg-tertiary p-2 pr-10 text-text-primary outline-none focus:border-accent"
                :placeholder="t('connectionModal.placeholders.jumpPassword')"
              />
              <button
                class="absolute right-2 top-2 text-text-secondary hover:text-text-primary"
                @click="showJumpPassword = !showJumpPassword"
              >
                <Eye v-if="!showJumpPassword" class="h-5 w-5" />
                <EyeOff v-else class="h-5 w-5" />
              </button>
            </div>
          </div>
        </section>
      </div>

      <div
        v-if="testResult"
        class="mt-4 flex items-center gap-2 rounded p-2 text-sm"
        :class="testResult.success ? 'bg-success/20 text-success' : 'bg-error/20 text-error'"
      >
        <CheckCircle v-if="testResult.success" class="h-4 w-4" />
        <XCircle v-else class="h-4 w-4" />
        <span>{{ testResult.message }}</span>
      </div>

      <div class="mt-6 flex items-center justify-between">
        <button
          class="flex items-center gap-2 rounded bg-warning px-4 py-2 text-sm text-text-primary disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="isTesting"
          @click="testConnection"
        >
          <Loader2 v-if="isTesting" class="h-4 w-4 animate-spin" />
          <span>{{ t('connectionModal.actions.testConnection') }}</span>
        </button>

        <div class="flex gap-2">
          <button
            class="rounded bg-bg-tertiary px-4 py-2 text-sm text-text-primary hover:bg-bg-elevated"
            @click="emit('close')"
          >
            {{ t('common.cancel') }}
          </button>
          <button
            class="rounded bg-accent px-4 py-2 text-sm text-white hover:bg-accent/80"
            @click="save"
          >
            {{ t('common.save') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
