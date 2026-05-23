import { invoke } from "@tauri-apps/api/core";
import type {
  SyncChangeLogEntry,
  SyncOverview,
  SyncServiceConfig,
  SyncState,
} from "../types";

export const syncService = {
  getState: () => invoke<SyncState>("sync_get_state"),
  getOverview: () => invoke<SyncOverview>("sync_get_overview"),
  saveState: (state: SyncState) =>
    invoke<SyncState>("sync_save_state", { state }),
  listChangeLog: (status?: string, objectType?: string, limit?: number) =>
    invoke<SyncChangeLogEntry[]>("sync_list_change_log", {
      status,
      objectType,
      limit,
    }),
  markChangesSynced: (changeIds: number[], serviceKey?: string | null) =>
    invoke<number>("sync_mark_changes_synced", { changeIds, serviceKey }),
  listServices: () => invoke<SyncServiceConfig[]>("sync_list_services"),
  upsertService: (service: SyncServiceConfig) =>
    invoke<SyncServiceConfig>("sync_upsert_service", { service }),
};
