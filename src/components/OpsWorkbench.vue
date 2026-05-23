<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  Bot,
  CheckCircle2,
  ClipboardCheck,
  DatabaseZap,
  History,
  Play,
  RefreshCw,
  Search,
  UploadCloud,
} from "lucide-vue-next";
import { useAssetStore } from "../stores/assets";
import { useNotificationStore } from "../stores/notifications";
import { useI18n } from "../composables/useI18n";
import type {
  HostAsset,
  JobBatchRequest,
  JobTemplate,
  SyncServiceConfig,
} from "../types";

const props = defineProps<{
  activeAsset?: HostAsset | null;
}>();

const assetStore = useAssetStore();
const notificationStore = useNotificationStore();
const { t } = useI18n();

type OpsTab = "console" | "jobs" | "audit" | "sync";

const activeTab = ref<OpsTab>("console");
const consoleQuery = ref("");
const selectedAssetId = ref<number | null>(null);
const isRunningConsole = ref(false);

const auditQuery = ref("");
const auditSeverity = ref("");
const isSearchingAudit = ref(false);

const batchForm = ref<JobBatchRequest>({
  templateId: null,
  commandText: "",
  scopeType: "tag",
  scopeValue: "",
  targetAssetIds: [],
  riskLevel: "medium",
  source: "ops-workbench",
});
const newTemplateName = ref("");
const newTemplateRequiresConfirmation = ref(true);
const isPreviewingBatch = ref(false);
const isExecutingBatch = ref(false);

const syncServiceDraft = ref<SyncServiceConfig>({
  serviceKey: "local-first",
  displayName: "Local First Mirror",
  baseUrl: "",
  authMode: "none",
  authToken: "",
  enabled: true,
  metadataJson: "",
  createdAt: 0,
  updatedAt: 0,
});
const isSavingSyncService = ref(false);
const isMarkingSynced = ref(false);

const consoleAnswer = computed(() => assetStore.lastOpsConsoleAnswer);
const batchPreview = computed(() => assetStore.lastJobBatchPreview);
const batchResult = computed(() => assetStore.lastJobBatchResult);
const syncOverview = computed(() => assetStore.syncOverview);
const syncServices = computed(() => assetStore.syncServices);
const syncChanges = computed(() => assetStore.syncChanges);
const selectedPendingChangeIds = computed(() =>
  syncChanges.value
    .filter((item) => item.syncStatus === "pending" && item.id !== undefined)
    .slice(0, 10)
    .map((item) => item.id as number),
);

const criticalAssets = computed(() =>
  assetStore.assets.filter((asset) => asset.criticality === "critical"),
);

watch(
  () => props.activeAsset?.id,
  (assetId) => {
    if (assetId !== undefined && assetId !== null) {
      selectedAssetId.value = assetId;
    }
  },
  { immediate: true },
);

watch(
  syncServices,
  (services) => {
    const primaryService = services[0];
    if (!primaryService) return;
    syncServiceDraft.value = {
      ...primaryService,
      baseUrl: primaryService.baseUrl ?? "",
      authToken: primaryService.authToken ?? "",
      metadataJson: primaryService.metadataJson ?? "",
    };
  },
  { immediate: true },
);

function formatDateTime(value?: number | null) {
  if (!value) return "N/A";
  return new Date(value * (value > 10_000_000_000 ? 1 : 1000)).toLocaleString();
}

function riskClass(risk?: string | null) {
  switch (risk) {
    case "critical":
      return "border-error/40 bg-error/10 text-error";
    case "high":
      return "border-warning/40 bg-warning/10 text-warning";
    case "low":
      return "border-success/40 bg-success/10 text-success";
    default:
      return "border-border-primary bg-bg-primary text-text-secondary";
  }
}

async function ensureOpsDataLoaded() {
  await Promise.all([assetStore.refreshOpsData(), assetStore.loadSyncOverview()]);
}

async function runConsoleQuery() {
  if (!consoleQuery.value.trim()) {
    notificationStore.warning(t("opsWorkbench.notifications.consoleQueryRequired"));
    return;
  }
  isRunningConsole.value = true;
  try {
    await assetStore.runOpsConsoleQuery(
      consoleQuery.value.trim(),
      selectedAssetId.value,
    );
  } catch (error) {
    console.error(error);
    notificationStore.error(t("opsWorkbench.notifications.consoleQueryFailed", { error }));
  } finally {
    isRunningConsole.value = false;
  }
}

