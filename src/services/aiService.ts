import { invoke } from "@tauri-apps/api/core";
import type { HostAsset, OpsConsoleAnswer } from "../types";

export const aiService = {
  opsConsoleQuery: (query: string, selectedAssetId?: number | null) =>
    invoke<OpsConsoleAnswer>("ops_console_query", { query, selectedAssetId }),
  planAction: (asset: HostAsset, userRequest: string) =>
    invoke<string>("ai_plan_action", { asset, userRequest }),
  explainState: (asset: HostAsset, observedState: string) =>
    invoke<string>("ai_explain_state", { asset, observedState }),
  generateRunbook: (asset: HostAsset, target: string) =>
    invoke<string>("ai_generate_runbook", { asset, target }),
};
