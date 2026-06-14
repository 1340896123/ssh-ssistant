import { writeFile } from "node:fs/promises";
import path from "node:path";
import { loadPlaywright, chromiumLaunchOptions } from "./helpers/playwright-runtime.mjs";
import { startPreviewServer } from "./helpers/web-preview.mjs";
import { exitFailure, exitSuccess } from "./helpers/script-exit.mjs";

const DEFAULT_WEB_APP_URL =
  process.env.SSH_ASSISTANT_WEB_APP_URL || "http://127.0.0.1:4173";

async function openFilesContext(page) {
  const primaryTab = page.getByTestId("context-tab-files");
  if ((await primaryTab.count()) > 0) {
    await primaryTab.waitFor({ state: "visible", timeout: 10000 });
    await primaryTab.click();
    await page.waitForTimeout(300);
    return;
  }

  const drawerTab = page.getByTestId("drawer-context-tab-files");
  if ((await drawerTab.count()) > 0) {
    await drawerTab.waitFor({ state: "visible", timeout: 10000 });
    await drawerTab.click();
    await page.waitForTimeout(300);
    return;
  }

  const fallbackTab = page.getByRole("button", { name: "Files" });
  await fallbackTab.waitFor({ state: "visible", timeout: 10000 });
  await fallbackTab.click();
  await page.waitForTimeout(300);
}

