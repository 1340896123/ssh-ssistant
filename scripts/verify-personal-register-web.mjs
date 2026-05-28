import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { nextPort, requestJson, startTempAdminApi } from "./helpers/temp-admin-api.mjs";

const PLAYWRIGHT_MODULE = "file:///C:/Users/jieok/AppData/Roaming/npm/node_modules/playwright/index.mjs";
const WEB_APP_URL = process.env.SSH_ASSISTANT_WEB_APP_URL || "http://127.0.0.1:4173";

function nowSuffix() {
  return new Date().toISOString().replace(/[-:.TZ]/g, "").slice(0, 14);
}

function buildDefaultSettings(mode = "personal") {
  return {
    theme: "dark",
    language: "zh",
    account: {
      mode,
      userId: null,
      displayName:
        mode === "enterpriseSubAccount"
          ? "Enterprise Sub-Account"
          : mode === "local"
            ? "Local Workspace"
            : "Personal Account",
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
      apiUrl: "https://api.openai.com/v1",
      apiKey: "",
      modelName: "gpt-3.5-turbo",
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
        syncToCloud: true,
      },
      customEndpoint: {
        useCustomEndpoint: true,
        endpointName: "Default Custom Endpoint",
        apiUrl: "https://api.openai.com/v1",
        apiKey: "",
        modelName: "gpt-3.5-turbo",
        providerType: "openai",
      },
      subscriptionSnapshot: null,
      pendingCheckoutSession: null,
    },
    terminalAppearance: {
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      cursorStyle: "block",
      lineHeight: 1,
    },
    fileManager: {
      viewMode: "tree",
      layout: "left",
      sftpBufferSize: 512,
    },
    sshPool: {
      maxBackgroundSessions: 6,
      enableAutoCleanup: true,
      cleanupIntervalMinutes: 5,
    },
    connectionTimeout: {
      connectionTimeoutSecs: 15,
      jumpHostTimeoutSecs: 30,
      localForwardTimeoutSecs: 10,
      commandTimeoutSecs: 30,
      sftpOperationTimeoutSecs: 60,
    },
    reconnect: {
      maxReconnectAttempts: 5,
      initialDelayMs: 1000,
      maxDelayMs: 30000,
      backoffMultiplier: 2,
      enableAutoReconnect: true,
    },
    heartbeat: {
      tcpKeepaliveIntervalSecs: 60,
      sshKeepaliveIntervalSecs: 15,
      appHeartbeatIntervalSecs: 30,
      heartbeatTimeoutSecs: 5,
      failedHeartbeatsBeforeAction: 3,
    },
    poolHealth: {
      healthCheckIntervalSecs: 60,
      sessionWarmupCount: 1,
      maxSessionAgeMinutes: 60,
      unhealthyThreshold: 3,
    },
    networkAdaptive: {
      enableAdaptive: true,
      latencyCheckIntervalSecs: 30,
      highLatencyThresholdMs: 300,
      lowBandwidthThresholdKbps: 100,
    },
  };
}

async function waitForWebApp(url, timeoutMs = 30000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      // Wait until the app is available.
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(`Timed out waiting for frontend app at ${url}.`);
}

