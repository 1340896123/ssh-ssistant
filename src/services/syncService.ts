import { invoke } from "@tauri-apps/api/core";
import type { SyncState } from "../types";

export const syncService = {
  getState: () => invoke<SyncState>("sync_get_state"),
  saveState: (state: SyncState) =>
    invoke<SyncState>("sync_save_state", { state }),
};
