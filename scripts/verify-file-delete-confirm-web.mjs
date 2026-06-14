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
    await primaryTab.click();
    return;
  }

  const drawerTab = page.getByTestId("drawer-context-tab-files");
  if ((await drawerTab.count()) > 0) {
    await drawerTab.click();
    return;
  }

  await page.getByRole("button", { name: "Files" }).click();
}

function buildMockState(askResult) {
  return {
    askResult,
    askCalls: [],
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
    deletions: [],
  };
}

async function bootstrapPage(page, askResult) {
  await page.addInitScript((result) => {
    const listeners = new Map();
    const state = {
      askResult: result,
      askCalls: [],
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
      deletions: [],
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
          case "delete_item": {
            const remotePath = String(args?.path || "");
            const name = remotePath.split("/").pop();
            const index = state.fileEntries.findIndex((item) => item.name === name);
            if (index === -1) {
              throw new Error(`Delete target not found: ${remotePath}`);
            }
            state.fileEntries.splice(index, 1);
            state.deletions.push({
              path: remotePath,
              isDir: Boolean(args?.isDir),
            });
            return null;
          }
          case "plugin:dialog|ask":
            state.askCalls.push({
              message: String(args?.message || ""),
              title: String(args?.title || ""),
              kind: String(args?.kind || ""),
            });
            return state.askResult;
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
    window.__FILE_MANAGER_DELETE_CONFIRM_STATE__ = state;
  }, askResult);
}

async function openFilesAndDeleteViaContext(page) {
  await page.goto(DEFAULT_WEB_APP_URL, { waitUntil: "domcontentloaded" });
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
    .filter({ hasText: "notes.txt" })
    .first();
  await targetRow.click();
  await targetRow.dispatchEvent("contextmenu", {
    button: 2,
    bubbles: true,
    cancelable: true,
    clientX: 180,
    clientY: 280,
  });
  await page.getByTestId("file-manager-context-menu").waitFor({ timeout: 10000 });
  await page.getByTestId("file-manager-context-delete").click();
}

async function main() {
  let previewServer = null;
  let browser = null;
  const webAppUrl = process.env.SSH_ASSISTANT_WEB_APP_URL
    ? DEFAULT_WEB_APP_URL
    : (previewServer = await startPreviewServer({
        port: 4173,
        label: "verify-file-delete-confirm-web",
      })).baseUrl;

  try {
    const { module: playwright } = await loadPlaywright();
    const { chromium } = playwright;
    browser = await chromium.launch(
      chromiumLaunchOptions(chromium, { headless: true }),
    );

    const cancelPage = await browser.newPage({
      viewport: { width: 1440, height: 960 },
    });
    await bootstrapPage(cancelPage, false);
    await cancelPage.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await cancelPage.waitForFunction(
      () => document.body.innerText.includes("Editor Asset"),
      null,
      { timeout: 15000 },
    );
    await openFilesContext(cancelPage);
    await cancelPage.waitForFunction(
      () =>
        document.body.innerText.includes("app-config.json") &&
        document.querySelectorAll('[data-testid="file-list-item"]').length >= 2,
      null,
      { timeout: 30000 },
    );
    const cancelTarget = cancelPage
      .getByTestId("file-list-item")
      .filter({ hasText: "notes.txt" })
      .first();
    await cancelTarget.click();
    await cancelTarget.dispatchEvent("contextmenu", {
      button: 2,
      bubbles: true,
      cancelable: true,
      clientX: 180,
      clientY: 280,
    });
    await cancelPage.getByTestId("file-manager-context-menu").waitFor({
      timeout: 10000,
    });
    await cancelPage.getByTestId("file-manager-context-delete").click();

    await cancelPage.waitForFunction(
      () => window.__FILE_MANAGER_DELETE_CONFIRM_STATE__.askCalls.length === 1,
      null,
      { timeout: 10000 },
    );
    const cancelState = await cancelPage.evaluate(
      () => window.__FILE_MANAGER_DELETE_CONFIRM_STATE__,
    );

    const confirmPage = await browser.newPage({
      viewport: { width: 1440, height: 960 },
    });
    await bootstrapPage(confirmPage, true);
    await confirmPage.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await confirmPage.waitForFunction(
      () => document.body.innerText.includes("Editor Asset"),
      null,
      { timeout: 15000 },
    );
    await openFilesContext(confirmPage);
    await confirmPage.waitForFunction(
      () =>
        document.body.innerText.includes("app-config.json") &&
        document.querySelectorAll('[data-testid="file-list-item"]').length >= 2,
      null,
      { timeout: 30000 },
    );
    const confirmTarget = confirmPage
      .getByTestId("file-list-item")
      .filter({ hasText: "notes.txt" })
      .first();
    await confirmTarget.click();
    await confirmTarget.dispatchEvent("contextmenu", {
      button: 2,
      bubbles: true,
      cancelable: true,
      clientX: 180,
      clientY: 280,
    });
    await confirmPage.getByTestId("file-manager-context-menu").waitFor({
      timeout: 10000,
    });
    await confirmPage.getByTestId("file-manager-context-delete").click();
    await confirmPage.waitForFunction(
      () => !document.body.innerText.includes("notes.txt"),
      null,
      { timeout: 10000 },
    );
    const confirmState = await confirmPage.evaluate(
      () => window.__FILE_MANAGER_DELETE_CONFIRM_STATE__,
    );

    const screenshotPath = path.resolve(
      ".playwright-cli",
      "file-delete-confirm-web.png",
    );
    await confirmPage.screenshot({ path: screenshotPath, fullPage: true });

    const payload = {
      ok: true,
      verified: {
        cancelBranch: {
          askInvoked: cancelState.askCalls.length === 1,
          deletePrevented: cancelState.deletions.length === 0,
          fileStillVisible: (await cancelPage.locator("body").innerText()).includes(
            "notes.txt",
          ),
          askMessageCorrect:
            cancelState.askCalls[0]?.message === "删除 1 个项目？",
        },
        confirmBranch: {
          askInvoked: confirmState.askCalls.length === 1,
          deletePerformed: confirmState.deletions.length === 1,
          fileRemoved: !(await confirmPage.locator("body").innerText()).includes(
            "notes.txt",
          ),
          survivingFileVisible: (await confirmPage.locator("body").innerText()).includes(
            "app-config.json",
          ),
        },
      },
      cancelState: {
        askCalls: cancelState.askCalls,
        deletions: cancelState.deletions,
        entries: cancelState.fileEntries,
      },
      confirmState: {
        askCalls: confirmState.askCalls,
        deletions: confirmState.deletions,
        entries: confirmState.fileEntries,
      },
      screenshotPath,
    };

    await writeFile(
      path.resolve(".playwright-cli", "file-delete-confirm-web.json"),
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
