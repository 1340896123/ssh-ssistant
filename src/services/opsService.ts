import { invoke } from "@tauri-apps/api/core";
import type { HostAsset, JobRun, JobTemplate } from "../types";

export const opsService = {
  listJobTemplates: () => invoke<JobTemplate[]>("ops_list_job_templates"),
  createJobTemplate: (template: JobTemplate) =>
    invoke<JobTemplate>("ops_create_job_template", { template }),
  removeJobTemplate: (id: number) =>
    invoke("ops_delete_job_template", { id }),
  listJobRuns: (assetId?: number) =>
    invoke<JobRun[]>("ops_list_job_runs", { assetId }),
  executeJob: (
    sessionId: string,
    commandText: string,
    assetId?: number,
    riskLevel?: string,
    source?: string,
  ) =>
    invoke<JobRun>("ops_execute_job", {
      sessionId,
      assetId,
      commandText,
      riskLevel,
      source,
    }),
  aiPlanAction: (asset: HostAsset, userRequest: string) =>
    invoke<string>("ai_plan_action", { asset, userRequest }),
  aiExplainState: (asset: HostAsset, observedState: string) =>
    invoke<string>("ai_explain_state", { asset, observedState }),
  aiGenerateRunbook: (asset: HostAsset, target: string) =>
    invoke<string>("ai_generate_runbook", { asset, target }),
};
