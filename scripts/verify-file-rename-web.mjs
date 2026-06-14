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
        label: "verify-file-rename-web",
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
          {
            name: "notes.txt",
            isDir: false,
            size: 42,
            mtime: Date.now(),
            permissions: 644,
            uid: 0,
            owner: "root",
          },
        ],
        renames: [],
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
            case "rename_item": {
              const oldPath = String(args?.oldPath || "");
              const newPath = String(args?.newPath || "");
              const oldName = oldPath.split("/").pop();
              const newName = newPath.split("/").pop();
              const entry = state.fileEntries.find((item) => item.name === oldName);
              if (!entry || !newName) {
                throw new Error(`Rename target not found: ${oldPath}`);
              }
              entry.name = newName;
              entry.mtime = Date.now();
              state.renames.push({ oldPath, newPath });
              return null;
            }
            case "read_remote_file":
              return "";
            case "write_remote_file":
            case "resize_pty":
            case "write_to_pty":
            case "write_binary_to_pty":
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
      window.__FILE_MANAGER_RENAME_STATE__ = state;
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
        document.querySelectorAll('[data-testid="file-list-item"]').length >= 2,
      null,
      { timeout: 30000 },
    );

    const targetRow = page
      .getByTestId("file-list-item")
      .filter({ hasText: "app-config.json" })
      .first();
    await targetRow.waitFor({ state: "visible", timeout: 10000 });
    await targetRow.click();
    await page.waitForTimeout(150);
    await targetRow.dispatchEvent("contextmenu", {
      button: 2,
      bubbles: true,
      cancelable: true,
      clientX: 180,
      clientY: 280,
    });
    try {
      await page.getByTestId("file-manager-context-menu").waitFor({
        timeout: 10000,
      });
    } catch (error) {
      const debug = await page.evaluate(() => ({
        body: document.body.innerText.slice(0, 2000),
        rowCount: document.querySelectorAll('[data-testid="file-list-item"]').length,
        menuVisible: Boolean(document.querySelector('[data-testid="file-manager-context-menu"]')),
      }));
      throw new Error(`Context menu did not appear. Debug: ${JSON.stringify(debug)}`);
    }
    await page.getByTestId("file-manager-context-rename").click();

    const renameInput = page.getByTestId("file-manager-rename-input");
    await renameInput.waitFor({ timeout: 10000 });
    await renameInput.click();
    await renameInput.fill("app-config-renamed.json");
    await renameInput.press("Enter");

    await page.waitForFunction(
      () => document.body.innerText.includes("app-config-renamed.json"),
      null,
      { timeout: 10000 },
    );
    await page.waitForFunction(
      () => !document.body.innerText.includes("app-config.json\n128 B"),
      null,
      { timeout: 10000 },
    );

    const state = await page.evaluate(() => window.__FILE_MANAGER_RENAME_STATE__);
    const screenshotPath = path.resolve(".playwright-cli", "file-rename-web.png");
    await page.screenshot({ path: screenshotPath, fullPage: true });

    const lastRename = state.renames[state.renames.length - 1] ?? null;
    const body = await page.locator("body").innerText();

    const payload = {
      ok: true,
      verified: {
        renameInputVisible: true,
        renamedFileVisible: body.includes("app-config-renamed.json"),
        oldFileHidden: !body.includes("app-config.json\n128 B"),
        renameInvoked: Boolean(lastRename),
      },
      lastRename,
      entries: state.fileEntries,
      screenshotPath,
      bodySnippet: body.slice(0, 2400),
    };

    await writeFile(
      path.resolve(".playwright-cli", "file-rename-web.json"),
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
