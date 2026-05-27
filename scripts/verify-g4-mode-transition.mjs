import assert from "node:assert/strict";

function createClearedSubscription() {
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

function createClearedAiConfig(ai) {
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

function buildAccountFingerprint(account) {
  return [
    account.mode,
    account.email?.trim() ?? "",
    account.userId?.trim() ?? "",
    account.enterpriseId?.trim() ?? "",
    account.enterpriseName?.trim() ?? "",
    account.subAccountId?.trim() ?? "",
  ].join("|");
}

function normalizeAccountForSave(currentAccount, nextAccountDraft) {
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

function evaluateSettingsModeTransition(previousAccount, nextAccountDraft) {
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

function evaluateLoginGatewayModeTransition(previousMode, targetMode, loginGatewayRequired) {
  return {
    shouldPreserveLocalSnapshot:
      previousMode === "local" &&
      targetMode !== "local" &&
      !loginGatewayRequired,
    shouldLogoutToLocal: targetMode === "local",
    shouldPrepareCloudLogin: targetMode !== "local",
  };
}

function buildBaseSettings() {
  return {
    theme: "dark",
    language: "zh",
    account: {
      mode: "local",
      userId: null,
      displayName: "Local Workspace",
      email: null,
      enterpriseId: null,
      enterpriseName: null,
      subAccountId: null,
      accessToken: null,
      refreshToken: null,
      expiresAt: null,
      refreshExpiresAt: null,
    },
    sync: {
      enabled: false,
      endpointUrl: "",
      organizationScope: "",
      syncAssets: true,
      syncSettings: true,
      lastCloudSyncAt: null,
    },
    ai: {
      apiUrl: "https://local.example/v1",
      apiKey: "local-key",
      modelName: "local-model",
      providerType: "openai",
      subscription: {
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
        syncToCloud: false,
      },
      customEndpoint: {
        useCustomEndpoint: true,
        endpointName: "Local Endpoint",
        apiUrl: "https://local.example/v1",
        apiKey: "local-key",
        modelName: "local-model",
        providerType: "openai",
      },
      subscriptionSnapshot: null,
      pendingCheckoutSession: null,
    },
  };
}

function verifySettingsTransitions() {
  const base = buildBaseSettings();

  const toPersonal = evaluateSettingsModeTransition(base.account, {
    ...base.account,
    mode: "personal",
    displayName: "Personal Demo",
    email: "personal@example.com",
    userId: "personal@example.com",
  });
  assert.equal(toPersonal.shouldClearCloudState, true);
  assert.equal(toPersonal.shouldSaveLocalSnapshot, true);
  assert.equal(toPersonal.shouldRestoreLocalSnapshot, false);
  assert.equal(toPersonal.shouldClearWorkspace, true);
  assert.equal(toPersonal.normalizedAccount.accessToken, null);
  assert.equal(toPersonal.normalizedAccount.mode, "personal");

  const personalState = {
    ...base.account,
    mode: "personal",
    displayName: "Personal Demo",
    email: "personal@example.com",
    userId: "personal@example.com",
    accessToken: "personal-token",
    refreshToken: "refresh-token",
    expiresAt: 111,
    refreshExpiresAt: 222,
  };
  const backToLocal = evaluateSettingsModeTransition(personalState, {
    ...personalState,
    mode: "local",
    displayName: "Local Workspace",
    email: null,
    userId: null,
    accessToken: "stale-token",
  });
  assert.equal(backToLocal.shouldClearCloudState, true);
  assert.equal(backToLocal.shouldSaveLocalSnapshot, false);
  assert.equal(backToLocal.shouldRestoreLocalSnapshot, true);
  assert.equal(backToLocal.shouldClearWorkspace, false);
  assert.equal(backToLocal.normalizedAccount.mode, "local");
  assert.equal(backToLocal.normalizedAccount.accessToken, null);
  assert.equal(backToLocal.normalizedAccount.displayName, "Local Workspace");

  const enterpriseState = evaluateSettingsModeTransition(personalState, {
    ...personalState,
    mode: "enterpriseSubAccount",
    displayName: "Acme Ops",
    email: null,
    userId: null,
    enterpriseId: "ent-acme",
    enterpriseName: "Acme",
    subAccountId: "sub-ops",
  });
  assert.equal(enterpriseState.shouldClearCloudState, true);
  assert.equal(enterpriseState.normalizedAccount.email, null);
  assert.equal(enterpriseState.normalizedAccount.userId, null);
  assert.equal(enterpriseState.normalizedAccount.enterpriseId, "ent-acme");
  assert.equal(enterpriseState.normalizedAccount.subAccountId, "sub-ops");
}

function verifyLoginGatewayTransitions() {
  const localToPersonal = evaluateLoginGatewayModeTransition("local", "personal", false);
  assert.equal(localToPersonal.shouldPreserveLocalSnapshot, true);
  assert.equal(localToPersonal.shouldPrepareCloudLogin, true);
  assert.equal(localToPersonal.shouldLogoutToLocal, false);

  const forcedGateway = evaluateLoginGatewayModeTransition("local", "personal", true);
  assert.equal(forcedGateway.shouldPreserveLocalSnapshot, false);

  const localMode = evaluateLoginGatewayModeTransition("enterpriseSubAccount", "local", false);
  assert.equal(localMode.shouldLogoutToLocal, true);
  assert.equal(localMode.shouldPrepareCloudLogin, false);
}

function verifyAiReset() {
  const base = buildBaseSettings();
  const cleared = createClearedAiConfig(base.ai);
  assert.equal(cleared.apiKey, "");
  assert.equal(cleared.modelName, "gpt-3.5-turbo");
  assert.equal(cleared.customEndpoint.endpointName, "Default Custom Endpoint");
  assert.equal(cleared.subscription.plan, "free");
  assert.equal(cleared.subscriptionSnapshot, null);
  assert.equal(cleared.pendingCheckoutSession, null);
}

function main() {
  verifySettingsTransitions();
  verifyLoginGatewayTransitions();
  verifyAiReset();

  console.log(
    JSON.stringify(
      {
        ok: true,
        verified: [
          "settings-mode-transition-local-personal",
          "settings-mode-transition-personal-local",
          "settings-mode-transition-personal-enterprise",
          "login-gateway-transition-decisions",
          "ai-reset-on-mode-switch",
        ],
      },
      null,
      2,
    ),
  );
}

main();
