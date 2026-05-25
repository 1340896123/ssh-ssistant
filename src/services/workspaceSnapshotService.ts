import { invoke } from "@tauri-apps/api/core";
import type { LocalWorkspaceSnapshot } from "../types";

export const workspaceSnapshotService = {
  get: (snapshotKey: string) =>
    invoke<LocalWorkspaceSnapshot | null>("get_local_workspace_snapshot", {
      snapshotKey,
    }),
  save: (snapshotKey: string, snapshot: LocalWorkspaceSnapshot) =>
    invoke("save_local_workspace_snapshot", {
      snapshotKey,
      snapshot,
    }),
};
