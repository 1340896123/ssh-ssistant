import type { AccountMode } from "../types";

/**
 * Build-time application variant.
 *
 * Each compiled binary is hard-wired to exactly one variant. The value is
 * injected by Vite via `VITE_APP_VARIANT` (see `scripts/build-variant.mjs`)
 * and cannot be switched at runtime — the in-app mode selector was removed
 * as part of the three-client split.
 *
 *   - local      → offline client, no cloud login, boots straight to workbench
 *   - personal   → browser login against the personal cloud (no password in client)
 *   - enterprise → browser login against the enterprise cloud, separate binary + scheme
 */
export type AppVariant = "local" | "personal" | "enterprise";

function resolveVariant(): AppVariant {
  const raw = import.meta.env.VITE_APP_VARIANT;
  if (raw === "local" || raw === "personal" || raw === "enterprise") {
    return raw;
  }
  // Sensible default for `vite build` / `npm run dev` without an explicit variant.
  return "personal";
}

export const APP_VARIANT: AppVariant = resolveVariant();

export interface VariantMeta {
  /** The AccountMode this variant is bound to. */
  mode: AccountMode;
  /** Custom-protocol scheme used for browser auth + billing callbacks. `null` for local. */
  scheme: string | null;
  /** Default cloud endpoint base URL. `null` for local. */
  defaultCloudEndpoint: string | null;
  /** Cloud account mode forwarded to the browser login flow. `null` for local. */
  cloudMode: "personal" | "enterpriseSubAccount" | null;
  /** Human-readable product name for display. */
  productName: string;
}

export const VARIANT_META: Record<AppVariant, VariantMeta> = {
  local: {
    mode: "local",
    scheme: null,
    defaultCloudEndpoint: null,
    cloudMode: null,
    productName: "SshStar Local",
  },
  personal: {
    mode: "personal",
    scheme: "sshstar-personal",
    defaultCloudEndpoint: "http://localhost:5047",
    cloudMode: "personal",
    productName: "SshStar",
  },
  enterprise: {
    mode: "enterpriseSubAccount",
    scheme: "sshstar-enterprise",
    defaultCloudEndpoint: "http://localhost:5047",
    cloudMode: "enterpriseSubAccount",
    productName: "SshStar Enterprise",
  },
};

export const ACTIVE_VARIANT_META: VariantMeta = VARIANT_META[APP_VARIANT];

export function isLocalVariant(variant: AppVariant = APP_VARIANT): boolean {
  return variant === "local";
}

export function isCloudVariant(variant: AppVariant = APP_VARIANT): boolean {
  return variant === "personal" || variant === "enterprise";
}

/**
 * Resolve the cloud endpoint to use: the persisted user override if set,
 * otherwise the variant default. Returns null for the local variant.
 */
export function resolveCloudEndpoint(userOverride?: string | null): string | null {
  if (isLocalVariant()) {
    return null;
  }
  return userOverride || ACTIVE_VARIANT_META.defaultCloudEndpoint;
}
