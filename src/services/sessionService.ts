import { invoke } from "@tauri-apps/api/core";
import type { AssetSessionConnectResult, Connection } from "../types";

export const sessionService = {
  connectAsset: (
    assetId: number,
    accessEndpointId?: number | null,
    existingSessionId?: string | null,
  ) =>
    invoke<AssetSessionConnectResult>("session_connect_asset", {
      assetId,
      accessEndpointId,
      existingSessionId,
    }),
  disconnectAsset: (sessionId: string, assetId?: number | null) =>
    invoke("session_disconnect_asset", { sessionId, assetId }),
  testConnection: (config: Connection) =>
    invoke<string>("test_connection", { config }),
};
