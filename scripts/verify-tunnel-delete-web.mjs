import { writeFile } from "node:fs/promises";
import path from "node:path";
import { loadPlaywright, chromiumLaunchOptions } from "./helpers/playwright-runtime.mjs";
import { startPreviewServer } from "./helpers/web-preview.mjs";
import { exitFailure, exitSuccess } from "./helpers/script-exit.mjs";

const DEFAULT_WEB_APP_URL =
  process.env.SSH_ASSISTANT_WEB_APP_URL || "http://127.0.0.1:4173";

function tunnelState(confirmResult) {
  return {
    confirmResult,
    confirmCalls: [],
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
        name: "Tunnel Asset",
        host: "10.10.10.10",
        port: 22,
        platform: "Linux",
        folderId: null,
        envId: null,
        labels: ["tunnel"],
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
        name: "Tunnel Asset credential",
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
        assetName: "Tunnel Asset",
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
    tunnels: [
      {
        id: 5000,
        name: "HTTP Forward",
        assetId: 1000,
        accessEndpointId: 2000,
        tunnelType: "local",
        localHost: "127.0.0.1",
        localPort: 8080,
        remoteHost: "127.0.0.1",
        remotePort: 80,
        remoteBindHost: "127.0.0.1",
      },
      {
        id: 5001,
        name: "SOCKS Proxy",
        assetId: 1000,
        accessEndpointId: 2000,
        tunnelType: "dynamic",
        localHost: "127.0.0.1",
        localPort: 1080,
        remoteBindHost: "127.0.0.1",
      },
    ],
    activeTunnelIds: [],
    deletedTunnelIds: [],
  };
}

async function bootstrap(page, confirmResult) {
  await page.addInitScript((result) => {
    const listeners = new Map();
    const state = {
      confirmResult: result,
      confirmCalls: [],
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
          name: "Tunnel Asset",
          host: "10.10.10.10",
          port: 22,
          platform: "Linux",
          folderId: null,
          envId: null,
          labels: ["tunnel"],
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
          name: "Tunnel Asset credential",
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
          assetName: "Tunnel Asset",
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
      tunnels: [
        {
          id: 5000,
          name: "HTTP Forward",
          assetId: 1000,
          accessEndpointId: 2000,
          tunnelType: "local",
          localHost: "127.0.0.1",
          localPort: 8080,
          remoteHost: "127.0.0.1",
          remotePort: 80,
          remoteBindHost: "127.0.0.1",
        },
        {
          id: 5001,
          name: "SOCKS Proxy",
          assetId: 1000,
          accessEndpointId: 2000,
          tunnelType: "dynamic",
          localHost: "127.0.0.1",
          localPort: 1080,
          remoteBindHost: "127.0.0.1",
        },
      ],
      activeTunnelIds: [],
      deletedTunnelIds: [],
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
          case "get_tunnels":
            return clone(state.tunnels);
          case "get_active_tunnels":
            return clone(state.activeTunnelIds);
          case "delete_tunnel":
            state.deletedTunnelIds.push(Number(args?.id || 0));
            state.tunnels = state.tunnels.filter(
              (item) => item.id !== Number(args?.id || 0),
            );
            return null;
          case "start_tunnel":
          case "stop_tunnel":
          case "create_tunnel":
          case "update_tunnel":
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
      unregisterListener: async () => {},
      listeners,
    };
    window.__MOCK_TAURI_READY__ = true;
    window.open = () => null;
    window.confirm = (message) => {
      state.confirmCalls.push(String(message || ""));
      return state.confirmResult;
    };
    window.alert = () => {};
    window.ResizeObserver = class {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
    if (!document.fonts) {
      document.fonts = { ready: Promise.resolve() };
    }
    window.__TUNNEL_DELETE_STATE__ = state;
  }, confirmResult);
}

async function main() {
  let previewServer = null;
  let browser = null;
  const webAppUrl = process.env.SSH_ASSISTANT_WEB_APP_URL
    ? DEFAULT_WEB_APP_URL
    : (previewServer = await startPreviewServer({
        port: 4173,
        label: "verify-tunnel-delete-web",
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
    await bootstrap(cancelPage, false);
    await cancelPage.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await cancelPage.waitForFunction(
      () => document.body.innerText.includes("Tunnel Asset"),
      null,
      { timeout: 15000 },
    );
    await cancelPage.getByTitle("隧道").click();
    await cancelPage.getByTestId("tunnel-panel-root").waitFor({ timeout: 10000 });
    await cancelPage
      .locator('[data-testid="tunnel-panel-item"][data-tunnel-name="HTTP Forward"]')
      .getByTestId("tunnel-panel-delete")
      .click();

    const cancelState = await cancelPage.evaluate(() => window.__TUNNEL_DELETE_STATE__);
    const cancelBody = await cancelPage.locator("body").innerText();

    const confirmPage = await browser.newPage({
      viewport: { width: 1440, height: 960 },
    });
    await bootstrap(confirmPage, true);
    await confirmPage.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await confirmPage.waitForFunction(
      () => document.body.innerText.includes("Tunnel Asset"),
      null,
      { timeout: 15000 },
    );
    await confirmPage.getByTitle("隧道").click();
    await confirmPage.getByTestId("tunnel-panel-root").waitFor({ timeout: 10000 });
    await confirmPage
      .locator('[data-testid="tunnel-panel-item"][data-tunnel-name="HTTP Forward"]')
      .getByTestId("tunnel-panel-delete")
      .click();
    await confirmPage.waitForFunction(
      () => !document.body.innerText.includes("HTTP Forward"),
      null,
      { timeout: 10000 },
    );

    const confirmState = await confirmPage.evaluate(() => window.__TUNNEL_DELETE_STATE__);
    const confirmBody = await confirmPage.locator("body").innerText();
    const screenshotPath = path.resolve(".playwright-cli", "tunnel-delete-web.png");
    await confirmPage.screenshot({ path: screenshotPath, fullPage: true });

    const payload = {
      ok: true,
      verified: {
        cancelBranch: {
          confirmInvoked: cancelState.confirmCalls.length === 1,
          deletePrevented: cancelState.deletedTunnelIds.length === 0,
          tunnelStillVisible: cancelBody.includes("HTTP Forward"),
        },
        confirmBranch: {
          confirmInvoked: confirmState.confirmCalls.length === 1,
          deletePerformed: confirmState.deletedTunnelIds.includes(5000),
          deletedTunnelHidden: !confirmBody.includes("HTTP Forward"),
          siblingTunnelVisible: confirmBody.includes("SOCKS Proxy"),
        },
      },
      cancelState: {
        confirmCalls: cancelState.confirmCalls,
        deletedTunnelIds: cancelState.deletedTunnelIds,
        tunnelCount: cancelState.tunnels.length,
      },
      confirmState: {
        confirmCalls: confirmState.confirmCalls,
        deletedTunnelIds: confirmState.deletedTunnelIds,
        tunnelCount: confirmState.tunnels.length,
      },
      screenshotPath,
    };

    await writeFile(
      path.resolve(".playwright-cli", "tunnel-delete-web.json"),
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
