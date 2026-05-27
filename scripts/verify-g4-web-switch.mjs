import { writeFile } from "node:fs/promises";

const PLAYWRIGHT_MODULE = "file:///C:/Users/jieok/AppData/Roaming/npm/node_modules/playwright/index.mjs";

async function main() {
  const { chromium } = await import(PLAYWRIGHT_MODULE);
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });

  await page.addInitScript(() => {
    const listeners = new Map();
    const noopAsync = async () => null;

    const settingsState = {
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

    window.localStorage.setItem("preferred-locale", "zh");

    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        switch (cmd) {
          case "get_settings":
            return JSON.parse(JSON.stringify(settingsState));
          case "save_settings":
            Object.assign(settingsState, args?.settings || {});
            return null;
          case "get_local_workspace_snapshot":
            return null;
          case "save_local_workspace_snapshot":
            return null;
          case "asset_clear_workspace":
          case "asset_export_local_workspace_snapshot":
          case "asset_restore_local_workspace_snapshot":
          case "asset_get_host_assets":
          case "asset_get_asset_folders":
          case "asset_get_environments":
          case "asset_get_asset_tags":
          case "asset_get_saved_views":
          case "asset_get_access_history":
          case "access_get_access_endpoints":
          case "access_get_credential_refs":
          case "sync_get_state":
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
    window.__G4_MOCK_STATE__ = settingsState;
    window.__G4_NOOP_ASYNC__ = noopAsync;
  });

  await page.goto("http://127.0.0.1:4173", { waitUntil: "networkidle" });
  const before = await page.locator("body").innerText();

  await page.getByRole("button", { name: "Switch" }).click();
  await page.waitForTimeout(1200);

  const after = await page.locator("body").innerText();
  const screenshotPath = ".playwright-cli/g4-web-switch-flow.png";
  await page.screenshot({ path: screenshotPath, fullPage: true });

  const payload = {
    ok: true,
    verified: {
      beforeIncludesLocalStatus:
        before.includes("Local Workspace · local") &&
        before.includes("Free · local-only"),
      afterShowsLoginGateway:
        after.includes("三种账号模式统一登录") &&
        after.includes("登录工作台") &&
        after.includes("切换为本地模式"),
    },
    screenshotPath,
    beforeSnippet: before.slice(0, 1200),
    afterSnippet: after.slice(0, 1200),
  };

  await writeFile(
    ".playwright-cli/g4-web-switch-flow.json",
    `${JSON.stringify(payload, null, 2)}\n`,
    "utf8",
  );

  console.log(JSON.stringify(payload, null, 2));
  await browser.close();
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
