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
        label: "verify-ops-jobs-web",
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
            labels: ["prod", "api"],
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
        previewCalls: [],
        executeCalls: [],
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
            case "ops_preview_job_batch": {
              const request = clone(args?.request || {});
              state.previewCalls.push(request);
              state.lastJobBatchPreview = {
                targetCount: 1,
                requiresConfirmation: true,
                targets: [
                  {
                    assetId: 1000,
                    assetName: "Ops Asset",
                    host: "10.10.10.10",
                    riskLevel: "critical",
                    matchReason: "Matched by tag: prod",
                    environmentName: "Production",
                    labels: ["prod", "api"],
                  },
                ],
                warnings: ["Critical asset requires confirmation."],
              };
              return clone(state.lastJobBatchPreview);
            }
            case "ops_execute_job_batch": {
              const request = clone(args?.request || {});
              state.executeCalls.push(request);
              state.lastJobBatchResult = {
                completed: 1,
                failed: 0,
                items: [
                  {
                    assetId: 1000,
                    assetName: "Ops Asset",
                    jobRunId: 5000,
                    status: "completed",
                    riskLevel: "critical",
                    usedExistingSession: false,
                    output: "batch execution ok",
                    error: null,
                  },
                ],
              };
              return clone(state.lastJobBatchResult);
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
      window.__OPS_JOBS_TEST_STATE__ = state;
    });

    await page.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => document.body.innerText.includes("Switch"),
      null,
      { timeout: 15000 },
    );
    await page.getByRole("button", { name: "运维工作台" }).click();
    await page.getByTestId("ops-workbench-root").waitFor({ timeout: 15000 });

    const jobsTab = page.getByTestId("ops-tab-jobs");
    await jobsTab.waitFor({ state: "visible", timeout: 10000 });
    await jobsTab.click();
    await page.waitForTimeout(300);
    try {
      const jobsPanel = page.getByTestId("ops-jobs-panel");
      if ((await jobsPanel.count()) > 0) {
        await jobsPanel.waitFor({ timeout: 10000 });
      } else {
        await page.getByTestId("ops-jobs-template-name").waitFor({ timeout: 10000 });
      }
    } catch (error) {
      const debug = await page.evaluate(() => ({
        body: document.body.innerText.slice(0, 3000),
        jobsTabCount: document.querySelectorAll('[data-testid="ops-tab-jobs"]').length,
        jobsPanelCount: document.querySelectorAll('[data-testid="ops-jobs-panel"]').length,
        jobsTemplateNameCount: document.querySelectorAll('[data-testid="ops-jobs-template-name"]').length,
        consolePanelCount: document.querySelectorAll('[data-testid="ops-console-panel"]').length,
      }));
      throw new Error(`Ops jobs panel did not appear. Debug: ${JSON.stringify(debug)}`);
    }

    await page.getByTestId("ops-jobs-template-name").fill("Restart API");
    await page.getByTestId("ops-jobs-command").fill("systemctl restart app");
    await page.getByTestId("ops-jobs-scope-type").selectOption("tag");
    await page.getByTestId("ops-jobs-scope-value").fill("prod");
    await page.getByTestId("ops-jobs-risk-level").selectOption("critical");
    await page.getByTestId("ops-jobs-requires-confirmation").check();

    await page.getByTestId("ops-jobs-preview").click();
    await page.getByTestId("ops-jobs-preview-result").waitFor({ timeout: 10000 });

    await page.getByTestId("ops-jobs-execute").click();
    await page.getByTestId("ops-jobs-execution-result").waitFor({ timeout: 10000 });

    const body = await page.locator("body").innerText();
    const state = await page.evaluate(() => window.__OPS_JOBS_TEST_STATE__);
    const screenshotPath = path.resolve(".playwright-cli", "ops-jobs-web.png");
    await page.screenshot({ path: screenshotPath, fullPage: true });

    const verified = {
      previewShown:
        body.includes("批量预览") &&
        body.includes("Ops Asset") &&
        body.includes("Matched by tag: prod"),
      executeShown:
        body.includes("执行复盘") &&
        body.includes("batch execution ok") &&
        body.includes("成功 1 / 失败 0"),
      previewInvoked:
        state.previewCalls.length === 1 &&
        state.previewCalls[0]?.commandText === "systemctl restart app" &&
        state.previewCalls[0]?.scopeType === "tag" &&
        state.previewCalls[0]?.scopeValue === "prod",
      executeInvoked:
        state.executeCalls.length === 1 &&
        state.executeCalls[0]?.commandText === "systemctl restart app" &&
        state.executeCalls[0]?.riskLevel === "critical",
    };

    const payload = {
      ok: Object.values(verified).every(Boolean),
      verified,
      state,
      screenshotPath,
      bodySnippet: body.slice(0, 2800),
    };

    await writeFile(
      path.resolve(".playwright-cli", "ops-jobs-web.json"),
      `${JSON.stringify(payload, null, 2)}\n`,
      "utf8",
    );

    console.log(JSON.stringify(payload, null, 2));

    if (!payload.ok) {
      throw new Error("Ops jobs verification failed");
    }
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
