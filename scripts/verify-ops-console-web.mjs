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
        label: "verify-ops-console-web",
      })).baseUrl;

  try {
    const { module: playwright } = await loadPlaywright();
    const { chromium } = playwright;
    browser = await chromium.launch(
      chromiumLaunchOptions(chromium, { headless: true }),
    );
    const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });

    await page.addInitScript(() => {
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
        assets: [
          {
            id: 1000,
            cloudId: null,
            name: "Ops Asset",
            host: "10.10.10.10",
            port: 22,
            platform: "Linux",
            folderId: null,
            envId: null,
            labels: ["prod"],
            owner: "ops-owner",
            criticality: "critical",
            defaultWorkspacePath: "/srv/app",
            accessEndpointId: 2000,
            bastionChainId: "",
            healthSummary: "healthy",
            lastAccessedAt: null,
            isFavorite: false,
            groupId: null,
          },
        ],
        folders: [],
        environments: [],
        tags: [],
        savedViews: [],
        accessHistory: [],
        endpoints: [],
        credentialRefs: [],
        jobTemplates: [],
        jobRuns: [],
        jobArchives: [],
        auditEvents: [],
        syncState: null,
        syncOverview: {
          totalAssets: 1,
          pendingChanges: 0,
          syncedAssets: 1,
          failedChanges: 0,
          lastPulledAt: null,
          lastPushedAt: null,
        },
        syncServices: [],
        syncChanges: [],
        lastOpsConsoleAnswer: null,
        lastJobBatchPreview: null,
        lastJobBatchResult: null,
        consoleQueries: [],
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
              return clone(state.assets);
            case "asset_get_asset_folders":
            case "asset_get_environments":
            case "asset_get_asset_tags":
            case "asset_get_saved_views":
            case "asset_get_access_history":
              return [];
            case "access_get_access_endpoints":
            case "access_get_credential_refs":
            case "session_get_ops_sessions":
            case "get_transfers":
            case "get_ssh_keys":
              return [];
            case "sync_get_state":
              return null;
            case "sync_get_overview":
              return clone(state.syncOverview);
            case "sync_list_services":
              return clone(state.syncServices);
            case "sync_list_changes":
              return clone(state.syncChanges);
            case "ops_list_job_templates":
              return clone(state.jobTemplates);
            case "ops_list_job_runs":
              return clone(state.jobRuns);
            case "ops_list_job_archives":
              return clone(state.jobArchives);
            case "audit_list_events":
              return clone(state.auditEvents);
            case "ops_console_query": {
              const query = String(args?.query || "");
              state.consoleQueries.push({
                query,
                selectedAssetId: args?.selectedAssetId ?? null,
              });
              state.lastOpsConsoleAnswer = {
                summary: `Found issues related to: ${query}`,
                statusExplanation: "Primary service is healthy but backlog is increasing.",
                matchedAssets: [
                  {
                    assetId: 1000,
                    assetName: "Ops Asset",
                    host: "10.10.10.10",
                    criticality: "critical",
                    environmentName: "Production",
                    matchReason: "Matched by service and risk profile.",
                    healthSummary: "healthy",
                  },
                ],
                recommendedChecks: [
                  "Check queue depth",
                  "Inspect recent deploy history",
                ],
                planSteps: [
                  {
                    id: "step-1",
                    title: "Check queue length",
                    description: "Verify queue backlog before restarting services.",
                    command: "systemctl status app",
                    riskLevel: "medium",
                    requiresConfirmation: false,
                    targetAssetName: "Ops Asset",
                  },
                ],
                reviewChecklist: [
                  "Confirm backlog trend",
                  "Review alert noise before remediation",
                ],
              };
              return clone(state.lastOpsConsoleAnswer);
            }
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
      window.__OPS_CONSOLE_TEST_STATE__ = state;
    });

    await page.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => document.body.innerText.includes("Switch"),
      null,
      { timeout: 15000 },
    );
    await page.getByRole("button", { name: "运维工作台" }).click();
    await page.getByTestId("ops-workbench-root").waitFor({ timeout: 15000 });

    const consoleTab = page.getByTestId("ops-tab-console");
    await consoleTab.click();
    await page.getByTestId("ops-console-panel").waitFor({ timeout: 10000 });

    await page.getByTestId("ops-console-run").click();
    await page.waitForFunction(
      () => document.body.innerText.includes("请输入 Ops Console 查询内容"),
      null,
      { timeout: 10000 },
    );
    const validationShown = (await page.locator("body").innerText()).includes(
      "请输入 Ops Console 查询内容",
    );

    await page.getByTestId("ops-console-query").fill("database backlog");
    await page.getByTestId("ops-console-asset-select").selectOption("1000");
    await page.getByTestId("ops-console-run").click();
    await page.getByTestId("ops-console-answer").waitFor({ timeout: 10000 });

    const body = await page.locator("body").innerText();
    const state = await page.evaluate(() => window.__OPS_CONSOLE_TEST_STATE__);
    const screenshotPath = path.resolve(".playwright-cli", "ops-console-web.png");
    await page.screenshot({ path: screenshotPath, fullPage: true });

    const payload = {
      ok: true,
      verified: {
        validationShown,
        answerVisible:
          body.includes("Found issues related to: database backlog") &&
          body.includes("Ops Asset") &&
          body.includes("Check queue depth") &&
          body.includes("Check queue length"),
        queryInvoked:
          state.consoleQueries.length === 1 &&
          state.consoleQueries[0]?.query === "database backlog" &&
          state.consoleQueries[0]?.selectedAssetId === 1000,
      },
      state,
      screenshotPath,
      bodySnippet: body.slice(0, 2600),
    };

    await writeFile(
      path.resolve(".playwright-cli", "ops-console-web.json"),
      `${JSON.stringify(payload, null, 2)}\n`,
      "utf8",
    );

    console.log(JSON.stringify(payload, null, 2));
  } finally {
    await browser?.close().catch(() => {});
    await previewServer?.stop().catch(() => {});
  }
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
