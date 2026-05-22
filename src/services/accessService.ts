import { invoke } from "@tauri-apps/api/core";
import type { AccessEndpoint, CredentialRef } from "../types";

export const accessService = {
  listEndpoints: (assetId?: number) =>
    invoke<AccessEndpoint[]>("access_get_access_endpoints", { assetId }),
  createEndpoint: (endpoint: AccessEndpoint) =>
    invoke<AccessEndpoint>("access_create_access_endpoint", { endpoint }),
  updateEndpoint: (endpoint: AccessEndpoint) =>
    invoke<AccessEndpoint>("access_update_access_endpoint", { endpoint }),
  removeEndpoint: (id: number) =>
    invoke("access_delete_access_endpoint", { id }),
  listCredentialRefs: () =>
    invoke<CredentialRef[]>("access_get_credential_refs"),
  createCredentialRef: (credentialRef: CredentialRef) =>
    invoke<CredentialRef>("access_create_credential_ref", { credentialRef }),
  updateCredentialRef: (credentialRef: CredentialRef) =>
    invoke<CredentialRef>("access_update_credential_ref", { credentialRef }),
  removeCredentialRef: (id: number) =>
    invoke("access_delete_credential_ref", { id }),
};
