import type { AISubscriptionConfig, Settings } from "../types";

export function createClearedSubscription(): AISubscriptionConfig {
  return {
    plan: "free",
    planDisplayName: "Free",
    status: "inactive",
    seats: 1,
    billingScope: "global",
    pricePerSeat: 0,
    currency: "USD",
    startedAt: null,
    renewalAt: null,
    allowCustomEndpoint: true,
    useCustomEndpoint: true,
    syncToCloud: true,
  };
}

export function createClearedAiConfig(ai: Settings["ai"]): Settings["ai"] {
  return {
    ...ai,
    apiUrl: "https://api.openai.com/v1",
    apiKey: "",
    modelName: "gpt-3.5-turbo",
    providerType: "openai",
    customEndpoint: {
      useCustomEndpoint: true,
      endpointName: "Default Custom Endpoint",
      apiUrl: "https://api.openai.com/v1",
      apiKey: "",
      modelName: "gpt-3.5-turbo",
      providerType: "openai",
    },
    subscription: createClearedSubscription(),
    subscriptionSnapshot: null,
    pendingCheckoutSession: null,
  };
}

export function buildAccountFingerprint(account: Settings["account"]) {
  return [
    account.mode,
    account.email?.trim() ?? "",
    account.userId?.trim() ?? "",
    account.enterpriseId?.trim() ?? "",
    account.enterpriseName?.trim() ?? "",
    account.subAccountId?.trim() ?? "",
  ].join("|");
}

export function normalizeAccountForSave(
  currentAccount: Settings["account"],
  nextAccountDraft: Settings["account"],
) {
  const currentFingerprint = buildAccountFingerprint(currentAccount);
  const nextFingerprint = buildAccountFingerprint(nextAccountDraft);
  const shouldClearCloudState =
    nextAccountDraft.mode === "local" || currentFingerprint !== nextFingerprint;
  const nextDisplayName =
    nextAccountDraft.mode === "local"
      ? "Local Workspace"
      : nextAccountDraft.displayName ||
        nextAccountDraft.email ||
        nextAccountDraft.userId ||
        nextAccountDraft.subAccountId ||
        "Personal Account";

  return {
    account: {
      ...nextAccountDraft,
      displayName: nextDisplayName,
      email:
        nextAccountDraft.mode === "personal"
          ? (nextAccountDraft.email || nextAccountDraft.userId || "").trim() || null
          : null,
      userId:
        nextAccountDraft.mode === "personal"
          ? (nextAccountDraft.userId || nextAccountDraft.email || "").trim() || null
          : null,
      enterpriseId:
        nextAccountDraft.mode === "enterpriseSubAccount"
          ? (nextAccountDraft.enterpriseId || "").trim() || null
          : null,
      enterpriseName:
        nextAccountDraft.mode === "enterpriseSubAccount"
          ? (nextAccountDraft.enterpriseName || "").trim() || null
          : null,
      subAccountId:
        nextAccountDraft.mode === "enterpriseSubAccount"
          ? (nextAccountDraft.subAccountId || "").trim() || null
          : null,
      accessToken: shouldClearCloudState ? null : nextAccountDraft.accessToken ?? null,
      refreshToken: shouldClearCloudState ? null : nextAccountDraft.refreshToken ?? null,
      expiresAt: shouldClearCloudState ? null : nextAccountDraft.expiresAt ?? null,
      refreshExpiresAt: shouldClearCloudState
        ? null
        : nextAccountDraft.refreshExpiresAt ?? null,
    },
    shouldClearCloudState,
  };
}

export function evaluateSettingsModeTransition(
  previousAccount: Settings["account"],
  nextAccountDraft: Settings["account"],
) {
  const normalized = normalizeAccountForSave(previousAccount, nextAccountDraft);
  const currentFingerprint = buildAccountFingerprint(previousAccount);
  const nextFingerprint = buildAccountFingerprint(normalized.account);
  const shouldClearCloudState =
    normalized.account.mode !== previousAccount.mode ||
    currentFingerprint !== nextFingerprint;

  return {
    normalizedAccount: normalized.account,
    shouldClearCloudState,
    shouldSaveLocalSnapshot:
      shouldClearCloudState &&
      previousAccount.mode === "local" &&
      normalized.account.mode !== "local",
    shouldRestoreLocalSnapshot:
      shouldClearCloudState && normalized.account.mode === "local",
    shouldClearWorkspace:
      shouldClearCloudState && normalized.account.mode !== "local",
  };
}

export function evaluateLoginGatewayModeTransition(
  previousMode: Settings["account"]["mode"],
  targetMode: Settings["account"]["mode"],
  loginGatewayRequired: boolean,
) {
  return {
    shouldPreserveLocalSnapshot:
      previousMode === "local" &&
      targetMode !== "local" &&
      !loginGatewayRequired,
    shouldLogoutToLocal: targetMode === "local",
    shouldPrepareCloudLogin: targetMode !== "local",
  };
}
