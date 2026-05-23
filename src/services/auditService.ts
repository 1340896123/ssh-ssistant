import { invoke } from "@tauri-apps/api/core";
import type { AuditEvent } from "../types";

export const auditService = {
  list: (assetId?: number, limit?: number) =>
    invoke<AuditEvent[]>("audit_list_events", { assetId, limit }),
  search: (
    query?: string,
    severity?: string,
    assetId?: number,
    limit?: number,
  ) =>
    invoke<AuditEvent[]>("audit_search_events", {
      query,
      severity,
      assetId,
      limit,
    }),
  create: (event: AuditEvent) =>
    invoke<AuditEvent>("audit_create_event", { event }),
};
