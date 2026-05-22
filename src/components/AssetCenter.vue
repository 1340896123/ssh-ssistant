<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  BookMarked,
  Briefcase,
  Cable,
  FolderPlus,
  FolderTree,
  HardDrive,
  History,
  Layers3,
  Plus,
  Search,
  ShieldAlert,
  Star,
  Tags,
} from "lucide-vue-next";
import { useAssetStore } from "../stores/assets";
import { useSessionStore } from "../stores/sessions";
import { useNotificationStore } from "../stores/notifications";
import type {
  AssetFolder,
  ConnectionHistoryEntry,
  ConnectionHistorySource,
  HostAsset,
} from "../types";
import ConnectionTreeItem from "./ConnectionTreeItem.vue";

type HistoryFilter = "all" | "success" | "failed";

const emit = defineEmits<{
  (e: "edit", asset: HostAsset | null): void;
  (e: "tunnels", asset: HostAsset): void;
}>();

const assetStore = useAssetStore();
const sessionStore = useSessionStore();
const notificationStore = useNotificationStore();

const searchQuery = ref("");
const historyFilter = ref<HistoryFilter>("all");
const isHistoryExpanded = ref(false);
const isFavoritesExpanded = ref(false);
const selectedAssetId = ref<number | null>(null);

onMounted(async () => {
  await assetStore.loadAssets();
});

const query = computed(() => searchQuery.value.trim().toLowerCase());
const isSearchMode = computed(() => query.value.length > 0);

const assets = computed(() => assetStore.assets);
const folders = computed(() => assetStore.folders);
const environments = computed(() => assetStore.environments);
const tags = computed(() => assetStore.tags);
const savedViews = computed(() => assetStore.savedViews);
const favoriteAssets = computed(() => assetStore.favoriteAssets);
const activeSessions = computed(() => sessionStore.sessions.length);
const treeData = computed(() => assetStore.treeData);

const favoritePreview = computed(() =>
  isFavoritesExpanded.value ? favoriteAssets.value : favoriteAssets.value.slice(0, 4),
);

const mappedHistoryEntries = computed(() =>
  assetStore.historyEntries
    .map((entry) => {
      const asset = assetStore.assets.find((item) => item.id === entry.connectionId);
      if (!asset) return null;
      return { entry, asset };
    })
    .filter(
      (item): item is { entry: ConnectionHistoryEntry; asset: HostAsset } =>
        item !== null,
    ),
);

const historyItems = computed(() =>
  mappedHistoryEntries.value.filter((item) => {
    if (historyFilter.value === "all") return true;
    return item.entry.status === historyFilter.value;
  }),
);

const visibleHistoryItems = computed(() =>
  historyItems.value.slice(0, isHistoryExpanded.value ? 8 : 4),
);

const environmentMap = computed(() => {
  const map = new Map<number, string>();
  for (const env of environments.value) {
    if (env.id !== undefined) map.set(env.id, env.name);
  }
  return map;
});

function endpointLabel(asset: HostAsset) {
  const endpoint = asset.id ? assetStore.defaultAccessEndpointForAsset(asset.id) : null;
  return endpoint ? `${endpoint.username}@${endpoint.host}` : asset.host;
}

const selectedAsset = computed(() =>
  assetStore.assets.find((asset) => asset.id === selectedAssetId.value) ?? null,
);

const selectedEndpoint = computed(() =>
  selectedAsset.value?.id
    ? assetStore.defaultAccessEndpointForAsset(selectedAsset.value.id)
    : null,
);

const selectedAuditEvents = computed(() =>
  assetStore.auditEvents
    .filter((event) => event.assetId === selectedAssetId.value)
    .slice(0, 4),
);

const searchResults = computed(() => {
  if (!query.value) return [];
  return assets.value.filter((asset) => {
    const values = [
      asset.name,
      asset.host,
      asset.owner,
      ...(asset.labels ?? []),
      environmentMap.value.get(asset.envId ?? -1) ?? "",
      endpointLabel(asset),
    ];
    return values.some((value) => value?.toLowerCase().includes(query.value));
  });
});

