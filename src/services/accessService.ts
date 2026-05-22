import { invoke } from "@tauri-apps/api/core";
import type { AccessEndpoint, CredentialRef } from "../types";

export const accessService = {
  listEndpoints: (assetId?: number) =>
    invoke<AccessEndpoint[]>("access_get_access_endpoints", { assetId }),
  listCredentialRefs: () =>
    invoke<CredentialRef[]>("access_get_credential_refs"),
};
