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
    notificationStore.warning("请输入 Ops Console 查询内容");
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
    notificationStore.error(`Ops Console 查询失败: ${error}`);
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
    notificationStore.warning("请先填写模板名称和命令");
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
    notificationStore.success("作业模板已创建");
  } catch (error) {
    console.error(error);
    notificationStore.error(`创建模板失败: ${error}`);
  }
}

async function previewBatch() {
  if (!batchForm.value.commandText.trim()) {
    notificationStore.warning("请输入要执行的命令");
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
    notificationStore.error(`批量预览失败: ${error}`);
  } finally {
    isPreviewingBatch.value = false;
  }
}

async function executeBatch() {
  if (!batchPreview.value) {
    notificationStore.warning("请先预览批量执行范围");
    return;
  }
  if (batchPreview.value.requiresConfirmation) {
    const confirmed = window.confirm(
      `将对 ${batchPreview.value.targetCount} 台资产执行命令，是否继续？`,
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
    notificationStore.success("批量作业执行完成");
  } catch (error) {
    console.error(error);
    notificationStore.error(`批量执行失败: ${error}`);
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
    notificationStore.error(`审计检索失败: ${error}`);
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
    notificationStore.success("同步服务配置已保存");
  } catch (error) {
    console.error(error);
    notificationStore.error(`保存同步服务失败: ${error}`);
  } finally {
    isSavingSyncService.value = false;
  }
}

async function markPendingChangesSynced() {
  if (selectedPendingChangeIds.value.length === 0) {
    notificationStore.info("当前没有待标记为已同步的变更");
    return;
  }
  isMarkingSynced.value = true;
  try {
    await assetStore.markSyncChangesSynced(
      selectedPendingChangeIds.value,
      syncServiceDraft.value.serviceKey,
    );
    notificationStore.success("已标记最近待同步变更");
  } catch (error) {
    console.error(error);
    notificationStore.error(`标记同步失败: ${error}`);
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
          <div class="text-sm font-semibold text-text-primary">Ops Workbench</div>
          <div class="mt-1 text-xs text-text-secondary">
            Global console, jobs, audit trail, and sync readiness in one place
          </div>
        </div>
        <button
          class="inline-flex items-center gap-1.5 rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-xs text-text-primary hover:bg-bg-elevated"
          @click="ensureOpsDataLoaded"
        >
          <RefreshCw class="h-3.5 w-3.5" />
          <span>Refresh</span>
        </button>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2 text-[11px]">
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          Assets {{ assetStore.assets.length }}
        </span>
        <span class="rounded-full border border-error/30 bg-error/10 px-2.5 py-1 text-error">
          Critical {{ criticalAssets.length }}
        </span>
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          Job Runs {{ assetStore.jobRuns.length }}
        </span>
        <span
          class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary"
        >
          Pending Sync {{ syncOverview?.pendingChanges ?? 0 }}
        </span>
      </div>
    </div>

    <div class="flex h-11 items-center gap-2 border-b border-border-primary px-3">
      <button
        class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
        :class="activeTab === 'console' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'console'"
      >
        <span class="inline-flex items-center gap-2">
          <Bot class="h-4 w-4" />
          <span>Ops Console</span>
        </span>
      </button>
      <button
        class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
        :class="activeTab === 'jobs' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'jobs'"
      >
        <span class="inline-flex items-center gap-2">
          <Play class="h-4 w-4" />
          <span>Jobs</span>
        </span>
      </button>
      <button
        class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
        :class="activeTab === 'audit' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'audit'"
      >
        <span class="inline-flex items-center gap-2">
          <History class="h-4 w-4" />
          <span>Audit</span>
        </span>
      </button>
      <button
        class="flex-1 rounded-lg px-3 py-2 text-sm transition-colors"
        :class="activeTab === 'sync' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary hover:bg-bg-primary hover:text-text-primary'"
        @click="activeTab = 'sync'"
      >
        <span class="inline-flex items-center gap-2">
          <DatabaseZap class="h-4 w-4" />
          <span>Sync</span>
        </span>
      </button>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
      <div v-if="activeTab === 'console'" class="space-y-4">
        <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
          <div class="grid gap-3 lg:grid-cols-[minmax(0,1fr)_220px_auto]">
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">Ops Query</label>
              <textarea
                v-model="consoleQuery"
                rows="3"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                placeholder="例如：哪些 prod 标签资产最近磁盘风险高，并给我一个安全执行计划"
              ></textarea>
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">Focus Asset</label>
              <select
                v-model="selectedAssetId"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
              >
                <option :value="null">Auto match</option>
                <option v-for="asset in assetStore.assets" :key="asset.id" :value="asset.id">
                  {{ asset.name }}
                </option>
              </select>
            </div>
            <div class="flex items-end">
              <button
                class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                :disabled="isRunningConsole"
                @click="runConsoleQuery"
              >
                <Search class="h-4 w-4" />
                <span>{{ isRunningConsole ? "Running..." : "Analyze" }}</span>
              </button>
            </div>
          </div>
        </div>

        <div
          v-if="consoleAnswer"
          class="space-y-4 rounded-xl border border-border-primary bg-bg-primary p-4"
        >
          <div>
            <div class="text-sm font-semibold text-text-primary">Summary</div>
            <div class="mt-2 text-sm leading-6 text-text-secondary">
              {{ consoleAnswer.summary }}
            </div>
          </div>

          <div v-if="consoleAnswer.statusExplanation" class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
            <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
              Status Explanation
            </div>
            <div class="mt-2 text-sm text-text-primary">
              {{ consoleAnswer.statusExplanation }}
            </div>
          </div>

          <div class="grid gap-4 xl:grid-cols-2">
            <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
              <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                Matched Assets
              </div>
              <div v-if="consoleAnswer.matchedAssets.length === 0" class="mt-3 text-sm text-text-secondary">
                No assets matched yet.
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
                Recommended Checks
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
              Execution Plan
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
                  <span v-if="step.targetAssetName">Target: {{ step.targetAssetName }}</span>
                  <span>{{ step.requiresConfirmation ? "Requires confirmation" : "Read-only or safe step" }}</span>
                </div>
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
            <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
              Review Checklist
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
        <div class="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
          <div class="space-y-4">
            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <div class="text-sm font-semibold text-text-primary">Job Templates</div>
                  <div class="mt-1 text-xs text-text-secondary">
                    Reuse safe command packs for single-host or batch operations
                  </div>
                </div>
                <span class="rounded-full border border-border-primary bg-bg-secondary px-2.5 py-1 text-[11px] text-text-secondary">
                  {{ assetStore.jobTemplates.length }} templates
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
              <div class="text-sm font-semibold text-text-primary">Recent Results Archive</div>
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
            <div class="text-sm font-semibold text-text-primary">Batch Execution</div>
            <div class="mt-1 text-xs text-text-secondary">
              Preview by label/environment before execution, then confirm and archive results
            </div>

            <div class="mt-4 grid gap-3">
              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">Template Name</label>
                <input
                  v-model="newTemplateName"
                  class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  placeholder="例如：Prod disk inspection"
                />
              </div>

              <div>
                <label class="mb-1 block text-xs uppercase text-text-secondary">Command</label>
                <textarea
                  v-model="batchForm.commandText"
                  rows="3"
                  class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent"
                  placeholder="df -h"
                ></textarea>
              </div>

              <div class="grid gap-3 md:grid-cols-3">
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Scope</label>
                  <select
                    v-model="batchForm.scopeType"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  >
                    <option value="tag">Tag</option>
                    <option value="environment">Environment</option>
                    <option value="asset">Asset Query</option>
                    <option value="all">All Assets</option>
                  </select>
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Scope Value</label>
                  <input
                    v-model="batchForm.scopeValue"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                    placeholder="prod / api / owner"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Risk</label>
                  <select
                    v-model="batchForm.riskLevel"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  >
                    <option value="low">low</option>
                    <option value="medium">medium</option>
                    <option value="high">high</option>
                    <option value="critical">critical</option>
                  </select>
                </div>
              </div>

              <label class="inline-flex items-center gap-2 text-sm text-text-secondary">
                <input
                  v-model="newTemplateRequiresConfirmation"
                  type="checkbox"
                  class="rounded border-border-primary bg-bg-tertiary text-accent focus:ring-accent"
                />
                <span>Template requires confirmation</span>
              </label>

              <div class="flex flex-wrap gap-2">
                <button
                  class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-bg-tertiary px-4 text-sm text-text-primary hover:bg-bg-elevated"
                  @click="createTemplateFromForm"
                >
                  <ClipboardCheck class="h-4 w-4" />
                  <span>Save Template</span>
                </button>
                <button
                  class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                  :disabled="isPreviewingBatch"
                  @click="previewBatch"
                >
                  <Search class="h-4 w-4" />
                  <span>{{ isPreviewingBatch ? "Previewing..." : "Preview Scope" }}</span>
                </button>
                <button
                  class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-warning px-4 text-sm text-text-primary hover:opacity-90"
                  :disabled="isExecutingBatch"
                  @click="executeBatch"
                >
                  <Play class="h-4 w-4" />
                  <span>{{ isExecutingBatch ? "Running..." : "Execute Batch" }}</span>
                </button>
              </div>
            </div>

            <div v-if="batchPreview" class="mt-5 rounded-xl border border-border-primary bg-bg-secondary p-4">
              <div class="flex items-center justify-between gap-3">
                <div class="text-sm font-semibold text-text-primary">Batch Preview</div>
                <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-[11px] text-text-secondary">
                  {{ batchPreview.targetCount }} target(s)
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
                    <span v-if="target.environmentName">Env: {{ target.environmentName }}</span>
                    <span v-if="target.labels.length > 0">Labels: {{ target.labels.join(", ") }}</span>
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
                <div class="text-sm font-semibold text-text-primary">Execution Review</div>
                <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-[11px] text-text-secondary">
                  Success {{ batchResult.completed }} / Failed {{ batchResult.failed }}
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
                        {{ item.usedExistingSession ? "Reused existing session" : "Temporary session created" }}
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
          <div class="grid gap-3 lg:grid-cols-[minmax(0,1fr)_180px_180px_auto]">
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">Search Audit</label>
              <input
                v-model="auditQuery"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                placeholder="事件、细节、metadata"
              />
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">Severity</label>
              <select
                v-model="auditSeverity"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
              >
                <option value="">All</option>
                <option value="info">info</option>
                <option value="warning">warning</option>
                <option value="error">error</option>
              </select>
            </div>
            <div>
              <label class="mb-1 block text-xs uppercase text-text-secondary">Asset Filter</label>
              <select
                v-model="selectedAssetId"
                class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
              >
                <option :value="null">All assets</option>
                <option v-for="asset in assetStore.assets" :key="asset.id" :value="asset.id">
                  {{ asset.name }}
                </option>
              </select>
            </div>
            <div class="flex items-end">
              <button
                class="inline-flex h-10 items-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                :disabled="isSearchingAudit"
                @click="searchAudit"
              >
                <Search class="h-4 w-4" />
                <span>{{ isSearchingAudit ? "Searching..." : "Search" }}</span>
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
        <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(340px,0.9fr)]">
          <div class="space-y-4">
            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="flex items-center justify-between gap-3">
                <div>
                  <div class="text-sm font-semibold text-text-primary">Sync Overview</div>
                  <div class="mt-1 text-xs text-text-secondary">
                    Local-first protocol status, object versions, and recent change log
                  </div>
                </div>
                <span class="rounded-full border border-border-primary bg-bg-secondary px-2.5 py-1 text-[11px] text-text-secondary">
                  {{ syncOverview?.protocolVersion || "local-first/v1" }}
                </span>
              </div>

              <div class="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">State</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ syncOverview?.state.status || "idle" }}</div>
                </div>
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">Pending</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ syncOverview?.pendingChanges || 0 }}</div>
                </div>
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">Total Changes</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ syncOverview?.totalChanges || 0 }}</div>
                </div>
                <div class="rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                  <div class="text-xs text-text-secondary">Last Change</div>
                  <div class="mt-1 text-sm font-medium text-text-primary">{{ formatDateTime(syncOverview?.lastChangeAt) }}</div>
                </div>
              </div>

              <div class="mt-4 rounded-lg border border-border-primary bg-bg-secondary px-3 py-3">
                <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                  Object Versions
                </div>
                <div class="mt-3 grid gap-2 md:grid-cols-2">
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
              <div class="text-sm font-semibold text-text-primary">Sync Service Adapter</div>
              <div class="mt-1 text-xs text-text-secondary">
                Configurable central service interface for future push/pull integration
              </div>

              <div class="mt-4 grid gap-3">
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Service Key</label>
                  <input
                    v-model="syncServiceDraft.serviceKey"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Display Name</label>
                  <input
                    v-model="syncServiceDraft.displayName"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Base URL</label>
                  <input
                    v-model="syncServiceDraft.baseUrl"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                    placeholder="https://sync.example.com"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Auth Mode</label>
                  <select
                    v-model="syncServiceDraft.authMode"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                  >
                    <option value="none">none</option>
                    <option value="token">token</option>
                  </select>
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Auth Token</label>
                  <input
                    v-model="syncServiceDraft.authToken"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary outline-none focus:border-accent"
                    placeholder="Optional"
                  />
                </div>
                <div>
                  <label class="mb-1 block text-xs uppercase text-text-secondary">Metadata JSON</label>
                  <textarea
                    v-model="syncServiceDraft.metadataJson"
                    rows="3"
                    class="w-full rounded border border-border-primary bg-bg-tertiary px-3 py-2 font-mono text-sm text-text-primary outline-none focus:border-accent"
                    placeholder='{"supportsPush":true,"supportsPull":true}'
                  ></textarea>
                </div>
                <label class="inline-flex items-center gap-2 text-sm text-text-secondary">
                  <input
                    v-model="syncServiceDraft.enabled"
                    type="checkbox"
                    class="rounded border-border-primary bg-bg-tertiary text-accent focus:ring-accent"
                  />
                  <span>Adapter enabled</span>
                </label>
                <button
                  class="inline-flex h-10 items-center justify-center gap-2 rounded border border-border-primary bg-accent px-4 text-sm text-white hover:opacity-90"
                  :disabled="isSavingSyncService"
                  @click="saveSyncService"
                >
                  <UploadCloud class="h-4 w-4" />
                  <span>{{ isSavingSyncService ? "Saving..." : "Save Adapter" }}</span>
                </button>
              </div>
            </div>

            <div class="rounded-xl border border-border-primary bg-bg-primary p-4">
              <div class="text-sm font-semibold text-text-primary">Registered Services</div>
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