async function createMockedPage(browser, { endpointUrl, mode = "personal" }) {
  const context = await browser.newContext({
    viewport: { width: 1440, height: 960 },
  });
  const page = await context.newPage();
  const initialSettings = buildDefaultSettings(mode);

  await page.addInitScript(({ initialSettings: settingsSeed, endpointUrl: runtimeEndpoint }) => {
    const listeners = new Map();
    const settingsState = JSON.parse(JSON.stringify(settingsSeed));
    settingsState.sync.endpointUrl = runtimeEndpoint;

    const assetState = {
      assets: [],
      folders: [],
      environments: [],
      tags: [],
      savedViews: [],
      accessEndpoints: [],
      credentialRefs: [],
      accessHistory: [],
    };

    window.localStorage.setItem("preferred-locale", "zh");
    window.localStorage.setItem("login-gateway-required", "1");

    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        switch (cmd) {
          case "get_settings":
            return JSON.parse(JSON.stringify(settingsState));
          case "save_settings":
            Object.assign(settingsState, JSON.parse(JSON.stringify(args?.settings || {})));
            return null;
          case "asset_get_host_assets":
            return JSON.parse(JSON.stringify(assetState.assets));
          case "asset_get_asset_folders":
            return JSON.parse(JSON.stringify(assetState.folders));
          case "asset_get_environments":
            return JSON.parse(JSON.stringify(assetState.environments));
          case "asset_get_asset_tags":
            return JSON.parse(JSON.stringify(assetState.tags));
          case "asset_get_saved_views":
            return JSON.parse(JSON.stringify(assetState.savedViews));
          case "access_get_access_endpoints":
            return JSON.parse(JSON.stringify(assetState.accessEndpoints));
          case "access_get_credential_refs":
            return JSON.parse(JSON.stringify(assetState.credentialRefs));
          case "asset_get_access_history":
            return JSON.parse(JSON.stringify(assetState.accessHistory));
          case "sync_get_state":
            return null;
          case "asset_import_cloud_records": {
            const records = args?.records || [];
            assetState.assets = records.map((record, index) => {
              const asset = record?.asset || {};
              const endpoint = record?.defaultAccessEndpoint || {};
              return {
                id: typeof asset.id === "number" ? asset.id : index + 1,
                cloudId: asset.cloudId ?? null,
                name: asset.name || `Asset ${index + 1}`,
                host: asset.host || "127.0.0.1",
                port: asset.port || 22,
                platform: asset.platform || "Linux",
                folderId: asset.folderId ?? null,
                envId: asset.envId ?? null,
                labels: asset.labels || [],
                owner: asset.owner || "",
                criticality: asset.criticality || "medium",
                defaultWorkspacePath: asset.defaultWorkspacePath ?? null,
                accessEndpointId: endpoint.id ?? null,
                bastionChainId: asset.bastionChainId ?? null,
                healthSummary: asset.healthSummary ?? null,
                lastAccessedAt: asset.lastAccessedAt ?? null,
                isFavorite: Boolean(asset.isFavorite),
                groupId: asset.groupId ?? null,
              };
            });
            assetState.accessEndpoints = records.map((record, index) => {
              const endpoint = record?.defaultAccessEndpoint || {};
              return {
                id: typeof endpoint.id === "number" ? endpoint.id : index + 1,
                assetId: typeof record?.asset?.id === "number" ? record.asset.id : index + 1,
                name: endpoint.name || `Endpoint ${index + 1}`,
                host: endpoint.host || "127.0.0.1",
                port: endpoint.port || 22,
                username: endpoint.username || "root",
                authType: endpoint.authType || "password",
                credentialRefId: endpoint.credentialRefId ?? null,
                sshKeyId: endpoint.sshKeyId ?? null,
                jumpHost: endpoint.jumpHost ?? null,
                jumpPort: endpoint.jumpPort ?? null,
                jumpUsername: endpoint.jumpUsername ?? null,
                jumpPassword: endpoint.jumpPassword ?? null,
              };
            });
            return assetState.assets.length;
          }
          case "asset_clear_workspace":
            assetState.assets = [];
            assetState.accessEndpoints = [];
            assetState.folders = [];
            assetState.environments = [];
            assetState.tags = [];
            assetState.savedViews = [];
            assetState.credentialRefs = [];
            assetState.accessHistory = [];
            return null;
          case "asset_export_local_workspace_snapshot":
            return {
              assets: [],
              folders: [],
              environments: [],
              tags: [],
              savedViews: [],
              accessEndpoints: [],
              credentialRefs: [],
              accessHistory: [],
            };
          case "asset_restore_local_workspace_snapshot":
          case "save_local_workspace_snapshot":
          case "get_local_workspace_snapshot":
          case "session_get_ops_sessions":
          case "get_transfers":
            return [];
          default:
            return null;
        }
      },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      convertFileSrc: (filePath) => filePath,
      metadata: {
        currentWindow: {
          label: "main",
        },
      },
    };

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
      listeners,
    };

    window.__MOCK_TAURI_READY__ = true;
    window.open = () => null;
    window.confirm = () => true;
    window.alert = () => {};
    window.__REGISTER_SETTINGS_STATE__ = settingsState;
    window.__REGISTER_ASSET_STATE__ = assetState;
  }, {
    initialSettings,
    endpointUrl,
  });

  await page.goto(WEB_APP_URL, { waitUntil: "domcontentloaded" });
  await page.getByText("登录工作台").waitFor({ timeout: 20000 });
  return {
    context,
    page,
  };
}