function formatRecentTime(timestamp: number) {
  const diff = Date.now() - timestamp;
  const minutes = Math.max(1, Math.floor(diff / 60000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

function sourceLabel(source: ConnectionHistorySource | "favorite") {
  switch (source) {
    case "favorite":
      return "Favorite";
    case "history":
      return "History";
    case "quick":
      return "Quick";
    case "search":
      return "Search";
    default:
      return "Tree";
  }
}

function connect(asset: HostAsset, source: ConnectionHistorySource = "tree") {
  selectedAssetId.value = asset.id ?? null;
  sessionStore.createSession(asset, source);
}

function editAsset(asset?: HostAsset) {
  emit("edit", asset ?? null);
}

async function toggleFavorite(asset: HostAsset) {
  if (asset.id === undefined) return;
  await assetStore.toggleFavorite(asset.id);
}

async function createFolder(parentId?: number) {
  const name = prompt("New asset folder name");
  if (!name?.trim()) return;
  await assetStore.addFolder({ name: name.trim(), parentId: parentId ?? null });
}

async function editFolder(folder: AssetFolder) {
  const name = prompt("Rename asset folder", folder.name);
  if (!name?.trim() || name === folder.name) return;
  await assetStore.updateFolder({ ...folder, name: name.trim() });
}

async function deleteFolder(folder: AssetFolder) {
  if (!folder.id) return;
  if (!window.confirm(`Delete folder "${folder.name}"?`)) return;
  await assetStore.deleteFolder(folder.id);
}

async function deleteAsset(asset: HostAsset) {
  if (!asset.id) return;
  if (!window.confirm(`Delete asset "${asset.name}"?`)) return;
  await assetStore.deleteAsset(asset.id);
  notificationStore.success(`Deleted asset ${asset.name}`);
}

function selectAsset(asset: HostAsset) {
  selectedAssetId.value = asset.id ?? null;
  if (asset.id !== undefined) {
    void assetStore.refreshOpsData(asset.id);
  }
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <div class="border-b border-border-primary bg-bg-secondary/95 px-3 py-3 backdrop-blur">
      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <div class="text-sm font-semibold text-text-primary">Asset Center</div>
          <div class="mt-1 truncate text-xs text-text-secondary">
            Host assets, environments, tags, favorites and saved views
          </div>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <button
            class="flex h-9 w-9 items-center justify-center rounded border border-border-primary bg-bg-tertiary text-text-primary transition-all hover:bg-bg-elevated"
            title="New folder"
            @click.stop="createFolder()"
          >
            <FolderPlus class="h-4 w-4" />
          </button>
          <button
            class="flex h-9 items-center gap-1.5 rounded border border-border-primary bg-bg-tertiary px-3 text-sm text-text-primary transition-all hover:bg-bg-elevated"
            @click.stop="editAsset()"
          >
            <Plus class="h-3.5 w-3.5" />
            <span class="whitespace-nowrap">New Asset</span>
          </button>
        </div>
      </div>

      <div class="mt-3">
        <div class="relative min-w-0 flex-1">
          <Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-text-secondary" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Search by host, owner, label or environment"
            class="h-9 w-full rounded border border-border-primary bg-bg-tertiary pl-8 pr-3 text-sm text-text-primary outline-none focus:border-accent"
          />
        </div>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2 text-[11px]">
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          Assets {{ assets.length }}
        </span>
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          Folders {{ folders.length }}
        </span>
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          Environments {{ environments.length }}
        </span>
        <span class="rounded-full border border-border-primary bg-bg-primary px-2.5 py-1 text-text-secondary">
          Active Sessions {{ activeSessions }}
        </span>
      </div>
    </div>

    <div v-if="assets.length === 0" class="flex-1 overflow-y-auto px-3 py-3">
      <div class="rounded-xl border border-dashed border-border-primary bg-bg-secondary/70 p-5 text-center">
        <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-bg-tertiary text-accent">
          <HardDrive class="h-6 w-6" />
        </div>
        <div class="text-sm font-medium text-text-primary">No assets yet</div>
        <div class="mt-1 text-xs leading-5 text-text-secondary">
          Create your first host asset and start using the ops workspace.
        </div>
        <div class="mt-4 flex items-center justify-center gap-2">
          <button
            class="h-9 rounded border border-border-primary bg-accent px-3 text-sm text-white transition-all hover:opacity-90"
            @click.stop="editAsset()"
          >
            Create first asset
          </button>
          <button
            class="flex h-9 items-center gap-1.5 rounded border border-border-primary bg-bg-tertiary px-3 text-sm text-text-primary transition-all hover:bg-bg-elevated"
            @click.stop="createFolder()"
          >
            <FolderPlus class="h-3.5 w-3.5" />
            <span>Create folder</span>
          </button>
        </div>
      </div>
    </div>

    <div v-else-if="isSearchMode" class="flex-1 overflow-y-auto px-3 py-3">
      <div v-if="searchResults.length === 0" class="rounded-xl border border-dashed border-border-primary bg-bg-secondary/70 p-5 text-center">
        <div class="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-bg-tertiary text-text-secondary">
          <Search class="h-5 w-5" />
        </div>
        <div class="text-sm font-medium text-text-primary">No matching assets</div>
        <div class="mt-1 text-xs text-text-secondary">
          Try another hostname, label, environment or owner.
        </div>
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="asset in searchResults"
          :key="asset.id"
          class="rounded-lg border border-border-primary bg-bg-primary px-3 py-3"
        >
          <div class="flex items-start justify-between gap-3">
            <button class="min-w-0 flex-1 text-left" @click="selectAsset(asset); connect(asset, 'search')">
              <div class="flex items-center gap-2">
                <span class="truncate text-sm text-text-primary">{{ asset.name }}</span>
                <span
                  class="rounded-full bg-bg-tertiary px-2 py-0.5 text-[11px] text-text-secondary"
                >
                  {{ asset.platform ?? "Linux" }}
                </span>
                <span
                  v-if="asset.criticality"
                  class="rounded-full px-2 py-0.5 text-[11px]"
                  :class="
                    asset.criticality === 'critical'
                      ? 'bg-error/10 text-error'
                      : asset.criticality === 'high'
                        ? 'bg-warning/10 text-warning'
                        : 'bg-bg-tertiary text-text-secondary'
                  "
                >
                  {{ asset.criticality }}
                </span>
              </div>
              <div class="mt-1 truncate text-xs text-text-secondary">{{ endpointLabel(asset) }}</div>
              <div class="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-text-secondary">
                <span v-if="asset.owner">Owner: {{ asset.owner }}</span>
                <span v-if="asset.envId && environmentMap.get(asset.envId)">Env: {{ environmentMap.get(asset.envId) }}</span>
                <span v-if="asset.healthSummary">{{ asset.healthSummary }}</span>
              </div>
            </button>
            <div class="flex items-center gap-1">
              <button class="rounded p-1 text-text-secondary hover:bg-bg-tertiary hover:text-warning" @click.stop="toggleFavorite(asset)">
                <Star class="h-3.5 w-3.5" :class="assetStore.isFavorite(asset.id ?? -1) ? 'fill-current text-warning' : ''" />
              </button>
              <button class="rounded p-1 text-text-secondary hover:bg-bg-tertiary hover:text-text-primary" @click.stop="editAsset(asset)">
                <Briefcase class="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="flex min-h-0 flex-1 flex-col">
      <div class="shrink-0 space-y-3 border-b border-border-primary bg-bg-secondary/35 px-3 py-3">
        <div class="grid grid-cols-2 gap-2 text-xs">
          <div class="rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
            <div class="flex items-center gap-2 text-text-secondary">
              <Layers3 class="h-3.5 w-3.5" />
              <span>Environments</span>
            </div>
            <div class="mt-2 text-lg font-semibold text-text-primary">{{ environments.length }}</div>
          </div>
          <div class="rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
            <div class="flex items-center gap-2 text-text-secondary">
              <Tags class="h-3.5 w-3.5" />
              <span>Tags</span>
            </div>
            <div class="mt-2 text-lg font-semibold text-text-primary">{{ tags.length }}</div>
          </div>
          <div class="rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
            <div class="flex items-center gap-2 text-text-secondary">
              <BookMarked class="h-3.5 w-3.5" />
              <span>Saved Views</span>
            </div>
            <div class="mt-2 text-lg font-semibold text-text-primary">{{ savedViews.length }}</div>
          </div>
          <div class="rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
            <div class="flex items-center gap-2 text-text-secondary">
              <ShieldAlert class="h-3.5 w-3.5" />
              <span>Critical Assets</span>
            </div>
            <div class="mt-2 text-lg font-semibold text-text-primary">
              {{ assets.filter((asset) => asset.criticality === "critical").length }}
            </div>
          </div>
        </div>

        <div v-if="favoriteAssets.length > 0" class="space-y-2 rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
          <div class="flex items-center justify-between gap-2">
            <div class="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-text-secondary">
              <Star class="h-3.5 w-3.5" />
              <span>Favorites</span>
            </div>
            <button
              v-if="favoriteAssets.length > 4"
              class="text-xs text-accent transition-colors hover:text-accent/80"
              @click="isFavoritesExpanded = !isFavoritesExpanded"
            >
              {{ isFavoritesExpanded ? "Show Less" : "Show More" }}
            </button>
          </div>

          <div class="grid gap-2">
            <div
              v-for="asset in favoritePreview"
              :key="`favorite-${asset.id}`"
              class="rounded-lg border border-border-primary bg-bg-primary px-3 py-2"
            >
              <div class="flex items-start justify-between gap-3">
                <button class="min-w-0 flex-1 text-left" @click="selectAsset(asset); connect(asset, 'quick')">
                  <div class="truncate text-sm text-text-primary">{{ asset.name }}</div>
                  <div class="mt-1 truncate text-xs text-text-secondary">{{ endpointLabel(asset) }}</div>
                </button>
                <button
                  class="rounded p-1 text-text-secondary transition-colors hover:bg-bg-tertiary hover:text-warning"
                  @click.stop="toggleFavorite(asset)"
                >
                  <Star class="h-3.5 w-3.5 fill-current text-warning" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <div class="space-y-2 rounded-xl border border-border-primary bg-bg-tertiary/70 p-3">
          <div class="flex items-center justify-between gap-2">
            <div class="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-text-secondary">
              <History class="h-3.5 w-3.5" />
              <span>Recent Access</span>
            </div>
            <button
              v-if="historyItems.length > 4"
              class="text-xs text-accent transition-colors hover:text-accent/80"
              @click="isHistoryExpanded = !isHistoryExpanded"
            >
              {{ isHistoryExpanded ? "Show Less" : "Show More" }}
            </button>
          </div>

          <div class="flex items-center gap-1 rounded-full bg-bg-primary p-1 text-[11px]">
            <button class="rounded-full px-2 py-1" :class="historyFilter === 'all' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary'" @click="historyFilter = 'all'">All</button>
            <button class="rounded-full px-2 py-1" :class="historyFilter === 'success' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary'" @click="historyFilter = 'success'">Success</button>
            <button class="rounded-full px-2 py-1" :class="historyFilter === 'failed' ? 'bg-bg-elevated text-text-primary' : 'text-text-secondary'" @click="historyFilter = 'failed'">Failed</button>
          </div>

          <div v-if="visibleHistoryItems.length === 0" class="rounded-lg border border-dashed border-border-primary bg-bg-primary px-3 py-4 text-center text-xs text-text-secondary">
            No access history yet
          </div>

          <div v-else class="space-y-2">
            <div
              v-for="item in visibleHistoryItems"
              :key="`${item.asset.id}-${item.entry.connectedAt}`"
              class="rounded-lg border border-border-primary bg-bg-primary px-3 py-2"
            >
              <div class="flex items-start justify-between gap-3">
                <button class="min-w-0 flex-1 text-left" @click="selectAsset(item.asset); connect(item.asset, 'history')">
                  <div class="flex items-center gap-2">
                    <span class="min-w-0 flex-1 truncate text-sm text-text-primary">{{ item.asset.name }}</span>
                    <span class="rounded-full bg-bg-tertiary px-2 py-0.5 text-[11px] text-text-secondary">
                      {{ sourceLabel(item.entry.source === "tree" ? "history" : item.entry.source) }}
                    </span>
                    <span class="shrink-0 text-[11px] text-text-secondary">{{ formatRecentTime(item.entry.connectedAt) }}</span>
                  </div>
                  <div class="mt-1 truncate text-xs text-text-secondary">{{ endpointLabel(item.asset) }}</div>
                  <div v-if="item.entry.reason" class="mt-1 truncate text-[11px]" :class="item.entry.status === 'failed' ? 'text-error' : 'text-text-secondary'">
                    {{ item.entry.reason }}
                  </div>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="min-h-0 flex-1 px-3 py-3">
        <div class="grid h-full min-h-0 gap-3 lg:grid-cols-[minmax(0,1.25fr)_minmax(280px,0.9fr)]">
          <div class="flex min-h-0 flex-col rounded-xl border border-border-primary bg-bg-secondary/40">
          <div class="flex items-center justify-between gap-3 border-b border-border-primary px-3 py-2.5 text-xs">
            <div class="flex items-center gap-1.5 font-medium uppercase tracking-wide text-text-secondary">
              <FolderTree class="h-3.5 w-3.5" />
              <span>Asset Directory</span>
            </div>
            <span class="text-text-secondary">Folders group your host assets and access paths</span>
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto px-2 py-2">
            <div class="min-h-[50px] space-y-0.5">
              <ConnectionTreeItem
                v-for="item in treeData"
                :key="`${'children' in item ? 'folder' : 'asset'}-${item.id}`"
                :item="item"
                :level="1"
                @connect="
                  (asset) => {
                    selectAsset(asset);
                    connect(asset);
                  }
                "
                @edit="editAsset"
                @delete="deleteAsset"
                @create-group="createFolder"
                @edit-group="editFolder"
                @delete-group="deleteFolder"
                @context-menu="() => {}"
              />
            </div>
          </div>
        </div>

          <div class="flex min-h-0 flex-col rounded-xl border border-border-primary bg-bg-secondary/40">
            <div class="border-b border-border-primary px-4 py-3">
              <div class="text-xs font-medium uppercase tracking-wide text-text-secondary">
                Asset Detail
              </div>
              <div v-if="selectedAsset" class="mt-2">
                <div class="text-sm font-semibold text-text-primary">
                  {{ selectedAsset.name }}
                </div>
                <div class="mt-1 text-xs text-text-secondary">
                  {{ selectedEndpoint ? `${selectedEndpoint.username}@${selectedEndpoint.host}` : selectedAsset.host }}
                </div>
              </div>
            </div>

            <div v-if="selectedAsset" class="min-h-0 flex-1 overflow-y-auto px-4 py-4">
              <div class="space-y-4">
                <div class="grid grid-cols-2 gap-2 text-xs">
                  <div class="rounded-lg border border-border-primary bg-bg-primary px-3 py-2">
                    <div class="text-text-secondary">Environment</div>
                    <div class="mt-1 text-text-primary">
                      {{ (selectedAsset.envId && environmentMap.get(selectedAsset.envId)) || "Unassigned" }}
                    </div>
                  </div>
                  <div class="rounded-lg border border-border-primary bg-bg-primary px-3 py-2">
                    <div class="text-text-secondary">Risk</div>
                    <div class="mt-1 text-text-primary">{{ selectedAsset.criticality }}</div>
                  </div>
                </div>

                <div class="rounded-lg border border-border-primary bg-bg-primary px-3 py-3 text-xs text-text-secondary">
                  <div class="font-medium text-text-primary">Workspace</div>
                  <div class="mt-1">
                    {{ selectedAsset.defaultWorkspacePath || "No default workspace configured" }}
                  </div>
                </div>

                <div class="rounded-lg border border-border-primary bg-bg-primary px-3 py-3 text-xs text-text-secondary">
                  <div class="font-medium text-text-primary">Default Endpoint</div>
                  <div class="mt-1">
                    {{ selectedEndpoint ? `${selectedEndpoint.username}@${selectedEndpoint.host}:${selectedEndpoint.port}` : "No access endpoint configured" }}
                  </div>
                  <div v-if="selectedEndpoint?.jumpHost" class="mt-1">
                    Jump via {{ selectedEndpoint.jumpUsername || "user" }}@{{ selectedEndpoint.jumpHost }}:{{ selectedEndpoint.jumpPort || 22 }}
                  </div>
                </div>

                <div class="flex flex-wrap gap-2">
                  <button
                    class="rounded border border-border-primary bg-accent px-3 py-2 text-sm text-white hover:opacity-90"
                    @click="selectAsset(selectedAsset); connect(selectedAsset, 'quick')"
                  >
                    Open Terminal
                  </button>
                  <button
                    class="rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary hover:bg-bg-elevated"
                    @click="
                      selectAsset(selectedAsset);
                      connect(selectedAsset, 'quick');
                      sessionStore.setActiveTab('files');
                    "
                  >
                    Open Files
                  </button>
                  <button
                    class="inline-flex items-center gap-1 rounded border border-border-primary bg-bg-tertiary px-3 py-2 text-sm text-text-primary hover:bg-bg-elevated"
                    @click="emit('tunnels', selectedAsset)"
                  >
                    <Cable class="h-3.5 w-3.5" />
                    <span>Manage Tunnels</span>
                  </button>
                </div>

                <div class="rounded-lg border border-border-primary bg-bg-primary px-3 py-3 text-xs text-text-secondary">
                  <div class="font-medium text-text-primary">Recent Audit</div>
                  <div v-if="selectedAuditEvents.length === 0" class="mt-2">
                    No recent audit events
                  </div>
                  <div v-else class="mt-2 space-y-2">
                    <div
                      v-for="event in selectedAuditEvents"
                      :key="event.id"
                      class="rounded border border-border-primary bg-bg-secondary px-2 py-2"
                    >
                      <div class="text-text-primary">{{ event.title }}</div>
                      <div class="mt-1 text-[11px] text-text-secondary">
                        {{ event.detail || event.eventType }}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div v-else class="flex min-h-0 flex-1 items-center justify-center px-6 text-center">
              <div>
                <div class="text-sm font-medium text-text-primary">Select an asset</div>
                <div class="mt-1 text-xs text-text-secondary">
                  Review endpoint, workspace, audit and quick actions here.
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
