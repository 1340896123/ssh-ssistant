const DEFAULT_BASE_URL = process.env.SSH_ASSISTANT_ADMIN_BASE_URL || "http://localhost:5047";
const SUBSCRIPTION_STATUS_ACTIVE = 2;

function normalizeBaseUrl(url) {
  return (url || DEFAULT_BASE_URL).replace(/\/+$/, "");
}

async function requestJson(baseUrl, path, options = {}) {
  const response = await fetch(`${baseUrl}${path}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    },
  });

  let payload = null;
  const text = await response.text();
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = text;
    }
  }

  if (!response.ok) {
    const detail =
      payload && typeof payload === "object" && "error" in payload
        ? payload.error
        : JSON.stringify(payload);
    throw new Error(`${response.status} ${response.statusText}: ${detail}`);
  }

  return payload;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function toIsoAfterDays(days) {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toISOString();
}

function parseSettingsJson(raw) {
  if (!raw) {
    return {};
  }
  return JSON.parse(raw);
}

function isActiveSubscriptionStatus(status) {
  if (typeof status === "number") {
    return status === SUBSCRIPTION_STATUS_ACTIVE;
  }

  return String(status).trim().toLowerCase() === "active";
}

function buildSettingsPayload({ account, sync, customEndpoint, settingsJson }) {
  return {
    mode: account.mode,
    accountKey: account.accountKey,
    displayName: account.displayName,
    email: account.email,
    enterpriseId: account.enterpriseId,
    enterpriseName: account.enterpriseName,
    subAccountId: account.subAccountId,
    accessToken: account.accessToken,
    syncEndpointUrl: sync.endpointUrl,
    organizationScope: sync.organizationScope,
    syncAssets: sync.syncAssets,
    syncSettings: sync.syncSettings,
    useCustomEndpoint: customEndpoint.useCustomEndpoint,
    endpointName: customEndpoint.endpointName,
    provider: customEndpoint.provider,
    baseUrl: customEndpoint.baseUrl,
    apiKey: customEndpoint.apiKey,
    modelName: customEndpoint.modelName,
    settingsJson: JSON.stringify(settingsJson),
  };
}

async function main() {
  const baseUrl = normalizeBaseUrl(process.argv[2]);
  const suffix = new Date().toISOString().replace(/[-:.TZ]/g, "").slice(0, 14);
  const enterpriseId = `ent-g2-${suffix}`;
  const subAccountId = `sub-g2-${suffix}`;
  const adminHeaders = {};

  const results = [];
  const record = (id, status, detail) => {
    results.push({ id, status, detail });
  };

  try {
    const adminLogin = await requestJson(baseUrl, "/api/admin/login", {
      method: "POST",
      body: JSON.stringify({
        username: "admin",
        password: "admin123",
      }),
    });
    adminHeaders.Authorization = `Bearer ${adminLogin.token}`;

    const dashboardBefore = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });

    const assetPool = (dashboardBefore.assets || []).filter((item) => item.ownerType === "enterprise");
    assert(assetPool.length >= 2, "Expected at least 2 enterprise assets in admin dashboard.");
    assert(new Set(assetPool.map((item) => item.host)).size >= 2, "Expected enterprise assets with distinct hosts.");
    assert(
      new Set(assetPool.map((item) => item.environment)).size >= 2,
      "Expected enterprise assets with distinct environments.",
    );
    const authorizedAssetIds = assetPool.slice(0, 2).map((item) => item.id);
    const narrowedAssetIds = authorizedAssetIds.slice(1);

    record("G2.3", "passed", `Found ${assetPool.length} enterprise assets; using ${authorizedAssetIds.join(", ")} for verification.`);

    const enterprisePayload = {
      id: enterpriseId,
      name: `G2 Enterprise ${suffix}`,
      seatCount: 6,
      subscriptionPlan: "enterprise",
      subscriptionStatus: SUBSCRIPTION_STATUS_ACTIVE,
      renewAt: toIsoAfterDays(30),
    };

    const enterpriseSaved = await requestJson(baseUrl, "/api/admin/enterprises", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify(enterprisePayload),
    });
    assert(enterpriseSaved.id === enterpriseId, "Created enterprise id mismatch.");
    assert(enterpriseSaved.name === enterprisePayload.name, "Created enterprise name mismatch.");
    assert(enterpriseSaved.seatCount === enterprisePayload.seatCount, "Created enterprise seat count mismatch.");
    assert(isActiveSubscriptionStatus(enterpriseSaved.subscriptionStatus), "Created enterprise subscription status mismatch.");

    const dashboardAfterEnterprise = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });
    const persistedEnterprise = dashboardAfterEnterprise.enterprises.find((item) => item.id === enterpriseId);
    assert(persistedEnterprise, "Created enterprise not found after dashboard refresh.");
    assert(persistedEnterprise.name === enterprisePayload.name, "Persisted enterprise name mismatch after refresh.");
    record("G2.1", "passed", `Enterprise ${enterpriseId} created and found again after dashboard refresh.`);

    const subAccountPayload = {
      id: subAccountId,
      enterpriseId,
      displayName: `G2 Sub ${suffix}`,
      email: `${subAccountId}@example.com`,
      secret: "g2-pass-123",
      enabled: true,
      assetIds: authorizedAssetIds,
    };

    const savedSubAccount = await requestJson(baseUrl, "/api/admin/sub-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify(subAccountPayload),
    });
    assert(savedSubAccount.id === subAccountId, "Created sub account id mismatch.");
    assert(savedSubAccount.enterpriseId === enterpriseId, "Created sub account enterprise mismatch.");
    assert(savedSubAccount.displayName === subAccountPayload.displayName, "Created sub account display name mismatch.");
    assert(savedSubAccount.email === subAccountPayload.email, "Created sub account email mismatch.");
    assert(savedSubAccount.enabled === true, "Created sub account should be enabled.");

    const dashboardAfterSubAccount = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });
    const persistedSubAccount = dashboardAfterSubAccount.subAccounts.find((item) => item.id === subAccountId);
    assert(persistedSubAccount, "Created sub account not found after dashboard refresh.");
    record("G2.2", "passed", `Sub account ${subAccountId} created and persisted.`);

    const assetAuthSaved = await requestJson(baseUrl, `/api/admin/sub-accounts/${subAccountId}/assets`, {
      method: "PUT",
      headers: adminHeaders,
      body: JSON.stringify({
        assetIds: authorizedAssetIds,
      }),
    });
    assert(
      JSON.stringify(assetAuthSaved.assetIds) === JSON.stringify(authorizedAssetIds),
      "Initial authorized asset ids mismatch.",
    );

    const dashboardAfterAuth = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });
    const refreshedAuth = dashboardAfterAuth.subAccounts.find((item) => item.id === subAccountId);
    assert(refreshedAuth, "Sub account missing after asset authorization refresh.");
    assert(
      JSON.stringify(refreshedAuth.assetIds) === JSON.stringify(authorizedAssetIds),
      "Authorized asset ids mismatch after dashboard refresh.",
    );
    record("G2.4", "passed", `Sub account authorization saved and refreshed with assets ${authorizedAssetIds.join(", ")}.`);

    const clientLogin = await requestJson(baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "enterpriseSubAccount",
        identifier: subAccountPayload.email,
        secret: subAccountPayload.secret,
      }),
    });
    assert(clientLogin.mode === "enterpriseSubAccount", "Client login mode should be enterpriseSubAccount.");
    assert(clientLogin.subAccountId === subAccountId, "Client login sub account id mismatch.");
    assert(clientLogin.enterpriseId === enterpriseId, "Client login enterprise id mismatch.");
    record("G2.5", "passed", `Sub account login succeeded in enterprise mode on ${new Date().toISOString().slice(0, 10)}.`);

    const pullAuthorized = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: clientLogin.mode,
        accountKey: clientLogin.accountKey,
        accessToken: clientLogin.accessToken,
      })}`,
    );
    const authorizedAssets = JSON.parse(pullAuthorized.assetsJson || "[]");
    const pulledAuthorizedIds = authorizedAssets.map((item) => item.asset?.cloudId || item.asset?.id).filter(Boolean);
    assert(
      JSON.stringify(pulledAuthorizedIds) === JSON.stringify(authorizedAssetIds),
      `Expected authorized asset ids ${authorizedAssetIds.join(", ")}, got ${pulledAuthorizedIds.join(", ")}.`,
    );

    await requestJson(baseUrl, `/api/admin/sub-accounts/${subAccountId}/assets`, {
      method: "PUT",
      headers: adminHeaders,
      body: JSON.stringify({
        assetIds: narrowedAssetIds,
      }),
    });

    const pullNarrowed = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: clientLogin.mode,
        accountKey: clientLogin.accountKey,
        accessToken: clientLogin.accessToken,
      })}`,
    );
    const narrowedAssets = JSON.parse(pullNarrowed.assetsJson || "[]");
    const pulledNarrowedIds = narrowedAssets.map((item) => item.asset?.cloudId || item.asset?.id).filter(Boolean);
    assert(
      JSON.stringify(pulledNarrowedIds) === JSON.stringify(narrowedAssetIds),
      `Expected narrowed asset ids ${narrowedAssetIds.join(", ")}, got ${pulledNarrowedIds.join(", ")}.`,
    );
    record("G2.6", "passed", `Asset pull updated from ${authorizedAssetIds.length} assets to ${narrowedAssetIds.length} asset after authorization change.`);

    const syncedSettingsPayload = {
      theme: "light",
      language: "en",
      terminalAppearance: {
        fontSize: 16,
        fontFamily: "Fira Code, monospace",
        cursorStyle: "bar",
        lineHeight: 1.1,
      },
      fileManager: {
        viewMode: "tree",
        layout: "left",
        sftpBufferSize: 768,
      },
      sshPool: {
        maxBackgroundSessions: 7,
        enableAutoCleanup: true,
        cleanupIntervalMinutes: 6,
      },
      connectionTimeout: {
        connectionTimeoutSecs: 20,
        jumpHostTimeoutSecs: 35,
        localForwardTimeoutSecs: 12,
        commandTimeoutSecs: 45,
        sftpOperationTimeoutSecs: 75,
      },
      reconnect: {
        maxReconnectAttempts: 6,
        initialDelayMs: 1200,
        maxDelayMs: 32000,
        backoffMultiplier: 2.2,
        enableAutoReconnect: true,
      },
      heartbeat: {
        tcpKeepaliveIntervalSecs: 50,
        sshKeepaliveIntervalSecs: 20,
        appHeartbeatIntervalSecs: 25,
        heartbeatTimeoutSecs: 6,
        failedHeartbeatsBeforeAction: 4,
      },
      poolHealth: {
        healthCheckIntervalSecs: 70,
        sessionWarmupCount: 2,
        maxSessionAgeMinutes: 90,
        unhealthyThreshold: 4,
      },
      networkAdaptive: {
        enableAdaptive: true,
        latencyCheckIntervalSecs: 40,
        highLatencyThresholdMs: 280,
        lowBandwidthThresholdKbps: 120,
      },
    };

    const syncedSettings = await requestJson(baseUrl, "/api/client/sync/settings", {
      method: "POST",
      body: JSON.stringify(
        buildSettingsPayload({
          account: {
            mode: clientLogin.mode,
            accountKey: clientLogin.accountKey,
            displayName: clientLogin.displayName,
            email: clientLogin.email,
            enterpriseId: clientLogin.enterpriseId,
            enterpriseName: clientLogin.enterpriseName,
            subAccountId: clientLogin.subAccountId,
            accessToken: clientLogin.accessToken,
          },
          sync: {
            endpointUrl: `${baseUrl}/api/client/sync`,
            organizationScope: enterpriseId,
            syncAssets: true,
            syncSettings: true,
          },
          customEndpoint: {
            useCustomEndpoint: true,
            endpointName: `custom-${suffix}`,
            provider: "openai",
            baseUrl: "https://example.invalid/v1",
            apiKey: `key-${suffix}`,
            modelName: "gpt-4o-mini",
          },
          settingsJson: syncedSettingsPayload,
        }),
      ),
    });

    const syncedSettingsParsed = parseSettingsJson(syncedSettings.settingsJson);
    assert(syncedSettingsParsed.theme === "light", "Synced settings theme mismatch.");
    assert(syncedSettingsParsed.fileManager?.layout === "left", "Synced settings file manager layout mismatch.");
    assert(syncedSettings.customEndpoint?.endpointName === `custom-${suffix}`, "Synced custom endpoint mismatch.");
    record("G2.7", "passed", "Client settings sync returned success and preserved the updated settings payload.");

    const pullAfterSync = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: clientLogin.mode,
        accountKey: clientLogin.accountKey,
        accessToken: clientLogin.accessToken,
      })}`,
    );
    const pullAfterSyncSettings = parseSettingsJson(pullAfterSync.settingsJson);
    assert(pullAfterSyncSettings.theme === "light", "Pulled settings theme mismatch after sync.");
    assert(pullAfterSyncSettings.fileManager?.layout === "left", "Pulled settings file manager layout mismatch after sync.");

    const refreshedClientLogin = await requestJson(baseUrl, "/api/client/refresh", {
      method: "POST",
      body: JSON.stringify({
        refreshToken: clientLogin.refreshToken,
      }),
    });

    const pullAfterRefresh = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: refreshedClientLogin.mode,
        accountKey: refreshedClientLogin.accountKey,
        accessToken: refreshedClientLogin.accessToken,
      })}`,
    );
    const pullAfterRefreshSettings = parseSettingsJson(pullAfterRefresh.settingsJson);
    assert(pullAfterRefreshSettings.theme === "light", "Pulled settings theme mismatch after re-login.");
    assert(
      pullAfterRefreshSettings.terminalAppearance?.fontSize === 16,
      "Pulled terminal font size mismatch after re-login.",
    );
    record("G2.8", "passed", "Settings remained available after client refresh login and pull.");
  } catch (error) {
    console.error(JSON.stringify({ ok: false, error: error.message, results }, null, 2));
    process.exitCode = 1;
    return;
  }

  console.log(JSON.stringify({ ok: true, baseUrl, results }, null, 2));
}

main();
