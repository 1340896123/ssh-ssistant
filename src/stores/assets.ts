import { defineStore } from "pinia";
import {
  accessService,
  assetService,
  auditService,
  opsService,
  syncService,
} from "../services";
import type {
  AccessEndpoint,
  AssetFolder,
  AssetTag,
  AssetUpsertPayload,
  AuditEvent,
  ConnectionHistoryEntry,
  ConnectionHistorySource,
  ConnectionHistoryStatus,
  CredentialRef,
  Environment,
  HostAsset,
  JobRun,
  JobTemplate,
  SavedAssetView,
  SyncState,
} from "../types";

const FAVORITES_STORAGE_KEY = "asset-favorites";
const HISTORY_STORAGE_KEY = "asset-history";
const MAX_HISTORY_ITEMS = 40;

function canUseStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function readFavorites(): number[] {
  if (!canUseStorage()) return [];
  try {
    const raw = window.localStorage.getItem(FAVORITES_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((value): value is number => typeof value === "number")
      : [];
  } catch {
    return [];
  }
}

function readHistory(): ConnectionHistoryEntry[] {
  if (!canUseStorage()) return [];
  try {
    const raw = window.localStorage.getItem(HISTORY_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((item): item is ConnectionHistoryEntry => {
          return (
            typeof item?.connectionId === "number" &&
            typeof item?.connectedAt === "number" &&
            typeof item?.status === "string" &&
            typeof item?.source === "string"
          );
        })
      : [];
  } catch {
    return [];
  }
}

function writeFavorites(favorites: number[]) {
  if (!canUseStorage()) return;
  window.localStorage.setItem(FAVORITES_STORAGE_KEY, JSON.stringify(favorites));
}

function writeHistory(history: ConnectionHistoryEntry[]) {
  if (!canUseStorage()) return;
  window.localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(history));
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
    auditEvents: [] as AuditEvent[],
    syncState: null as SyncState | null,
    favorites: readFavorites() as number[],
    history: readHistory() as ConnectionHistoryEntry[],
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
      state.assets.filter(
        (asset) => asset.id !== undefined && state.favorites.includes(asset.id),
      ),
    historyEntries: (state) =>
      state.history
        .filter((entry) =>
          state.assets.some((asset) => asset.id === entry.connectionId),
        )
        .sort((a, b) => b.connectedAt - a.connectedAt),
    environmentMap: (state) =>
      new Map(state.environments.map((environment) => [environment.id ?? -1, environment])),
    assetMap: (state) => new Map(state.assets.map((asset) => [asset.id ?? -1, asset])),
  },
  actions: {
    async loadAssets() {
      const [
        assets,
        folders,
        environments,
        tags,
        savedViews,
        accessEndpoints,
        credentialRefs,
        syncState,
      ] = await Promise.all([
        assetService.list(),
        assetService.listFolders(),
        assetService.listEnvironments(),
        assetService.listTags(),
        assetService.listSavedViews(),
        accessService.listEndpoints(),
        accessService.listCredentialRefs(),
        syncService.getState().catch(() => null),
      ]);

      this.assets = assets.map((asset) => ({
        ...asset,
        platform: asset.platform ?? "Linux",
        folderId: asset.folderId ?? asset.groupId ?? null,
        labels: asset.labels ?? [],
        criticality: asset.criticality ?? "medium",
      }));
      this.folders = folders;
      this.environments = environments;
      this.tags = tags;
      this.savedViews = savedViews;
      this.accessEndpoints = accessEndpoints;
      this.credentialRefs = credentialRefs;
      this.syncState = syncState;
      this.favorites = this.favorites.filter((id) =>
        this.assets.some((asset) => asset.id === id),
      );
      this.history = this.history.filter((entry) =>
        this.assets.some((asset) => asset.id === entry.connectionId),
      );
      writeFavorites(this.favorites);
      writeHistory(this.history);
    },
    defaultAccessEndpointForAsset(assetId?: number) {
      if (assetId === undefined) return null;
      return (
        this.accessEndpoints.find(
          (endpoint) =>
            endpoint.assetId === assetId &&
            this.assets.find((asset) => asset.id === assetId)?.accessEndpointId === endpoint.id,
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
        username: endpoint?.username ?? credentialRef?.username ?? asset.owner ?? "root",
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
        },
        defaultAccessEndpoint: nextEndpoint,
        defaultCredentialRef: nextCredentialRef,
      };
    },
    async refreshOpsData(assetId?: number) {
      const [jobTemplates, jobRuns, auditEvents] = await Promise.all([
        opsService.listJobTemplates(),
        opsService.listJobRuns(assetId),
        auditService.list(assetId),
      ]);
      this.jobTemplates = jobTemplates;
      this.jobRuns = jobRuns;
      this.auditEvents = auditEvents;
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
      this.favorites = this.favorites.filter((favoriteId) => favoriteId !== id);
      this.history = this.history.filter((entry) => entry.connectionId !== id);
      writeFavorites(this.favorites);
      writeHistory(this.history);
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
      const nextFavorites = this.favorites.includes(assetId)
        ? this.favorites.filter((id) => id !== assetId)
        : [assetId, ...this.favorites].slice(0, 12);
      const isFavorite = nextFavorites.includes(assetId);
      await assetService.toggleFavorite(assetId, isFavorite);
      this.favorites = nextFavorites;
      writeFavorites(this.favorites);
      await this.loadAssets();
    },
    isFavorite(assetId: number) {
      return this.favorites.includes(assetId);
    },
    recordHistory(entry: ConnectionHistoryEntry) {
      this.history = [entry, ...this.history]
        .sort((a, b) => b.connectedAt - a.connectedAt)
        .slice(0, MAX_HISTORY_ITEMS);
      writeHistory(this.history);
    },
    addSuccessfulConnection(
      connectionId: number,
      source: ConnectionHistorySource = "tree",
    ) {
      this.recordHistory({
        connectionId,
        connectedAt: Date.now(),
        status: "success",
        source,
      });
    },
    addFailedConnection(
      connectionId: number,
      reason?: string,
      source: ConnectionHistorySource = "tree",
    ) {
      this.recordHistory({
        connectionId,
        connectedAt: Date.now(),
        status: "failed" as ConnectionHistoryStatus,
        reason,
        source,
      });
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
    async createJobTemplate(template: JobTemplate) {
      const created = await opsService.createJobTemplate(template);
      await this.refreshOpsData();
      return created;
    },
    async deleteJobTemplate(id: number) {
      await opsService.removeJobTemplate(id);
      await this.refreshOpsData();
    },
    async appendAuditEvent(event: AuditEvent) {
      const created = await auditService.create(event);
      await this.refreshOpsData(event.assetId ?? undefined);
      return created;
    },
    async saveSyncState(state: SyncState) {
      this.syncState = await syncService.saveState(state);
      return this.syncState;
    },
  },
});
