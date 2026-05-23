import { invoke } from "@tauri-apps/api/core";
import type { HostAsset } from "../types";

export const aiService = {
  planAction: (asset: HostAsset, userRequest: string) =>
    invoke<string>("ai_plan_action", { asset, userRequest }),
  explainState: (asset: HostAsset, observedState: string) =>
    invoke<string>("ai_explain_state", { asset, observedState }),
  generateRunbook: (asset: HostAsset, target: string) =>
    invoke<string>("ai_generate_runbook", { asset, target }),
};