function applyTemplate(template: JobTemplate) {
  batchForm.value = {
    templateId: template.id ?? null,
    commandText: template.command,
    scopeType: template.scopeType ?? "tag",
    scopeValue: template.scopeValue ?? "",
    targetAssetIds: [],
    riskLevel: (template.riskLevel as JobBatchRequest["riskLevel"]) ?? "medium",
    source: "job-template",
  };
  newTemplateName.value = template.name;
  newTemplateRequiresConfirmation.value = template.requiresConfirmation;
}

async function createTemplateFromForm() {
  if (!newTemplateName.value.trim() || !batchForm.value.commandText.trim()) {
    notificationStore.warning(t("opsWorkbench.notifications.templateFieldsRequired"));
    return;
  }
  try {
    await assetStore.createJobTemplate({
      id: undefined,
      name: newTemplateName.value.trim(),
      command: batchForm.value.commandText.trim(),
      scopeType: batchForm.value.scopeType,
      scopeValue: batchForm.value.scopeValue?.trim() || null,
      riskLevel: batchForm.value.riskLevel ?? "medium",
      requiresConfirmation: newTemplateRequiresConfirmation.value,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    });
    notificationStore.success(t("opsWorkbench.notifications.templateCreated"));
  } catch (error) {
    console.error(error);
    notificationStore.error(t("opsWorkbench.notifications.templateCreateFailed", { error }));
  }
}

async function previewBatch() {
  if (!batchForm.value.commandText.trim()) {
    notificationStore.warning(t("opsWorkbench.notifications.commandRequired"));
    return;
  }
  isPreviewingBatch.value = true;
  try {
    await assetStore.previewJobBatch({
      ...batchForm.value,
      commandText: batchForm.value.commandText.trim(),
      scopeValue: batchForm.value.scopeValue?.trim() || null,
    });
  } catch (error) {
    console.error(error);
    notificationStore.error(t("opsWorkbench.notifications.batchPreviewFailed", { error }));
  } finally {
    isPreviewingBatch.value = false;
  }
}

async function executeBatch() {
  if (!batchPreview.value) {
    notificationStore.warning(t("opsWorkbench.notifications.batchPreviewRequired"));
    return;
  }
  if (batchPreview.value.requiresConfirmation) {
    const confirmed = window.confirm(
      t("opsWorkbench.notifications.batchExecuteConfirm", {
        count: batchPreview.value.targetCount,
      }),
    );
    if (!confirmed) return;
  }

  isExecutingBatch.value = true;
  try {
    await assetStore.executeJobBatch({
      ...batchForm.value,
      commandText: batchForm.value.commandText.trim(),
      scopeValue: batchForm.value.scopeValue?.trim() || null,
    });
    notificationStore.success(t("opsWorkbench.notifications.batchExecuted"));
  } catch (error) {
    console.error(error);
    notificationStore.error(t("opsWorkbench.notifications.batchExecuteFailed", { error }));
  } finally {
    isExecutingBatch.value = false;
  }
}

async function searchAudit() {
  isSearchingAudit.value = true;
  try {
    await assetStore.searchAuditEvents(
      auditQuery.value.trim() || undefined,
      auditSeverity.value || undefined,
      selectedAssetId.value ?? undefined,
      200,
    );
  } catch (error) {
    console.error(error);
    notificationStore.error(t("opsWorkbench.notifications.auditSearchFailed", { error }));
  } finally {
    isSearchingAudit.value = false;
  }
}

async function saveSyncService() {
  isSavingSyncService.value = true;
  try {
    await assetStore.saveSyncService({
      ...syncServiceDraft.value,
      baseUrl: syncServiceDraft.value.baseUrl?.trim() || null,
      authToken: syncServiceDraft.value.authToken?.trim() || null,
      metadataJson: syncServiceDraft.value.metadataJson?.trim() || null,
    });
    notificationStore.success(t("opsWorkbench.notifications.syncServiceSaved"));
  } catch (error) {
    console.error(error);
    notificationStore.error(t("opsWorkbench.notifications.syncServiceSaveFailed", { error }));
  } finally {
    isSavingSyncService.value = false;
  }
}

