import { invoke } from "@tauri-apps/api/core";
import type {
  AssetAccessHistoryEntry,
  AssetUpsertPayload,
  AssetFolder,
  AssetTag,
  Environment,
  HostAsset,
  SavedAssetView,
} from "../types";

export const assetService = {
  list: () => invoke<HostAsset[]>("asset_get_host_assets"),
  search: (query: string) =>
    invoke<HostAsset[]>("asset_search_host_assets", { query }),
  create: (payload: AssetUpsertPayload) =>
    invoke<HostAsset>("asset_create_host_asset", { payload }),
  update: (payload: AssetUpsertPayload) =>
    invoke<HostAsset>("asset_update_host_asset", { payload }),
  remove: (id: number) => invoke("asset_delete_host_asset", { id }),
  touch: (id: number) => invoke("asset_touch_host_asset", { id }),
  toggleFavorite: (id: number, isFavorite: boolean) =>
    invoke("asset_toggle_favorite", { id, isFavorite }),
  listFolders: () => invoke<AssetFolder[]>("asset_get_asset_folders"),
  createFolder: (folder: AssetFolder) =>
    invoke<AssetFolder>("asset_create_asset_folder", { folder }),
  updateFolder: (folder: AssetFolder) =>
    invoke("asset_update_asset_folder", { folder }),
  removeFolder: (id: number) => invoke("asset_delete_asset_folder", { id }),
  listEnvironments: () => invoke<Environment[]>("asset_get_environments"),
  createEnvironment: (environment: Environment) =>
    invoke<Environment>("asset_create_environment", { environment }),
  updateEnvironment: (environment: Environment) =>
    invoke("asset_update_environment", { environment }),
  removeEnvironment: (id: number) => invoke("asset_delete_environment", { id }),
  listTags: () => invoke<AssetTag[]>("asset_get_asset_tags"),
  createTag: (tag: AssetTag) => invoke<AssetTag>("asset_create_asset_tag", { tag }),
  removeTag: (id: number) => invoke("asset_delete_asset_tag", { id }),
  listSavedViews: () => invoke<SavedAssetView[]>("asset_get_saved_views"),
  listAccessHistory: (assetId?: number, limit?: number) =>
    invoke<AssetAccessHistoryEntry[]>("asset_get_access_history", { assetId, limit }),
  createSavedView: (view: SavedAssetView) =>
    invoke<SavedAssetView>("asset_create_saved_view", { view }),
  removeSavedView: (id: number) => invoke("asset_delete_saved_view", { id }),
  importLegacyClientState: (
    favoriteAssetIds: number[],
    historyEntries: AssetAccessHistoryEntry[],
  ) =>
    invoke("asset_import_legacy_client_state", {
      favoriteAssetIds,
      historyEntries,
    }),
};