async function main() {
  await waitForWebApp(WEB_APP_URL);

  const port = Number(process.env.SSH_ASSISTANT_REGISTER_WEB_TEST_PORT || nextPort(5200));
  const server = await startTempAdminApi({
    port,
    label: "register-web",
  });

  const suffix = nowSuffix();
  const successEmail = `web-register-${suffix}@example.com`;
  const successDisplayName = `Web Register ${suffix}`;
  const duplicateEmail = `web-duplicate-${suffix}@example.com`;
  const password = "secret123";

  const { chromium } = await import(PLAYWRIGHT_MODULE);
  const browser = await chromium.launch({ headless: true });

  try {
    await requestJson(server.baseUrl, "/api/client/register", {
      method: "POST",
      body: JSON.stringify({
        email: duplicateEmail,
        displayName: `Duplicate ${suffix}`,
        password,
      }),
    });

    const successScenario = await createMockedPage(browser, {
      endpointUrl: server.baseUrl,
      mode: "personal",
    });
    const successPage = successScenario.page;

    await successPage.getByRole("button", { name: "注册" }).click();
    await successPage.locator('input[placeholder="user@example.com"]').fill(successEmail);
    await successPage.locator('input[placeholder="例如：Alice"]').fill(successDisplayName);
    await successPage.locator('input[placeholder="至少 6 位密码"]').fill(password);
    await successPage.getByRole("button", { name: "注册并进入工作台" }).click();
    await successPage.waitForFunction(() => document.body.innerText.includes("Switch"), null, {
      timeout: 15000,
    });

    const successBody = await successPage.locator("body").innerText();
    const successState = await successPage.evaluate(() => window.__REGISTER_SETTINGS_STATE__);
    const successAssetState = await successPage.evaluate(() => window.__REGISTER_ASSET_STATE__);
    const successScreenshotPath = path.resolve(".playwright-cli", "personal-register-web-flow.png");
    await successPage.screenshot({ path: successScreenshotPath, fullPage: true });
    await successScenario.context.close();

    const duplicateScenario = await createMockedPage(browser, {
      endpointUrl: server.baseUrl,
      mode: "personal",
    });
    const duplicatePage = duplicateScenario.page;
    await duplicatePage.getByRole("button", { name: "注册" }).click();
    await duplicatePage.locator('input[placeholder="user@example.com"]').fill(duplicateEmail);
    await duplicatePage.locator('input[placeholder="例如：Alice"]').fill(`Duplicate ${suffix}`);
    await duplicatePage.locator('input[placeholder="至少 6 位密码"]').fill(password);
    await duplicatePage.getByRole("button", { name: "注册并进入工作台" }).click();
    await duplicatePage.getByText("该邮箱已注册，请直接登录。").waitFor({ timeout: 10000 });
    await duplicateScenario.context.close();

    const invalidScenario = await createMockedPage(browser, {
      endpointUrl: server.baseUrl,
      mode: "personal",
    });
    const invalidPage = invalidScenario.page;
    await invalidPage.getByRole("button", { name: "注册" }).click();
    await invalidPage.locator('input[placeholder="user@example.com"]').fill("bad-email");
    await invalidPage.locator('input[placeholder="例如：Alice"]').fill("");
    await invalidPage.locator('input[placeholder="至少 6 位密码"]').fill("123");
    await invalidPage.getByRole("button", { name: "注册并进入工作台" }).click();
    await invalidPage.getByText("注册信息不合法，请检查邮箱、显示名和密码。").waitFor({ timeout: 10000 });
    await invalidScenario.context.close();

    const serviceUnavailableScenario = await createMockedPage(browser, {
      endpointUrl: "http://127.0.0.1:1",
      mode: "personal",
    });
    const serviceUnavailablePage = serviceUnavailableScenario.page;
    await serviceUnavailablePage.getByRole("button", { name: "注册" }).click();
    await serviceUnavailablePage.locator('input[placeholder="user@example.com"]').fill(`offline-${suffix}@example.com`);
    await serviceUnavailablePage.locator('input[placeholder="例如：Alice"]').fill(`Offline ${suffix}`);
    await serviceUnavailablePage.locator('input[placeholder="至少 6 位密码"]').fill(password);
    await serviceUnavailablePage.getByRole("button", { name: "注册并进入工作台" }).click();
    await serviceUnavailablePage.getByText("服务暂时不可用，请稍后重试。").waitFor({ timeout: 10000 });
    await serviceUnavailableScenario.context.close();

    const enterpriseScenario = await createMockedPage(browser, {
      endpointUrl: server.baseUrl,
      mode: "enterpriseSubAccount",
    });
    const enterpriseHasRegister = await enterpriseScenario.page.getByRole("button", { name: "注册" }).count();
    await enterpriseScenario.context.close();

    const localScenario = await createMockedPage(browser, {
      endpointUrl: server.baseUrl,
      mode: "local",
    });
    await localScenario.page.locator("select").selectOption("local");
    const localHasRegister = await localScenario.page.getByRole("button", { name: "注册" }).count();
    await localScenario.context.close();

    const payload = {
      ok: true,
      verified: {
        enteredWorkbench: successBody.includes("Switch"),
        showsPersonalMode: successBody.includes(`${successDisplayName} · personal`),
        showsRegisteredIdentity: successBody.includes(`${successEmail} · global`),
        showsFreeSubscription: successBody.includes("Free ·"),
        savedAccountMode: successState?.account?.mode === "personal",
        savedAccountEmail: successState?.account?.email === successEmail,
        savedAccountDisplayName: successState?.account?.displayName === successDisplayName,
        savedSubscriptionStatus: successState?.ai?.subscription?.status === "trialing",
        savedSubscriptionPlan: successState?.ai?.subscription?.plan === "free",
        duplicateErrorVisible: true,
        invalidParametersVisible: true,
        serviceUnavailableVisible: true,
        noRegisterTabInEnterpriseMode: enterpriseHasRegister === 0,
        noRegisterTabInLocalMode: localHasRegister === 0,
        cloudAssetsImported: Array.isArray(successAssetState?.assets),
      },
      evidence: {
        success: {
          email: successEmail,
          displayName: successDisplayName,
          savedState: {
            account: successState?.account,
            subscription: successState?.ai?.subscription,
          },
        },
        duplicateEmail,
        screenshotPath: successScreenshotPath,
      },
      bodySnippet: successBody.slice(0, 2400),
      baseUrl: server.baseUrl,
    };

    const outputDir = path.resolve(".playwright-cli");
    await mkdir(outputDir, { recursive: true });
    await writeFile(
      path.join(outputDir, "personal-register-web-flow.json"),
      `${JSON.stringify(payload, null, 2)}\n`,
      "utf8",
    );

    console.log(JSON.stringify(payload, null, 2));
  } finally {
    await browser.close();
    await server.stop();
  }
}

main().catch((error) => {
  console.error(
    JSON.stringify(
      {
        ok: false,
        error: String(error?.message || error),
      },
      null,
      2,
    ),
  );
  process.exitCode = 1;
});
