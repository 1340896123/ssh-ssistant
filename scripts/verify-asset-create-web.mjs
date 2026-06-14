import { writeFile } from "node:fs/promises";
import path from "node:path";
import { loadPlaywright, chromiumLaunchOptions } from "./helpers/playwright-runtime.mjs";
import { startPreviewServer } from "./helpers/web-preview.mjs";
import { exitFailure, exitSuccess } from "./helpers/script-exit.mjs";

const DEFAULT_WEB_APP_URL = process.env.SSH_ASSISTANT_WEB_APP_URL || "http://127.0.0.1:4173";

async function main() {
  let previewServer = null;
  let browser = null;
  const webAppUrl = process.env.SSH_ASSISTANT_WEB_APP_URL
    ? DEFAULT_WEB_APP_URL
    : (previewServer = await startPreviewServer({
        port: 4173,
        label: "verify-asset-create-web",
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
        nextAssetId: 1000,
        nextEndpointId: 2000,
        nextCredentialId: 3000,
        nextSessionId: 4000,
      };

      window.localStorage.setItem("preferred-locale", "zh");

      const clone = (value) => JSON.parse(JSON.stringify(value));

      function createAssetBundle(payload) {
        const assetId = state.nextAssetId++;
        const endpointId = state.nextEndpointId++;
        const credentialId = payload.defaultCredentialRef ? state.nextCredentialId++ : null;

        const createdAt = Date.now();
        const asset = {
          cloudId: payload.asset.cloudId ?? null,
          name: payload.asset.name,
          host: payload.asset.host,
          port: Number(payload.asset.port || 22),
          platform: payload.asset.platform ?? "Linux",
          folderId: payload.asset.folderId ?? payload.asset.groupId ?? null,
          envId: payload.asset.envId ?? null,
          labels: payload.asset.labels ?? [],
          owner: payload.asset.owner ?? "",
          criticality: payload.asset.criticality ?? "medium",
          defaultWorkspacePath: payload.asset.defaultWorkspacePath ?? "",
          accessEndpointId: endpointId,
          bastionChainId: payload.asset.bastionChainId ?? null,
          healthSummary: payload.asset.healthSummary ?? null,
          lastAccessedAt: payload.asset.lastAccessedAt ?? null,
          isFavorite: Boolean(payload.asset.isFavorite),
          groupId: payload.asset.folderId ?? payload.asset.groupId ?? null,
          id: assetId,
          createdAt,
          updatedAt: createdAt,
        };

        const endpoint = {
          id: endpointId,
          assetId,
          name:
            payload.defaultAccessEndpoint?.name ||
            `${asset.name} default endpoint`,
          host: payload.defaultAccessEndpoint?.host || asset.host,
          port: Number(payload.defaultAccessEndpoint?.port || asset.port || 22),
          username: payload.defaultAccessEndpoint?.username || "root",
          authType: payload.defaultAccessEndpoint?.authType || "password",
          credentialRefId: credentialId,
          sshKeyId: payload.defaultAccessEndpoint?.sshKeyId ?? null,
          jumpHost: payload.defaultAccessEndpoint?.jumpHost ?? null,
          jumpPort: payload.defaultAccessEndpoint?.jumpPort ?? null,
          jumpUsername: payload.defaultAccessEndpoint?.jumpUsername ?? null,
          jumpPassword: payload.defaultAccessEndpoint?.jumpPassword ?? null,
        };

        const credentialRef = credentialId
          ? {
              id: credentialId,
              name:
                payload.defaultCredentialRef?.name ||
                `${asset.name} credential`,
              credentialKind:
                payload.defaultCredentialRef?.credentialKind || "password",
              username:
                payload.defaultCredentialRef?.username || endpoint.username,
              secret: payload.defaultCredentialRef?.secret ?? "",
              sshKeyId: payload.defaultCredentialRef?.sshKeyId ?? null,
              assetId,
              createdAt,
              updatedAt: createdAt,
            }
          : null;

        state.assets.push(asset);
        state.endpoints.push(endpoint);
        if (credentialRef) {
          state.credentialRefs.push(credentialRef);
        }

        return asset;
      }

      function createSessionForAsset(assetId, source = "search") {
        const asset = state.assets.find((item) => item.id === assetId);
        if (!asset) {
          throw new Error(`Asset ${assetId} not found`);
        }
        const sessionId = `session-${state.nextSessionId++}`;
        const session = {
          id: sessionId,
          assetId: asset.id,
          assetName: asset.name,
          createdAt: Date.now(),
          accessEndpointId: asset.accessEndpointId ?? null,
          credentialRefId:
            state.endpoints.find((endpoint) => endpoint.assetId === asset.id)
              ?.credentialRefId ?? null,
          bastionChainId: asset.bastionChainId ?? null,
          currentPath: "/home/root",
          riskLevel: asset.criticality ?? "medium",
          healthSummary: asset.healthSummary ?? null,
          lastJobRunId: null,
          status: "connected",
          activeTab: "terminal",
          files: [],
          connectedAt: Date.now(),
          envId: asset.envId ?? null,
          os: "Linux",
          source,
        };
        state.sessions = [session];
        return {
          sessionId,
          assetId: asset.id,
          assetName: asset.name,
          createdAt: session.createdAt,
          envId: asset.envId ?? null,
          accessEndpointId: session.accessEndpointId,
          credentialRefId: session.credentialRefId,
          bastionChainId: session.bastionChainId,
          riskLevel: session.riskLevel,
          healthSummary: session.healthSummary,
          osInfo: "Linux",
        };
      }

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
              return clone(state.folders);
            case "asset_get_environments":
              return clone(state.environments);
            case "asset_get_asset_tags":
              return clone(state.tags);
            case "asset_get_saved_views":
              return clone(state.savedViews);
            case "asset_get_access_history":
              return clone(state.accessHistory);
            case "access_get_access_endpoints":
              return clone(state.endpoints);
            case "access_get_credential_refs":
              return clone(state.credentialRefs);
            case "asset_create_host_asset":
              return clone(createAssetBundle(args?.payload || {}));
            case "asset_update_host_asset":
              return clone(args?.payload?.asset || null);
            case "session_connect_asset":
              return clone(
                createSessionForAsset(args?.assetId, args?.source || "search"),
              );
            case "session_disconnect_asset":
              state.sessions = state.sessions.filter(
                (session) => session.id !== args?.sessionId,
              );
              return null;
            case "asset_touch_host_asset":
            case "asset_import_legacy_client_state":
            case "asset_export_local_workspace_snapshot":
            case "asset_restore_local_workspace_snapshot":
            case "asset_clear_workspace":
            case "sync_get_state":
            case "get_local_workspace_snapshot":
            case "save_local_workspace_snapshot":
              return null;
            case "session_get_ops_sessions":
              return clone(state.sessions);
            case "get_transfers":
              return clone(state.transfers);
            case "get_ssh_keys":
              return clone(state.sshKeys);
            case "exec_command": {
              const command = String(args?.command || "");
              if (command === "pwd") {
                return "/home/root\n";
              }
              if (command === "echo $SHELL") {
                return "/bin/bash\n";
              }
              if (command === "which tree") {
                return "/usr/bin/tree\n";
              }
              if (command.includes("tree -L 2 --noreport")) {
                return ".\n";
              }
              if (command.includes("git status -s | head -n 10")) {
                return "";
              }
              if (command.includes("test -f")) {
                return "";
              }
              if (command.includes("compgen -c")) {
                return "ls\ncat\npwd\n";
              }
              return "";
            }
            case "read_remote_file":
              return "";
            case "write_to_pty":
            case "write_binary_to_pty":
            case "resize_pty":
              return null;
            case "list_files":
              return [];
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
      window.__ASSET_CREATE_STATE__ = state;
    });

    await page.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await page.getByTestId("asset-center-root").waitFor({ timeout: 15000 });
    await page.getByTestId("asset-center-empty-state").waitFor({ timeout: 10000 });

    const assetName = `Web Asset ${Date.now()}`;
    const assetHost = "10.10.10.10";
    const assetOwner = "qa-owner";
    const assetLabels = "web, smoke";
    const endpointUser = "root";
    const endpointPassword = "secret-123";

    await page.getByTestId("asset-center-new-asset").click();
    await page.getByTestId("connection-modal").waitFor({ timeout: 10000 });
    const modalVisible = await page.getByTestId("connection-modal").isVisible();

    await page.getByTestId("connection-modal-name").fill(assetName);
    await page.getByTestId("connection-modal-host").fill(assetHost);
    await page.getByTestId("connection-modal-owner").fill(assetOwner);
    await page.getByTestId("connection-modal-labels").fill(assetLabels);
    await page.getByTestId("connection-modal-username").fill(endpointUser);
    await page.getByTestId("connection-modal-password").fill(endpointPassword);
    await page.getByTestId("connection-modal-save").click();

    await page.getByTestId("asset-center-content").waitFor({ timeout: 10000 });
    await page.getByTestId("connection-modal").waitFor({
      state: "hidden",
      timeout: 10000,
    });
    await page.getByTestId("asset-center-search").fill(assetName);
    await page.getByTestId("asset-center-search-results").waitFor({
      timeout: 10000,
    });
    await page.waitForFunction(
      () =>
        document.querySelectorAll(
          '[data-testid="asset-center-search-result-connect"]',
        ).length > 0,
      null,
      { timeout: 10000 },
    );

    await page.waitForFunction(
      ({ expectedName, expectedHost }) =>
        document.body.innerText.includes(expectedName) &&
        document.body.innerText.includes(`root@${expectedHost}`),
      { expectedName: assetName, expectedHost: assetHost },
      { timeout: 10000 },
    );

    const searchPanelHtml = await page
      .getByTestId("asset-center-search-results")
      .innerHTML();
    const searchResultConnectCount = await page
      .locator('[data-testid="asset-center-search-result-connect"]')
      .count();
    if (searchResultConnectCount === 0) {
      throw new Error(
        `No search result connect buttons rendered. Search HTML: ${searchPanelHtml.slice(
          0,
          1200,
        )}`,
      );
    }

    await page
      .locator('[data-testid="asset-center-search-result-connect"]')
      .first()
      .click();

    await page.waitForFunction(
      (expectedName) =>
        document.body.innerText.includes(expectedName) &&
        document.body.innerText.includes("Linux") &&
        document.body.innerText.includes("connected"),
      assetName,
      { timeout: 10000 },
    );

    const body = await page.locator("body").innerText();
    const persisted = await page.evaluate(() => window.__ASSET_CREATE_STATE__);
    const screenshotPath = path.resolve(".playwright-cli", "asset-create-web.png");
    await page.screenshot({ path: screenshotPath, fullPage: true });

    const createdAsset =
      persisted.assets.find((asset) => asset.name === assetName) ?? null;
    const createdEndpoint =
      persisted.endpoints.find(
        (endpoint) => endpoint.assetId === createdAsset?.id,
      ) ?? null;

    const payload = {
      ok: true,
      verified: {
        modalOpened: modalVisible,
        assetPersisted: Boolean(createdAsset),
        assetVisibleInSearch: body.includes(assetName),
        endpointVisibleInSearch: body.includes(`root@${assetHost}`),
        statsUpdated: body.includes("资产 1"),
        sessionCreated: persisted.sessions.length === 1,
        sessionVisible: body.includes(assetName) && body.includes("Linux"),
      },
      created: {
        asset: createdAsset,
        endpoint: createdEndpoint,
        session: persisted.sessions[0] ?? null,
      },
      screenshotPath,
      bodySnippet: body.slice(0, 2400),
      searchPanelHtml,
      searchResultConnectCount,
    };

    await writeFile(
      path.resolve(".playwright-cli", "asset-create-web.json"),
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
