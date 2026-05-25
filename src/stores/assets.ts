import { defineStore } from "pinia";
import {
  accessService,
  assetService,
  auditService,
  cloudService,
  opsService,
  syncService,
} from "../services";
import type {
  AccessEndpoint,
  AssetAccessHistoryEntry,
  AssetFolder,
  AssetTag,
  AssetUpsertPayload,
  AuditEvent,
  CloudAssetRecord,
  ConnectionHistoryEntry,
  ConnectionHistorySource,
  CredentialRef,
  Environment,
  HostAsset,
  JobBatchPreview,
  JobBatchRequest,
  JobBatchResult,
  JobRun,
  JobRunArchive,
  JobTemplate,
  OpsConsoleAnswer,
  SavedAssetView,
  SyncChangeLogEntry,
  SyncOverview,
  SyncServiceConfig,
  SyncState,
} from "../types";

const LEGACY_ASSET_FAVORITES_KEY = "asset-favorites";
const LEGACY_ASSET_HISTORY_KEY = "asset-history";
const LEGACY_CONNECTION_FAVORITES_KEY = "connection-favorites";
const LEGACY_CONNECTION_HISTORY_KEY = "connection-history";

function canUseStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function readNumberArrayStorage(key: string): number[] {
  if (!canUseStorage()) return [];
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((value): value is number => typeof value === "number")
      : [];
  } catch {
    return [];
  }
}

function readLegacyHistoryStorage(key: string): AssetAccessHistoryEntry[] {
  if (!canUseStorage()) return [];
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed
          .filter((item): item is ConnectionHistoryEntry => {
            return (
              typeof item?.connectionId === "number" &&
              typeof item?.connectedAt === "number" &&
              typeof item?.status === "string" &&
              typeof item?.source === "string"
            );
          })
          .map((item) => ({
            assetId: item.connectionId,
            connectedAt: item.connectedAt,
            status: item.status,
            reason: item.reason,
            source: item.source,
          }))
      : [];
  } catch {
    return [];
  }
}

function clearLegacyClientStateStorage() {
  if (!canUseStorage()) return;
  for (const key of [
    LEGACY_ASSET_FAVORITES_KEY,
    LEGACY_ASSET_HISTORY_KEY,
    LEGACY_CONNECTION_FAVORITES_KEY,
    LEGACY_CONNECTION_HISTORY_KEY,
  ]) {
    window.localStorage.removeItem(key);
  }
}

function readLegacyFavoriteIds(): number[] {
  return Array.from(
    new Set([
      ...readNumberArrayStorage(LEGACY_ASSET_FAVORITES_KEY),
      ...readNumberArrayStorage(LEGACY_CONNECTION_FAVORITES_KEY),
    ]),
  );
}

function readLegacyHistoryEntries(): AssetAccessHistoryEntry[] {
  return [
    ...readLegacyHistoryStorage(LEGACY_ASSET_HISTORY_KEY),
    ...readLegacyHistoryStorage(LEGACY_CONNECTION_HISTORY_KEY),
  ];
}

function mapHistorySource(raw?: string): ConnectionHistorySource {
  if (raw === "quick" || raw === "history" || raw === "tree" || raw === "search") {
    return raw;
  }
  return "tree";
}

function mapHistoryStatus(raw?: string): "success" | "failed" {
  return raw === "failed" ? "failed" : "success";
}

function dedupeAccessHistory(
  entries: AssetAccessHistoryEntry[],
): AssetAccessHistoryEntry[] {
  const latestEntries = new Map<number, AssetAccessHistoryEntry>();

  for (const entry of [...entries].sort((a, b) => b.connectedAt - a.connectedAt)) {
    if (!latestEntries.has(entry.assetId)) {
      latestEntries.set(entry.assetId, entry);
    }
  }

  return Array.from(latestEntries.values()).sort(
    (a, b) => b.connectedAt - a.connectedAt,
  );
}

