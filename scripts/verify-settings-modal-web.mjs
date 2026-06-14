import { writeFile } from "node:fs/promises";
import path from "node:path";
import { loadPlaywright, chromiumLaunchOptions } from "./helpers/playwright-runtime.mjs";
import { startPreviewServer } from "./helpers/web-preview.mjs";
import { exitFailure, exitSuccess } from "./helpers/script-exit.mjs";

const DEFAULT_WEB_APP_URL = process.env.SSH_ASSISTANT_WEB_APP_URL || "http://127.0.0.1:4173";

async function openSettings(page) {
  const settingsButton = page.getByTestId("app-settings-button");
  await settingsButton.waitFor({ state: "visible", timeout: 10000 });
  await settingsButton.click();
  await page.waitForTimeout(300);
}

async function openSettingsTab(page, testId) {
  const tabButton = page.getByTestId(testId);
  await tabButton.waitFor({ state: "visible", timeout: 10000 });
  await tabButton.click();
  await page.waitForTimeout(250);
}

async function main() {
  let previewServer = null;
  const webAppUrl = process.env.SSH_ASSISTANT_WEB_APP_URL
    ? DEFAULT_WEB_APP_URL
    : (previewServer = await startPreviewServer({ port: 4173, label: "verify-settings-modal-web" })).baseUrl;
  const { module: playwright } = await loadPlaywright();
  const { chromium } = playwright;
  const browser = await chromium.launch(chromiumLaunchOptions(chromium, { headless: true }));
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });

  await page.addInitScript(() => {
    const listeners = new Map();
    const settingsState = {
      theme: "dark",
      language: "zh",
      account: {
        mode: "personal",
        userId: "demo-user",
        displayName: "Demo Personal",
        email: "demo@example.com",
        enterpriseId: null,
        enterpriseName: null,
        subAccountId: null,
        accessToken: "token",
        refreshToken: "refresh",
        expiresAt: Date.now() + 60_000,
        refreshExpiresAt: Date.now() + 600_000,
      },
      sync: {
        enabled: false,
        endpointUrl: "http://127.0.0.1:5047",
        organizationScope: "global",
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
          status: "trialing",
          seats: 1,
          billingScope: "personal",
          pricePerSeat: 0,
          currency: "USD",
          startedAt: null,
          renewalAt: null,
          allowCustomEndpoint: true,
          useCustomEndpoint: false,
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
        subscriptionSnapshot: {
          subscription: {
            plan: "free",
            planDisplayName: "Free",
            status: "trialing",
            seats: 1,
            billingScope: "personal",
            pricePerSeat: 0,
            currency: "USD",
            startedAt: null,
            renewalAt: Date.now() + 86400000,
            allowCustomEndpoint: true,
            useCustomEndpoint: false,
            syncToCloud: true,
          },
          currentInvoice: null,
          recentInvoices: [],
          paymentProviders: [{ providerKey: "manual", displayName: "Manual", providerType: "manual" }],
          usage: {
            billingMonth: "2026-06",
            totalRequests: 0,
            managedRequests: 0,
            promptTokens: 0,
            completionTokens: 0,
            totalTokens: 0,
            estimatedCost: 0,
            currency: "USD",
            topAccounts: [],
          },
        },
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

    window.localStorage.setItem("preferred-locale", "zh");

    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd) => {
        switch (cmd) {
          case "get_settings":
            return JSON.parse(JSON.stringify(settingsState));
          case "save_settings":
            return null;
          case "asset_get_host_assets":
          case "asset_get_asset_folders":
          case "asset_get_environments":
          case "asset_get_asset_tags":
          case "asset_get_saved_views":
          case "asset_get_access_history":
          case "access_get_access_endpoints":
          case "access_get_credential_refs":
          case "session_get_ops_sessions":
          case "get_transfers":
          case "get_ssh_keys":
            return [];
          case "sync_get_state":
          case "get_local_workspace_snapshot":
          case "save_local_workspace_snapshot":
          case "asset_clear_workspace":
          case "asset_export_local_workspace_snapshot":
          case "asset_restore_local_workspace_snapshot":
            return null;
          default:
            return null;
        }
      },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      convertFileSrc: (filePath) => filePath,
      metadata: { currentWindow: { label: "main" } },
    };

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
      listeners,
    };
    window.__MOCK_TAURI_READY__ = true;
    window.open = () => null;
    window.confirm = () => true;
    window.alert = () => {};
  });

  await page.goto(webAppUrl, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(() => document.body.innerText.includes("Switch"), null, {
    timeout: 15000,
  });

  await openSettings(page);
  await page.getByText("设置").waitFor({ timeout: 10000 });
  const generalBody = await page.locator("body").innerText();
  await openSettingsTab(page, "settings-tab-account");
  await page.waitForFunction(
    () =>
      document.body.innerText.includes("同步服务地址") &&
      document.body.innerText.includes("组织范围"),
    null,
    { timeout: 10000 },
  );
  const accountBody = await page.locator("body").innerText();
  await openSettingsTab(page, "settings-tab-ai");
  await page.waitForFunction(
    () =>
      document.body.innerText.includes("提供商类型") &&
      document.body.innerText.includes("模型名称"),
    null,
    { timeout: 10000 },
  );
  const aiBody = await page.locator("body").innerText();
  await openSettingsTab(page, "settings-tab-sshKeys");
  await page.waitForFunction(
    () =>
      document.body.innerText.includes("SSH 密钥") &&
      document.body.innerText.includes("添加密钥"),
    null,
    { timeout: 10000 },
  );

  const body = await page.locator("body").innerText();
  const screenshotPath = path.resolve(".playwright-cli", "settings-modal-web.png");
  await page.screenshot({ path: screenshotPath, fullPage: true });

  const payload = {
    ok: true,
    verified: {
      settingsModalVisible: generalBody.includes("设置") && generalBody.includes("通用"),
      accountTabVisible: accountBody.includes("账号与同步"),
      aiTabVisible: aiBody.includes("AI 助手"),
      sshKeysTabVisible: body.includes("SSH 密钥"),
      cloudSyncFieldsVisible: accountBody.includes("同步服务地址") && accountBody.includes("组织范围"),
      aiProviderFieldsVisible: aiBody.includes("提供商类型") && aiBody.includes("模型名称"),
      sshKeysActionsVisible: body.includes("SSH 密钥") && body.includes("添加密钥"),
    },
    screenshotPath,
    bodySnippet: body.slice(0, 2400),
  };

  await writeFile(
    path.resolve(".playwright-cli", "settings-modal-web.json"),
    `${JSON.stringify(payload, null, 2)}\n`,
    "utf8",
  );

  console.log(JSON.stringify(payload, null, 2));
  await browser.close();
  await previewServer?.stop();
}

main()
  .then(() => exitSuccess())
  .catch((error) => {
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
  void exitFailure();
});
