import { writeFile } from "node:fs/promises";
import path from "node:path";
import { loadPlaywright, chromiumLaunchOptions } from "./helpers/playwright-runtime.mjs";
import { startPreviewServer } from "./helpers/web-preview.mjs";
import { exitFailure, exitSuccess } from "./helpers/script-exit.mjs";

const DEFAULT_WEB_APP_URL =
  process.env.SSH_ASSISTANT_WEB_APP_URL || "http://127.0.0.1:4173";

async function main() {
  let previewServer = null;
  let browser = null;
  const webAppUrl = process.env.SSH_ASSISTANT_WEB_APP_URL
    ? DEFAULT_WEB_APP_URL
    : (previewServer = await startPreviewServer({
        port: 4173,
        label: "verify-connection-test-web",
      })).baseUrl;

  try {
    const { module: playwright } = await loadPlaywright();
    const { chromium } = playwright;
    browser = await chromium.launch(
      chromiumLaunchOptions(chromium, { headless: true }),
    );

    const { successPage, failurePage } = await setupPages(browser, webAppUrl);

    const successResult = await runSuccessAndValidationFlow(successPage);
    const failureResult = await runFailureFlow(failurePage);

    const payload = {
      ok: true,
      verified: {
        localValidationShown: successResult.localValidationShown,
        successBranch: successResult.successBranch,
        failureBranch: failureResult.failureBranch,
      },
      successResult,
      failureResult,
    };

    await writeFile(
      path.resolve(".playwright-cli", "connection-test-web.json"),
      `${JSON.stringify(payload, null, 2)}\n`,
      "utf8",
    );

    console.log(JSON.stringify(payload, null, 2));
  } finally {
    await browser?.close().catch(() => {});
    await previewServer?.stop().catch(() => {});
  }
}

async function setupPages(browser, webAppUrl) {
  const successPage = await browser.newPage({
    viewport: { width: 1440, height: 960 },
  });
  await bootstrap(successPage, "success");
  await successPage.goto(webAppUrl, { waitUntil: "domcontentloaded" });
  await successPage.getByTestId("connection-list-root").waitFor({ timeout: 15000 });

  const failurePage = await browser.newPage({
    viewport: { width: 1440, height: 960 },
  });
  await bootstrap(failurePage, "failure");
  await failurePage.goto(webAppUrl, { waitUntil: "domcontentloaded" });
  await failurePage.getByTestId("connection-list-root").waitFor({ timeout: 15000 });

  return { successPage, failurePage };
}

async function runSuccessAndValidationFlow(page) {
  await page.getByTestId("connection-list-new").click();
  await page.getByTestId("connection-modal").waitFor({ timeout: 10000 });

  await page.getByTestId("connection-modal-name").fill("QA Node");
  await page.getByTestId("connection-modal-host").fill("");
  await page.getByTestId("connection-modal-test").click();
  await page.waitForFunction(
    () => document.body.innerText.includes("主机和端点用户名为必填项"),
    null,
    { timeout: 10000 },
  );

  const localValidationShown = (await page.locator("body").innerText()).includes(
    "主机和端点用户名为必填项",
  );

  await page.getByTestId("connection-modal-host").fill("10.10.10.10");
  await page.getByTestId("connection-modal-username").fill("root");
  await page.getByTestId("connection-modal-password").fill("secret-123");
  await page.getByTestId("connection-modal-test").click();
  await page.waitForFunction(
    () => document.body.innerText.includes("连接成功！"),
    null,
    { timeout: 10000 },
  );

  const successBody = await page.locator("body").innerText();
  const successState = await page.evaluate(() => window.__CONNECTION_TEST_STATE__);
  const successScreenshotPath = path.resolve(
    ".playwright-cli",
    "connection-test-success-web.png",
  );
  await page.screenshot({ path: successScreenshotPath, fullPage: true });

  return {
    localValidationShown,
    successBranch:
      successBody.includes("连接成功！") &&
      successState.testCalls.length === 1 &&
      successState.testCalls[0]?.host === "10.10.10.10" &&
      successState.testCalls[0]?.username === "root",
    successBodySnippet: successBody.slice(0, 2400),
    successState,
    successScreenshotPath,
  };
}

async function runFailureFlow(page) {
  await page.getByTestId("connection-list-new").click();
  await page.getByTestId("connection-modal").waitFor({ timeout: 10000 });

  await page.getByTestId("connection-modal-name").fill("Failure Node");
  await page.getByTestId("connection-modal-host").fill("10.10.10.11");
  await page.getByTestId("connection-modal-username").fill("broken");
  await page.getByTestId("connection-modal-password").fill("wrong-secret");
  await page.getByTestId("connection-modal-test").click();
  await page.waitForFunction(
    () => document.body.innerText.includes("Error: Unable to connect to remote host"),
    null,
    { timeout: 10000 },
  );

  const failureBody = await page.locator("body").innerText();
  const failureState = await page.evaluate(() => window.__CONNECTION_TEST_STATE__);
  const failureScreenshotPath = path.resolve(
    ".playwright-cli",
    "connection-test-failure-web.png",
  );
  await page.screenshot({ path: failureScreenshotPath, fullPage: true });

  return {
    failureBranch:
      failureBody.includes("Error: Unable to connect to remote host") &&
      failureState.testCalls.length === 1 &&
      failureState.testCalls[0]?.host === "10.10.10.11" &&
      failureState.testCalls[0]?.username === "broken",
    failureBodySnippet: failureBody.slice(0, 2400),
    failureState,
    failureScreenshotPath,
  };
}

async function bootstrap(page, mode) {
  await page.addInitScript((testMode) => {
    const listeners = new Map();
    const state = {
      settings: {
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
      },
      assets: [],
      folders: [],
      environments: [],
      tags: [],
      savedViews: [],
      endpoints: [],
      credentialRefs: [],
      accessHistory: [],
      syncState: null,
      sessions: [],
      sshKeys: [],
      transfers: [],
      testCalls: [],
    };

    window.localStorage.setItem("preferred-locale", "zh");
    const clone = (value) => JSON.parse(JSON.stringify(value));

    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        switch (cmd) {
          case "get_settings":
            return clone(state.settings);
          case "save_settings":
            state.settings = {
              ...state.settings,
              ...(args?.settings || {}),
            };
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
          case "asset_import_legacy_client_state":
          case "asset_touch_host_asset":
            return null;
          case "test_connection":
            state.testCalls.push(clone(args?.config || {}));
            if (testMode === "failure") {
              throw new Error("Error: Unable to connect to remote host");
            }
            return "ok";
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
    window.__CONNECTION_TEST_STATE__ = state;
  }, mode);
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