export const useAssetStore = defineStore("assets", {
  state: () => ({
    assets: [] as HostAsset[],
    folders: [] as AssetFolder[],
    environments: [] as Environment[],
    tags: [] as AssetTag[],
    savedViews: [] as SavedAssetView[],
    accessEndpoints: [] as AccessEndpoint[],
    credentialRefs: [] as CredentialRef[],
    jobTemplates: [] as JobTemplate[],
    jobRuns: [] as JobRun[],
    jobArchives: [] as JobRunArchive[],
    auditEvents: [] as AuditEvent[],
    syncState: null as SyncState | null,
    syncOverview: null as SyncOverview | null,
    syncChanges: [] as SyncChangeLogEntry[],
    syncServices: [] as SyncServiceConfig[],
    accessHistory: [] as AssetAccessHistoryEntry[],
    lastOpsConsoleAnswer: null as OpsConsoleAnswer | null,
    lastJobBatchPreview: null as JobBatchPreview | null,
    lastJobBatchResult: null as JobBatchResult | null,
    hasImportedLegacyClientState: false,
  }),
  getters: {
    treeData: (state) => {
      const buildTree = (parentId: number | null): (AssetFolder | HostAsset)[] => {
        const result: (AssetFolder | HostAsset)[] = [];
        const childFolders = state.folders.filter(
          (folder) => (folder.parentId ?? null) === parentId,
        );
        childFolders.forEach((folder) => {
          const children = buildTree(folder.id ?? null);
          result.push({ ...folder, children });
        });
        const childAssets = state.assets.filter(
          (asset) => (asset.folderId ?? asset.groupId ?? null) === parentId,
        );
        childAssets.forEach((asset) => result.push(asset));
        return result;
      };
      return buildTree(null);
    },
    favoriteAssets: (state) =>
      state.assets.filter((asset) => asset.id !== undefined && Boolean(asset.isFavorite)),
    favorites: (state) =>
      state.assets
        .filter((asset) => asset.id !== undefined && Boolean(asset.isFavorite))
        .map((asset) => asset.id as number),
    historyEntries: (state) =>
      dedupeAccessHistory(state.accessHistory)
        .filter((entry) =>
          state.assets.some((asset) => asset.id === entry.assetId),
        )
        .map(
          (entry): ConnectionHistoryEntry => ({
            connectionId: entry.assetId,
            connectedAt: entry.connectedAt,
            status: entry.status,
            reason: entry.reason,
            source: entry.source,
          }),
        )
        .sort((a, b) => b.connectedAt - a.connectedAt),
    environmentMap: (state) =>
      new Map(state.environments.map((environment) => [environment.id ?? -1, environment])),
    assetMap: (state) => new Map(state.assets.map((asset) => [asset.id ?? -1, asset])),
  },
  actions: {
    async importLegacyClientStateIfNeeded() {
      if (this.hasImportedLegacyClientState) return;

      const favoriteAssetIds = readLegacyFavoriteIds();
      const historyEntries = readLegacyHistoryEntries();
      if (favoriteAssetIds.length === 0 && historyEntries.length === 0) {
        this.hasImportedLegacyClientState = true;
        return;
      }

      await assetService.importLegacyClientState(favoriteAssetIds, historyEntries);
      clearLegacyClientStateStorage();
      this.hasImportedLegacyClientState = true;
    },
    async loadAssets() {
      await this.importLegacyClientStateIfNeeded();

      const [
        assets,
        folders,
        environments,
        tags,
        savedViews,
        accessEndpoints,
        credentialRefs,
        accessHistory,
        syncState,
      ] = await Promise.all([
        assetService.list(),
        assetService.listFolders(),
        assetService.listEnvironments(),
        assetService.listTags(),
        assetService.listSavedViews(),
        accessService.listEndpoints(),
        accessService.listCredentialRefs(),
        assetService.listAccessHistory(undefined, 200),
        syncService.getState().catch(() => null),
      ]);

      this.assets = assets.map((asset) => ({
        ...asset,
        platform: asset.platform ?? "Linux",
        folderId: asset.folderId ?? asset.groupId ?? null,
        labels: asset.labels ?? [],
        owner: asset.owner ?? "",
        criticality: asset.criticality ?? "medium",
        isFavorite: Boolean(asset.isFavorite),
      }));
      this.folders = folders;
      this.environments = environments;
      this.tags = tags;
      this.savedViews = savedViews;
      this.accessEndpoints = accessEndpoints;
      this.credentialRefs = credentialRefs;
      this.accessHistory = accessHistory.map((entry) => ({
        assetId: entry.assetId,
        connectedAt: entry.connectedAt,
        status: mapHistoryStatus(entry.status),
        reason: entry.reason,
        source: mapHistorySource(entry.source),
      }));
      this.syncState = syncState;
    },
    defaultAccessEndpointForAsset(assetId?: number) {
      if (assetId === undefined) return null;
      return (
        this.accessEndpoints.find(
          (endpoint) =>
            endpoint.assetId === assetId &&
            this.assets.find((asset) => asset.id === assetId)?.accessEndpointId ===
              endpoint.id,
        ) ??
        this.accessEndpoints.find((endpoint) => endpoint.assetId === assetId) ??
        null
      );
    },
    credentialRefById(credentialRefId?: number | null) {
      if (credentialRefId == null) return null;
      return this.credentialRefs.find((item) => item.id === credentialRefId) ?? null;
    },
    buildAssetPayload(
      asset: HostAsset,
      endpoint?: AccessEndpoint | null,
      credentialRef?: CredentialRef | null,
    ): AssetUpsertPayload {
      const assetId = asset.id ?? endpoint?.assetId ?? 0;
      const nextEndpoint: AccessEndpoint = {
        id: endpoint?.id ?? asset.accessEndpointId ?? undefined,
        assetId,
        name: endpoint?.name ?? `${asset.name} default endpoint`,
        host: endpoint?.host ?? asset.host,
        port: endpoint?.port ?? asset.port,
        username: endpoint?.username ?? credentialRef?.username ?? "root",
        authType:
          endpoint?.authType ??
          (credentialRef?.credentialKind === "sshKey" ? "key" : "password"),
        credentialRefId: credentialRef?.id ?? endpoint?.credentialRefId ?? null,
        sshKeyId: endpoint?.sshKeyId ?? credentialRef?.sshKeyId ?? null,
        jumpHost: endpoint?.jumpHost ?? null,
        jumpPort: endpoint?.jumpPort ?? null,
        jumpUsername: endpoint?.jumpUsername ?? null,
        jumpPassword: null,
      };

      const nextCredentialRef =
        credentialRef && (credentialRef.secret || credentialRef.sshKeyId || credentialRef.id)
          ? {
              ...credentialRef,
              assetId: asset.id ?? credentialRef.assetId ?? null,
              updatedAt: Date.now(),
            }
          : null;

      return {
        asset: {
          ...asset,
          folderId: asset.folderId ?? asset.groupId ?? null,
          groupId: asset.folderId ?? asset.groupId ?? null,
          labels: asset.labels ?? [],
          criticality: asset.criticality ?? "medium",
          platform: asset.platform ?? "Linux",
          owner: asset.owner?.trim() || "",
        },
        defaultAccessEndpoint: nextEndpoint,
        defaultCredentialRef: nextCredentialRef,
      };
    },
    async refreshOpsData(assetId?: number) {
      const [jobTemplates, jobRuns, jobArchives, auditEvents, accessHistory] =
        await Promise.all([
        opsService.listJobTemplates(),
        opsService.listJobRuns(assetId),
        opsService.listJobArchives(assetId, assetId ? 40 : 120),
        auditService.list(assetId),
        assetService.listAccessHistory(assetId, assetId ? 40 : 200),
        ]);
      this.jobTemplates = jobTemplates;
      this.jobRuns = jobRuns;
      this.jobArchives = jobArchives;
      this.auditEvents = auditEvents;
      if (assetId === undefined) {
        this.accessHistory = accessHistory.map((entry) => ({
          assetId: entry.assetId,
          connectedAt: entry.connectedAt,
          status: mapHistoryStatus(entry.status),
          reason: entry.reason,
          source: mapHistorySource(entry.source),
        }));
      }
    },
    async loadSyncOverview() {
      const [overview, services, changes] = await Promise.all([
        syncService.getOverview(),
        syncService.listServices(),
        syncService.listChangeLog(undefined, undefined, 100),
      ]);
      this.syncOverview = overview;
      this.syncState = overview.state;
      this.syncServices = services;
      this.syncChanges = changes;
      return overview;
    },
    async addAsset(
      asset: HostAsset,
      endpoint?: AccessEndpoint | null,
      credentialRef?: CredentialRef | null,
    ) {
      const created = await assetService.create(
        this.buildAssetPayload(asset, endpoint, credentialRef),
      );
      await this.loadAssets();
      return created;
    },
    async updateAsset(
      asset: HostAsset,
      endpoint?: AccessEndpoint | null,
      credentialRef?: CredentialRef | null,
    ) {
      const updated = await assetService.update(
        this.buildAssetPayload(
          asset,
          endpoint ?? this.defaultAccessEndpointForAsset(asset.id),
          credentialRef ??
            this.credentialRefById(
              endpoint?.credentialRefId ??
                this.defaultAccessEndpointForAsset(asset.id)?.credentialRefId ??
                null,
            ),
        ),
      );
      await this.loadAssets();
      return updated;
    },
    async deleteAsset(id: number) {
      await assetService.remove(id);
      await this.loadAssets();
    },
    async addFolder(folder: AssetFolder) {
      await assetService.createFolder(folder);
      await this.loadAssets();
    },
    async updateFolder(folder: AssetFolder) {
      await assetService.updateFolder(folder);
      await this.loadAssets();
    },
    async deleteFolder(id: number) {
      await assetService.removeFolder(id);
      await this.loadAssets();
    },
    async addEnvironment(environment: Environment) {
      await assetService.createEnvironment(environment);
      await this.loadAssets();
    },
    async updateEnvironment(environment: Environment) {
      await assetService.updateEnvironment(environment);
      await this.loadAssets();
    },
    async deleteEnvironment(id: number) {
      await assetService.removeEnvironment(id);
      await this.loadAssets();
    },
    async addTag(tag: AssetTag) {
      await assetService.createTag(tag);
      await this.loadAssets();
    },
    async deleteTag(id: number) {
      await assetService.removeTag(id);
      await this.loadAssets();
    },
    async addSavedView(view: SavedAssetView) {
      const created = await assetService.createSavedView(view);
      await this.loadAssets();
      return created;
    },
    async deleteSavedView(id: number) {
      await assetService.removeSavedView(id);
      await this.loadAssets();
    },
    async toggleFavorite(assetId: number) {
      const asset = this.assets.find((item) => item.id === assetId);
      const nextFavorite = !asset?.isFavorite;
      await assetService.toggleFavorite(assetId, nextFavorite);
      await this.loadAssets();
    },
    isFavorite(assetId: number) {
      return Boolean(this.assets.find((asset) => asset.id === assetId)?.isFavorite);
    },
    addSuccessfulConnection(
      connectionId: number,
      source: ConnectionHistorySource = "tree",
    ) {
      const existing = this.accessHistory.filter((entry) => entry.assetId !== connectionId);
      this.accessHistory = [
        {
          assetId: connectionId,
          connectedAt: Date.now(),
          status: "success",
          source,
        },
        ...existing,
      ];
    },
    addFailedConnection(
      connectionId: number,
      reason?: string,
      source: ConnectionHistorySource = "tree",
    ) {
      const existing = this.accessHistory.filter((entry) => entry.assetId !== connectionId);
      this.accessHistory = [
        {
          assetId: connectionId,
          connectedAt: Date.now(),
          status: "failed",
          reason,
          source,
        },
        ...existing,
      ];
    },
    async touchAsset(id: number) {
      await assetService.touch(id);
      await this.loadAssets();
    },
    async executeJob(
      sessionId: string,
      commandText: string,
      assetId?: number,
      riskLevel?: string,
      source?: string,
    ) {
      const run = await opsService.executeJob(
        sessionId,
        commandText,
        assetId,
        riskLevel,
        source,
      );
      await this.refreshOpsData(assetId);
      return run;
    },
    async previewJobBatch(request: JobBatchRequest) {
      this.lastJobBatchPreview = await opsService.previewJobBatch(request);
      return this.lastJobBatchPreview;
    },
    async executeJobBatch(request: JobBatchRequest) {
      this.lastJobBatchResult = await opsService.executeJobBatch(request);
      await Promise.all([this.refreshOpsData(), this.loadSyncOverview()]);
      return this.lastJobBatchResult;
    },
    async runOpsConsoleQuery(query: string, selectedAssetId?: number | null) {
      this.lastOpsConsoleAnswer = await opsService.opsConsoleQuery(
        query,
        selectedAssetId ?? null,
      );
      return this.lastOpsConsoleAnswer;
    },
    async createJobTemplate(template: JobTemplate) {
      const created = await opsService.createJobTemplate(template);
      await Promise.all([this.refreshOpsData(), this.loadSyncOverview()]);
      return created;
    },
    async deleteJobTemplate(id: number) {
      await opsService.removeJobTemplate(id);
      await Promise.all([this.refreshOpsData(), this.loadSyncOverview()]);
    },
    async appendAuditEvent(event: AuditEvent) {
      const created = await auditService.create(event);
      await this.refreshOpsData(event.assetId ?? undefined);
      return created;
    },
    async saveSyncState(state: SyncState) {
      this.syncState = await syncService.saveState(state);
      await this.loadSyncOverview();
      return this.syncState;
    },
    async markSyncChangesSynced(changeIds: number[], serviceKey?: string | null) {
      const updated = await syncService.markChangesSynced(changeIds, serviceKey);
      await this.loadSyncOverview();
      return updated;
    },
    async saveSyncService(service: SyncServiceConfig) {
      const saved = await syncService.upsertService(service);
      await this.loadSyncOverview();
      return saved;
    },
    async searchAuditEvents(
      query?: string,
      severity?: string,
      assetId?: number,
      limit?: number,
    ) {
      this.auditEvents = await auditService.search(query, severity, assetId, limit);
      return this.auditEvents;
    },
    buildCloudAssetRecords(): CloudAssetRecord[] {
      return this.assets.map((asset) => ({
        asset,
        defaultAccessEndpoint: this.defaultAccessEndpointForAsset(asset.id) ?? {
          assetId: asset.id ?? 0,
          name: `${asset.name} default endpoint`,
          host: asset.host,
          port: asset.port,
          username: "root",
          authType: "password",
          credentialRefId: null,
          sshKeyId: null,
          jumpHost: null,
          jumpPort: null,
          jumpUsername: null,
          jumpPassword: null,
        },
        defaultCredentialRef: this.credentialRefById(
          this.defaultAccessEndpointForAsset(asset.id)?.credentialRefId ?? null,
        ),
      }));
    },
    async syncAssetsToCloud(
      baseUrl: string,
      mode: string,
      accountKey: string,
      accessToken: string,
      onUnauthorized?: () => Promise<void> | void,
    ) {
      let response;
      try {
        response = await cloudService.syncAssets(baseUrl, {
          mode,
          accountKey,
          accessToken,
          assetsJson: JSON.stringify(this.buildCloudAssetRecords()),
        });
      } catch (error) {
        if (String(error).includes("401") && onUnauthorized) {
          await onUnauthorized();
        }
        throw error;
      }

      const parsed = JSON.parse(response.assetsJson || "[]") as CloudAssetRecord[];
      if (parsed.length > 0) {
        await assetService.importCloudRecords(parsed, true);
        await this.loadAssets();
      }

      return response;
    },
    async pullAssetsFromCloud(
      baseUrl: string,
      mode: string,
      accountKey: string,
      accessToken: string,
    ) {
      const response = await cloudService.pull(baseUrl, mode, accountKey, accessToken);
      const parsed = JSON.parse(response.assetsJson || "[]") as CloudAssetRecord[];
      await assetService.importCloudRecords(parsed, true);
      await this.loadAssets();
      return response;
    },
  },
});