async function main() {
  let previewServer = null;
  let browser = null;
  const webAppUrl = process.env.SSH_ASSISTANT_WEB_APP_URL
    ? DEFAULT_WEB_APP_URL
    : (previewServer = await startPreviewServer({
        port: 4173,
        label: "verify-file-upload-web",
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
            viewMode: "flat",
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
            name: "Editor Asset",
            host: "10.10.10.10",
            port: 22,
            platform: "Linux",
            folderId: null,
            envId: null,
            labels: ["editor"],
            owner: "qa-owner",
            criticality: "medium",
            defaultWorkspacePath: "/home/root",
            accessEndpointId: 2000,
            bastionChainId: "",
            healthSummary: "",
            lastAccessedAt: null,
            isFavorite: false,
            groupId: null,
          },
        ],
        folders: [],
        environments: [],
        tags: [],
        savedViews: [],
        endpoints: [
          {
            id: 2000,
            assetId: 1000,
            name: "默认端点",
            host: "10.10.10.10",
            port: 22,
            username: "root",
            authType: "password",
            credentialRefId: 3000,
            sshKeyId: null,
            jumpHost: null,
            jumpPort: null,
            jumpUsername: null,
            jumpPassword: null,
          },
        ],
        credentialRefs: [
          {
            id: 3000,
            name: "Editor Asset credential",
            credentialKind: "password",
            username: "root",
            secret: "secret-123",
            sshKeyId: null,
            assetId: 1000,
            createdAt: Date.now(),
            updatedAt: Date.now(),
          },
        ],
        accessHistory: [],
        syncState: null,
        sessions: [
          {
            id: "session-4000",
            assetId: 1000,
            assetName: "Editor Asset",
            createdAt: Date.now(),
            accessEndpointId: 2000,
            credentialRefId: 3000,
            bastionChainId: "",
            currentPath: "/home/root",
            riskLevel: "medium",
            healthSummary: "",
            lastJobRunId: null,
            status: "connected",
            activeTab: "terminal",
            files: [],
            connectedAt: Date.now(),
            envId: null,
            os: "Linux",
          },
        ],
        sshKeys: [],
        transfers: [],
        uploadCalls: [],
        fileEntries: [
          {
            name: "app-config.json",
            isDir: false,
            size: 128,
            mtime: Date.now(),
            permissions: 644,
            uid: 0,
            owner: "root",
          },
        ],
      };

      const clone = (value) => JSON.parse(JSON.stringify(value));

      window.localStorage.setItem("preferred-locale", "zh");
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
              if (args?.settings?.fileManager) {
                state.settings.fileManager = {
                  ...state.settings.fileManager,
                  ...args.settings.fileManager,
                };
              }
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
              return clone(state.endpoints);
            case "access_get_credential_refs":
              return clone(state.credentialRefs);
            case "session_get_ops_sessions":
              return clone(state.sessions);
            case "get_transfers":
              return clone(state.transfers);
            case "get_ssh_keys":
              return [];
            case "sync_get_state":
            case "get_local_workspace_snapshot":
            case "save_local_workspace_snapshot":
            case "asset_import_legacy_client_state":
            case "asset_touch_host_asset":
              return null;
            case "get_working_directory":
              return "/home/root";
            case "list_files_page":
              return clone({
                entries: state.fileEntries,
                nextCursor: null,
                hasMore: false,
              });
            case "list_files":
              return clone(state.fileEntries);
            case "plugin:dialog|open":
              return ["C:\\uploads\\deploy.sh"];
            case "upload_file":
              state.uploadCalls.push({
                localPath: String(args?.localPath || ""),
                remotePath: String(args?.remotePath || ""),
                transferId: String(args?.transferId || ""),
              });
              return null;
            case "download_file":
            case "read_remote_file":
            case "write_remote_file":
            case "resize_pty":
            case "write_to_pty":
            case "write_binary_to_pty":
            case "cancel_transfer":
            case "pause_transfer":
            case "resume_transfer":
              return null;
            case "exec_command": {
              const command = String(args?.command || "");
              if (command === "pwd") return "/home/root\n";
              if (command === "echo $SHELL") return "/bin/bash\n";
              if (command.includes("compgen -c")) return "ls\ncat\npwd\n";
              if (command === "which tree") return "/usr/bin/tree\n";
              if (command.includes("tree -L 2 --noreport")) return ".\n";
              return "";
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
        unregisterListener: async () => {},
        listeners,
      };
      window.__MOCK_TAURI_READY__ = true;
      window.open = () => null;
      window.confirm = () => true;
      window.alert = () => {};
      window.ResizeObserver = class {
        observe() {}
        unobserve() {}
        disconnect() {}
      };
      if (!document.fonts) {
        document.fonts = { ready: Promise.resolve() };
      }
      window.__FILE_UPLOAD_STATE__ = state;
    });

    await page.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => document.body.innerText.includes("Editor Asset"),
      null,
      { timeout: 15000 },
    );

    await openFilesContext(page);
    await page.waitForFunction(
      () =>
        document.body.innerText.includes("app-config.json") &&
        document.querySelector('[data-testid="file-manager-upload-file"]'),
      null,
      { timeout: 30000 },
    );

    await page.getByTestId("file-manager-upload-file").click();
    await page.getByTestId("transfer-list-root").waitFor({ timeout: 10000 });
    await page.getByTestId("transfer-list-header").click();
    await page.getByTestId("transfer-list-body").waitFor({ timeout: 10000 });

    const body = await page.locator("body").innerText();
    const uploadCalls = await page.evaluate(() => {
      const globalState = window.__FILE_UPLOAD_STATE__;
      return globalState?.uploadCalls ?? [];
    }).catch(() => []);
    const screenshotPath = path.resolve(".playwright-cli", "file-upload-web.png");
    await page.screenshot({ path: screenshotPath, fullPage: true });

    const lastUpload = uploadCalls[uploadCalls.length - 1] ?? null;
    const verified = {
      transferListVisible: body.includes("传输列表"),
      uploadItemVisible: body.includes("deploy.sh"),
      uploadStatusVisible: body.includes("运行中") || body.includes("待处理"),
      uploadInvoked: Boolean(lastUpload),
      remotePathDerived: lastUpload?.remotePath === "/home/root/deploy.sh",
      localPathPreserved: lastUpload?.localPath === "C:\\uploads\\deploy.sh",
    };

    const failedChecks = Object.entries(verified)
      .filter(([, passed]) => !passed)
      .map(([name]) => name);
    if (failedChecks.length > 0) {
      throw new Error(
        `Upload verification failed: ${failedChecks.join(", ")}. Last upload: ${JSON.stringify(lastUpload)}. Body: ${body.slice(0, 2400)}`,
      );
    }

    const payload = {
      ok: true,
      verified,
      lastUpload,
      screenshotPath,
      bodySnippet: body.slice(0, 2600),
    };

    await writeFile(
      path.resolve(".playwright-cli", "file-upload-web.json"),
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
