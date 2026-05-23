import { invoke } from "@tauri-apps/api/core";
import type {
  AssetSessionConnectResult,
  Connection,
  ConnectionHistorySource,
  OpsSession,
} from "../types";

export const sessionService = {
  connectAsset: (
    assetId: number,
    accessEndpointId?: number | null,
    existingSessionId?: string | null,
    source?: ConnectionHistorySource,
  ) =>
    invoke<AssetSessionConnectResult>("session_connect_asset", {
      assetId,
      accessEndpointId,
      existingSessionId,
      source,
    }),
  disconnectAsset: (sessionId: string, assetId?: number | null) =>
    invoke("session_disconnect_asset", { sessionId, assetId }),
  listOpsSessions: () =>
    invoke<OpsSession[]>("session_get_ops_sessions"),
  testConnection: (config: Connection) =>
    invoke<string>("test_connection", { config }),
};