async function markPendingChangesSynced() {
  if (selectedPendingChangeIds.value.length === 0) {
    notificationStore.info(t("opsWorkbench.notifications.noPendingChanges"));
    return;
  }
  isMarkingSynced.value = true;
  try {
    await assetStore.markSyncChangesSynced(
      selectedPendingChangeIds.value,
      syncServiceDraft.value.serviceKey,
    );
    notificationStore.success(t("opsWorkbench.notifications.pendingChangesMarked"));
  } catch (error) {
    console.error(error);
    notificationStore.error(t("opsWorkbench.notifications.pendingChangesMarkFailed", { error }));
  } finally {
    isMarkingSynced.value = false;
  }
}

onMounted(async () => {
  await ensureOpsDataLoaded();
});
</script>

<template>
  <div class="flex h-full min-h-0 flex-col bg-bg-secondary">
    <div class="border-b border-border-primary px-4 py-3">
      <div class="flex items-start justify-between gap-3">
        <div>
          <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.title') }}</div>
          <div class="mt-1 text-xs text-text-secondary">
            {{ t('opsWorkbench.subtitle') }}
          </div>
        </div>
        <button
          class="inline-flex items-center gap-1.5 rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-xs text-text-primary hover:bg-bg-elevated"
          @click="ensureOpsDataLoaded"
        >
          <RefreshCw class="h-3.5 w-3.5" />
          <span>{{ t('common.refresh') }}</span>
        </button>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2 text-[11px]">
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          {{ t('workbench.assetsCount', { count: assetStore.assets.length }) }}
        </span>
        <span class="rounded-full border border-error/30 bg-error/10 px-2.5 py-1 text-error">
          {{ t('workbench.criticalCount', { count: criticalAssets.length }) }}
        </span>
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          {{ t('workbench.jobRunsCount', { count: assetStore.jobRuns.length }) }}
        </span>
        <span
          class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary"
        >
          {{ t('workbench.pendingSyncCount', { count: syncOverview?.pendingChanges ?? 0 }) }}
        </span>
      </div>
    </div>

    <div class="border-b border-border-primary px-3">
      <div class="no-scrollbar flex h-11 min-w-0 items-center gap-2 overflow-x-auto">
      <button
        class="inline-flex shrink-0 items-center rounded-lg px-3 py-2 text-sm whitespace-nowrap transition-colors"
        :class="activeTab === 'console' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'console'"
      >
        <span class="inline-flex items-center gap-2">
          <Bot class="h-4 w-4" />
          <span>{{ t('opsWorkbench.tabs.console') }}</span>
        </span>
      </button>
      <button
        class="inline-flex shrink-0 items-center rounded-lg px-3 py-2 text-sm whitespace-nowrap transition-colors"
        :class="activeTab === 'jobs' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'jobs'"
      >
        <span class="inline-flex items-center gap-2">
          <Play class="h-4 w-4" />
          <span>{{ t('opsWorkbench.tabs.jobs') }}</span>
        </span>
      </button>
      <button
        class="inline-flex shrink-0 items-center rounded-lg px-3 py-2 text-sm whitespace-nowrap transition-colors"
        :class="activeTab === 'audit' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'audit'"
      >
        <span class="inline-flex items-center gap-2">
          <History class="h-4 w-4" />
          <span>{{ t('opsWorkbench.tabs.audit') }}</span>
        </span>
      </button>
      <button
        class="inline-flex shrink-0 items-center rounded-lg px-3 py-2 text-sm whitespace-nowrap transition-colors"
        :class="activeTab === 'sync' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'sync'"
      >
        <span class="inline-flex items-center gap-2">
          <DatabaseZap class="h-4 w-4" />
          <span>{{ t('opsWorkbench.tabs.sync') }}</span>
        </span>
      </button>
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
      <div v-if="activeTab === 'console'" class="space-y-4">
        <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
          <div class="grid gap-3">
            <div class="min-w-0">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.console.queryLabel') }}</label>
              <textarea
                v-model="consoleQuery"
                rows="3"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                :placeholder="t('opsWorkbench.console.queryPlaceholder')"
              ></textarea>
            </div>
            <div class="min-w-0">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.console.focusAssetLabel') }}</label>
              <select
                v-model="selectedAssetId"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
              >
                <option :value="null">{{ t('opsWorkbench.console.focusAssetAuto') }}</option>
                <option v-for="asset in assetStore.assets" :key="asset.id" :value="asset.id">
                  {{ asset.name }}
                </option>
              </select>
            </div>
            <div class="flex justify-end">
              <button
                class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                :disabled="isRunningConsole"
                @click="runConsoleQuery"
              >
                <Search class="h-4 w-4" />
                <span>{{ isRunningConsole ? t('opsWorkbench.console.runningAction') : t('opsWorkbench.console.runAction') }}</span>
              </button>
            </div>
          </div>
        </div>

        <div
          v-if="consoleAnswer"
          class="space-y-4 rounded-xl border border-border-primary bg-bg-primary p-4"
        >
          <div>
            <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.console.summary') }}</div>
            <div class="mt-2 text-sm leading-6 text-text-secondary">
              {{ consoleAnswer.summary }}
            </div>
          </div>

          <div v-if="consoleAnswer.statusExplanation" class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
            <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
              {{ t('opsWorkbench.console.statusExplanation') }}
            </div>
            <div class="mt-2 text-sm text-text-primary">
              {{ consoleAnswer.statusExplanation }}
            </div>
          </div>

          <div class="grid gap-4">
            <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
              <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                {{ t('opsWorkbench.console.matchedAssets') }}
              </div>
              <div v-if="consoleAnswer.matchedAssets.length === 0" class="mt-3 text-sm text-text-secondary">
                {{ t('opsWorkbench.console.matchedAssetsEmpty') }}
              </div>
              <div v-else class="mt-3 space-y-2">
                <div
                  v-for="asset in consoleAnswer.matchedAssets"
                  :key="asset.assetId"
                  class="rounded border border-border-primary bg-bg-primary px-3 py-2"
                >
                  <div class="flex items-center justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate text-sm text-text-primary">{{ asset.assetName }}</div>
                      <div class="mt-1 truncate text-xs text-text-secondary">{{ asset.host }}</div>
                    </div>
                    <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="riskClass(asset.criticality)">
                      {{ asset.criticality }}
                    </span>
                  </div>
                  <div class="mt-2 flex flex-wrap gap-2 text-[11px] text-text-secondary">
                    <span v-if="asset.environmentName">Env: {{ asset.environmentName }}</span>
                    <span>{{ asset.matchReason }}</span>
                    <span v-if="asset.healthSummary">{{ asset.healthSummary }}</span>
                  </div>
                </div>
              </div>
            </div>

            <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
              <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                {{ t('opsWorkbench.console.recommendedChecks') }}
              </div>
              <div class="mt-3 space-y-2">
                <div
                  v-for="item in consoleAnswer.recommendedChecks"
                  :key="item"
                  class="rounded border border-border-primary bg-bg-primary px-3 py-2 text-sm text-text-primary"
                >
                  {{ item }}
                </div>
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
              <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                {{ t('opsWorkbench.console.executionPlan') }}
              </div>
            <div class="mt-3 space-y-3">
              <div
                v-for="step in consoleAnswer.planSteps"
                :key="step.id"
                class="rounded border border-border-primary bg-bg-primary px-3 py-3"
              >
                <div class="flex items-center justify-between gap-3">
                  <div class="text-sm font-medium text-text-primary">{{ step.title }}</div>
                  <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="riskClass(step.riskLevel)">
                    {{ step.riskLevel }}
                  </span>
                </div>
                <div class="mt-2 text-sm text-text-secondary">{{ step.description }}</div>
                <div v-if="step.command" class="mt-2 rounded bg-bg-secondary px-3 py-2 font-mono text-xs text-text-primary">
                  {{ step.command }}
                </div>
                <div class="mt-2 flex flex-wrap gap-2 text-[11px] text-text-secondary">
                    <span v-if="step.targetAssetName">{{ t('opsWorkbench.console.targetAsset', { name: step.targetAssetName }) }}</span>
                    <span>{{ step.requiresConfirmation ? t('opsWorkbench.console.requiresConfirmation') : t('opsWorkbench.console.safeStep') }}</span>
                </div>
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
              <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                {{ t('opsWorkbench.console.reviewChecklist') }}
              </div>
            <div class="mt-3 flex flex-col gap-2">
              <div
                v-for="item in consoleAnswer.reviewChecklist"
                :key="item"
                class="rounded border border-border-primary bg-bg-primary px-3 py-2 text-sm text-text-primary"
              >
                {{ item }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-else-if="activeTab === 'jobs'" class="space-y-4">
        <div class="grid gap-4">
          <div class="space-y-4">
            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.jobs.templatesTitle') }}</div>
                  <div class="mt-1 text-xs text-text-secondary">
                    {{ t('opsWorkbench.jobs.templatesSubtitle') }}
                  </div>
                </div>
                <span class="rounded-full border border-border-primary bg-bg-secondary px-2.5 py-1 text-[11px] text-text-secondary">
                  {{ t('opsWorkbench.jobs.templateCount', { count: assetStore.jobTemplates.length }) }}
                </span>
              </div>

              <div class="mt-4 space-y-2">
                <button
                  v-for="template in assetStore.jobTemplates"
                  :key="template.id"
                  class="w-full rounded-lg border border-border-primary bg-bg-secondary px-3 py-3 text-left hover:bg-bg-elevated"
                  @click="applyTemplate(template)"
                >
                  <div class="flex items-center justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate text-sm text-text-primary">{{ template.name }}</div>
                      <div class="mt-1 truncate font-mono text-xs text-text-secondary">
                        {{ template.command }}
                      </div>
                    </div>
                    <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="riskClass(template.riskLevel)">
                      {{ template.riskLevel }}
                    </span>
                  </div>
                </button>
              </div>
            </div>

            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.jobs.archiveTitle') }}</div>
              <div class="mt-3 space-y-2">
                <div
                  v-for="archive in assetStore.jobArchives.slice(0, 8)"
                  :key="archive.id"
                  class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3"
                >
                  <div class="flex items-center justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate text-sm text-text-primary">{{ archive.summary || archive.command }}</div>
                      <div class="mt-1 text-[11px] text-text-secondary">{{ formatDateTime(archive.archivedAt) }}</div>
                    </div>
                    <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="riskClass(archive.riskLevel)">
                      {{ archive.status }}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
            <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.jobs.batchExecutionTitle') }}</div>
            <div class="mt-1 text-xs text-text-secondary">
              {{ t('opsWorkbench.jobs.batchExecutionSubtitle') }}
            </div>

            <div class="mt-4 grid gap-3">
              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.jobs.templateNameLabel') }}</label>
                <input
                  v-model="newTemplateName"
                  class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  :placeholder="t('opsWorkbench.jobs.templateNamePlaceholder')"
                />
              </div>

              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.jobs.commandLabel') }}</label>
                <textarea
                  v-model="batchForm.commandText"
                  rows="3"
                  class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent"
                  :placeholder="t('opsWorkbench.jobs.commandPlaceholder')"
                ></textarea>
              </div>

              <div class="grid gap-3">
                <div class="min-w-0">
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.jobs.scopeLabel') }}</label>
                  <select
                    v-model="batchForm.scopeType"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  >
                    <option value="tag">{{ t('opsWorkbench.jobs.scopeOptions.tag') }}</option>
                    <option value="environment">{{ t('opsWorkbench.jobs.scopeOptions.environment') }}</option>
                    <option value="asset">{{ t('opsWorkbench.jobs.scopeOptions.asset') }}</option>
                    <option value="all">{{ t('opsWorkbench.jobs.scopeOptions.all') }}</option>
                  </select>
                </div>
                <div class="min-w-0">
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.jobs.scopeValueLabel') }}</label>
                  <input
                    v-model="batchForm.scopeValue"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                    :placeholder="t('opsWorkbench.jobs.scopeValuePlaceholder')"
                  />
                </div>
                <div class="min-w-0">
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.jobs.riskLabel') }}</label>
                  <select
                    v-model="batchForm.riskLevel"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  >
                    <option value="low">{{ t('opsWorkbench.jobs.riskOptions.low') }}</option>
                    <option value="medium">{{ t('opsWorkbench.jobs.riskOptions.medium') }}</option>
                    <option value="high">{{ t('opsWorkbench.jobs.riskOptions.high') }}</option>
                    <option value="critical">{{ t('opsWorkbench.jobs.riskOptions.critical') }}</option>
                  </select>
                </div>
              </div>

              <label class="inline-flex items-center gap-2 text-sm text-text-secondary">
                <input
                  v-model="newTemplateRequiresConfirmation"
                  type="checkbox"
                  class="rounded border-border-primary bg-bg-tertiary text-accent focus:ring-accent"
                />
                <span>{{ t('opsWorkbench.jobs.requiresConfirmation') }}</span>
              </label>

              <div class="flex flex-wrap gap-2">
                <button
                  class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-bg-tertiary px-4 text-sm text-text-primary hover:bg-bg-elevated"
                  @click="createTemplateFromForm"
                >
                  <ClipboardCheck class="h-4 w-4" />
                  <span>{{ t('opsWorkbench.jobs.saveTemplate') }}</span>
                </button>
                <button
                  class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                  :disabled="isPreviewingBatch"
                  @click="previewBatch"
                >
                  <Search class="h-4 w-4" />
                  <span>{{ isPreviewingBatch ? t('opsWorkbench.jobs.previewingScope') : t('opsWorkbench.jobs.previewScope') }}</span>
                </button>
                <button
                  class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-warning px-4 text-sm text-text-primary hover:opacity-90"
                  :disabled="isExecutingBatch"
                  @click="executeBatch"
                >
                  <Play class="h-4 w-4" />
                  <span>{{ isExecutingBatch ? t('opsWorkbench.jobs.executingBatch') : t('opsWorkbench.jobs.executeBatch') }}</span>
                </button>
              </div>
            </div>

            <div v-if="batchPreview" class="mt-5 rounded-xl border border-border-primary bg-bg-secondary p-4">
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.jobs.batchPreviewTitle') }}</div>
                <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-[11px] text-text-secondary">
                  {{ t('opsWorkbench.jobs.batchPreviewTargets', { count: batchPreview.targetCount }) }}
                </span>
              </div>
              <div class="mt-3 space-y-2">
                <div
                  v-for="target in batchPreview.targets"
                  :key="target.assetId"
                  class="rounded-lg border border-border-primary bg-bg-primary px-3 py-2"
                >
                  <div class="flex items-center justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate text-sm text-text-primary">{{ target.assetName }}</div>
                      <div class="mt-1 truncate text-[11px] text-text-secondary">{{ target.host }}</div>
                    </div>
                    <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="riskClass(target.riskLevel)">
                      {{ target.riskLevel }}
                    </span>
                  </div>
                  <div class="mt-2 flex flex-wrap gap-2 text-[11px] text-text-secondary">
                    <span>{{ target.matchReason }}</span>
                    <span v-if="target.environmentName">{{ t('opsWorkbench.jobs.environment', { name: target.environmentName }) }}</span>
                    <span v-if="target.labels.length > 0">{{ t('opsWorkbench.jobs.labels', { labels: target.labels.join(", ") }) }}</span>
                  </div>
                </div>
              </div>
              <div v-if="batchPreview.warnings.length > 0" class="mt-3 space-y-2">
                <div
                  v-for="warning in batchPreview.warnings"
                  :key="warning"
                  class="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning"
                >
                  {{ warning }}
                </div>
              </div>
            </div>

            <div v-if="batchResult" class="mt-5 rounded-xl border border-border-primary bg-bg-secondary p-4">
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.jobs.executionReviewTitle') }}</div>
                <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-[11px] text-text-secondary">
                  {{ t('opsWorkbench.jobs.executionReviewSummary', { completed: batchResult.completed, failed: batchResult.failed }) }}
                </span>
              </div>
              <div class="mt-3 space-y-2">
                <div
                  v-for="item in batchResult.items"
                  :key="`${item.assetId}-${item.jobRunId}`"
                  class="rounded-lg border border-border-primary bg-bg-primary px-3 py-3"
                >
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate text-sm text-text-primary">{{ item.assetName }}</div>
                      <div class="mt-1 text-[11px] text-text-secondary">
                        {{ item.usedExistingSession ? t('opsWorkbench.jobs.existingSession') : t('opsWorkbench.jobs.temporarySession') }}
                      </div>
                    </div>
                    <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="riskClass(item.riskLevel)">
                      {{ item.status }}
                    </span>
                  </div>
                  <div v-if="item.error" class="mt-2 text-xs text-error">{{ item.error }}</div>
                  <pre v-else-if="item.output" class="mt-2 overflow-x-auto rounded bg-bg-secondary px-3 py-2 text-[11px] text-text-secondary">{{ item.output }}</pre>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-else-if="activeTab === 'audit'" class="space-y-4">
        <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
          <div class="grid gap-3">
            <div class="min-w-0">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.audit.searchLabel') }}</label>
              <input
                v-model="auditQuery"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                :placeholder="t('opsWorkbench.audit.searchPlaceholder')"
              />
            </div>
            <div class="min-w-0">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.audit.severityLabel') }}</label>
              <select
                v-model="auditSeverity"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
              >
                <option value="">{{ t('opsWorkbench.audit.severityOptions.all') }}</option>
                <option value="info">{{ t('opsWorkbench.audit.severityOptions.info') }}</option>
                <option value="warning">{{ t('opsWorkbench.audit.severityOptions.warning') }}</option>
                <option value="error">{{ t('opsWorkbench.audit.severityOptions.error') }}</option>
              </select>
            </div>
            <div class="min-w-0">
              <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.audit.assetFilterLabel') }}</label>
              <select
                v-model="selectedAssetId"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
              >
                <option :value="null">{{ t('opsWorkbench.audit.assetFilterAll') }}</option>
                <option v-for="asset in assetStore.assets" :key="asset.id" :value="asset.id">
                  {{ asset.name }}
                </option>
              </select>
            </div>
            <div class="flex justify-end">
              <button
                class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                :disabled="isSearchingAudit"
                @click="searchAudit"
              >
                <Search class="h-4 w-4" />
                <span>{{ isSearchingAudit ? t('opsWorkbench.audit.searchingAction') : t('opsWorkbench.audit.searchAction') }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="space-y-2">
          <div
            v-for="event in assetStore.auditEvents"
            :key="event.id"
            class="rounded-xl border border-border-primary bg-bg-primary px-4 py-3"
          >
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0">
                <div class="truncate text-sm font-medium text-text-primary">{{ event.title }}</div>
                <div class="mt-1 text-xs text-text-secondary">{{ event.eventType }}</div>
              </div>
              <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="riskClass(event.severity)">
                {{ event.severity }}
              </span>
            </div>
            <div v-if="event.detail" class="mt-2 text-sm text-text-secondary">
              {{ event.detail }}
            </div>
            <div class="mt-2 text-[11px] text-text-secondary">
              {{ formatDateTime(event.createdAt) }}
            </div>
          </div>
        </div>
      </div>

      <div v-else class="space-y-4">
        <div class="grid gap-4">
          <div class="space-y-4">
            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.sync.overviewTitle') }}</div>
                  <div class="mt-1 text-xs text-text-secondary">
                    {{ t('opsWorkbench.sync.overviewSubtitle') }}
                  </div>
                </div>
                <span class="rounded-full border border-border-primary bg-bg-secondary px-2.5 py-1 text-[11px] text-text-secondary">
                  {{ syncOverview?.protocolVersion || "local-first/v1" }}
                </span>
              </div>

              <div class="mt-4 grid gap-3">
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">{{ t('opsWorkbench.sync.stateLabel') }}</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ syncOverview?.state.status || "idle" }}</div>
                </div>
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">{{ t('opsWorkbench.sync.pendingLabel') }}</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ syncOverview?.pendingChanges || 0 }}</div>
                </div>
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">{{ t('opsWorkbench.sync.totalChangesLabel') }}</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ syncOverview?.totalChanges || 0 }}</div>
                </div>
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">{{ t('opsWorkbench.sync.lastChangeLabel') }}</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ formatDateTime(syncOverview?.lastChangeAt) }}</div>
                </div>
              </div>

              <div class="mt-4 rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                  Object Versions
                </div>
                <div class="mt-3 grid gap-2">
                  <div
                    v-for="item in syncOverview?.objectVersionSummary || []"
                    :key="item.objectType"
                    class="rounded border border-border-primary bg-bg-primary px-3 py-2"
                  >
                    <div class="text-sm text-text-primary">{{ item.objectType }}</div>
                    <div class="mt-1 text-[11px] text-text-secondary">
                      {{ item.count }} object(s) · max version {{ item.maxVersion }}
                    </div>
                  </div>
                </div>
              </div>

              <div class="mt-4 rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                <div class="flex items-center justify-between gap-3">
                  <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                    Recent Change Log
                  </div>
                  <button
                    class="inline-flex h-8 items-center gap-1.5 rounded border border-border-primary bg-bg-primary px-3 text-xs text-text-primary hover:bg-bg-elevated"
                    :disabled="isMarkingSynced"
                    @click="markPendingChangesSynced"
                  >
                    <CheckCircle2 class="h-3.5 w-3.5" />
                    <span>{{ isMarkingSynced ? "Updating..." : "Mark Recent Pending as Synced" }}</span>
                  </button>
                </div>

                <div class="mt-3 space-y-2">
                  <div
                    v-for="change in syncChanges.slice(0, 12)"
                    :key="change.id"
                    class="rounded border border-border-primary bg-bg-primary px-3 py-3"
                  >
                    <div class="flex items-start justify-between gap-3">
                      <div class="min-w-0">
                        <div class="truncate text-sm text-text-primary">{{ change.summary }}</div>
                        <div class="mt-1 text-[11px] text-text-secondary">
                          {{ change.objectType }} · {{ change.objectId }} · v{{ change.objectVersion }}
                        </div>
                      </div>
                      <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="change.syncStatus === 'synced' ? 'border-success/30 bg-success/10 text-success' : 'border-warning/30 bg-warning/10 text-warning'">
                        {{ change.syncStatus }}
                      </span>
                    </div>
                    <div class="mt-2 text-[11px] text-text-secondary">
                      {{ formatDateTime(change.createdAt) }}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="space-y-4">
            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.sync.serviceAdapterTitle') }}</div>
              <div class="mt-1 text-xs text-text-secondary">
                {{ t('opsWorkbench.sync.serviceAdapterSubtitle') }}
              </div>

              <div class="mt-4 grid gap-3">
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.sync.serviceKeyLabel') }}</label>
                  <input
                    v-model="syncServiceDraft.serviceKey"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.sync.displayNameLabel') }}</label>
                  <input
                    v-model="syncServiceDraft.displayName"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.sync.baseUrlLabel') }}</label>
                  <input
                    v-model="syncServiceDraft.baseUrl"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                    placeholder="https://sync.example.com"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.sync.authModeLabel') }}</label>
                  <select
                    v-model="syncServiceDraft.authMode"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  >
                    <option value="none">{{ t('opsWorkbench.sync.authModeOptions.none') }}</option>
                    <option value="token">{{ t('opsWorkbench.sync.authModeOptions.token') }}</option>
                  </select>
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.sync.authTokenLabel') }}</label>
                  <input
                    v-model="syncServiceDraft.authToken"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                    :placeholder="t('opsWorkbench.sync.authTokenPlaceholder')"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">{{ t('opsWorkbench.sync.metadataJsonLabel') }}</label>
                  <textarea
                    v-model="syncServiceDraft.metadataJson"
                    rows="3"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent"
                    :placeholder="t('opsWorkbench.sync.metadataJsonPlaceholder')"
                  ></textarea>
                </div>
                <label class="inline-flex items-center gap-2 text-sm text-text-secondary">
                  <input
                    v-model="syncServiceDraft.enabled"
                    type="checkbox"
                    class="rounded border-border-primary bg-bg-tertiary text-accent focus:ring-accent"
                  />
                  <span>{{ t('opsWorkbench.sync.adapterEnabled') }}</span>
                </label>
                <button
                  class="inline-flex h-10 items-center justify-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                  :disabled="isSavingSyncService"
                  @click="saveSyncService"
                >
                  <UploadCloud class="h-4 w-4" />
                  <span>{{ isSavingSyncService ? t('opsWorkbench.sync.savingAdapter') : t('opsWorkbench.sync.saveAdapter') }}</span>
                </button>
              </div>
            </div>

            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="text-sm font-semibold text-text-primary">{{ t('opsWorkbench.sync.registeredServicesTitle') }}</div>
              <div class="mt-3 space-y-2">
                <div
                  v-for="service in syncServices"
                  :key="service.serviceKey"
                  class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3"
                >
                  <div class="flex items-center justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate text-sm text-text-primary">{{ service.displayName }}</div>
                      <div class="mt-1 text-[11px] text-text-secondary">{{ service.serviceKey }}</div>
                    </div>
                    <span class="rounded-full border px-2 py-0.5 text-[11px]" :class="service.enabled ? 'border-success/30 bg-success/10 text-success' : 'border-border-primary bg-bg-primary text-text-secondary'">
                      {{ service.enabled ? "enabled" : "disabled" }}
                    </span>
                  </div>
                  <div class="mt-2 text-[11px] text-text-secondary">
                    {{ service.baseUrl || "No remote endpoint configured" }}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
