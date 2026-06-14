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
        label: "verify-asset-edit-web",
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
      const now = Date.now();
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
            name: "Editable Asset",
            host: "10.10.10.10",
            port: 22,
            platform: "Linux",
            folderId: null,
            envId: null,
            labels: ["legacy", "web"],
            owner: "qa-owner",
            criticality: "medium",
            defaultWorkspacePath: "/home/root",
            accessEndpointId: 2000,
            bastionChainId: "",
            healthSummary: "healthy",
            lastAccessedAt: null,
            isFavorite: false,
            groupId: null,
            createdAt: now,
            updatedAt: now,
          },
          {
            id: 1001,
            cloudId: null,
            name: "Sibling Asset",
            host: "10.10.10.11",
            port: 22,
            platform: "Linux",
            folderId: null,
            envId: null,
            labels: ["backup"],
            owner: "qa-backup",
            criticality: "low",
            defaultWorkspacePath: "/srv",
            accessEndpointId: 2001,
            bastionChainId: "",
            healthSummary: "",
            lastAccessedAt: null,
            isFavorite: false,
            groupId: null,
            createdAt: now,
            updatedAt: now,
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
          {
            id: 2001,
            assetId: 1001,
            name: "默认端点",
            host: "10.10.10.11",
            port: 22,
            username: "ops",
            authType: "password",
            credentialRefId: 3001,
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
            name: "Editable Asset credential",
            credentialKind: "password",
            username: "root",
            secret: "secret-123",
            sshKeyId: null,
            assetId: 1000,
            createdAt: now,
            updatedAt: now,
          },
          {
            id: 3001,
            name: "Sibling Asset credential",
            credentialKind: "password",
            username: "ops",
            secret: "secret-456",
            sshKeyId: null,
            assetId: 1001,
            createdAt: now,
            updatedAt: now,
          },
        ],
        accessHistory: [],
        syncState: null,
        sessions: [],
        sshKeys: [],
        transfers: [],
        updateCalls: [],
      };

      window.localStorage.setItem("preferred-locale", "zh");
      const clone = (value) => JSON.parse(JSON.stringify(value));

      function applyAssetUpdate(payload) {
        const assetPayload = payload?.asset || {};
        const endpointPayload = payload?.defaultAccessEndpoint || {};
        const credentialPayload = payload?.defaultCredentialRef || null;
        const assetId = Number(assetPayload.id || 0);
        const assetIndex = state.assets.findIndex((asset) => asset.id === assetId);
        if (assetIndex === -1) {
          throw new Error(`Asset ${assetId} not found`);
        }

        const currentAsset = state.assets[assetIndex];
        const updatedAt = Date.now();
        const nextAsset = {
          ...currentAsset,
          ...assetPayload,
          id: currentAsset.id,
          accessEndpointId:
            endpointPayload?.id ?? currentAsset.accessEndpointId ?? null,
          folderId: assetPayload.folderId ?? assetPayload.groupId ?? currentAsset.folderId ?? null,
          groupId: assetPayload.folderId ?? assetPayload.groupId ?? currentAsset.groupId ?? null,
          updatedAt,
        };
        state.assets[assetIndex] = nextAsset;

        const endpointIndex = state.endpoints.findIndex(
          (endpoint) => endpoint.assetId === assetId,
        );
        if (endpointIndex >= 0) {
          state.endpoints[endpointIndex] = {
            ...state.endpoints[endpointIndex],
            ...endpointPayload,
            id: state.endpoints[endpointIndex].id,
            assetId,
            credentialRefId:
              credentialPayload?.id ??
              endpointPayload?.credentialRefId ??
              state.endpoints[endpointIndex].credentialRefId ??
              null,
          };
        }

        if (credentialPayload) {
          const credentialIndex = state.credentialRefs.findIndex(
            (credential) => credential.assetId === assetId,
          );
          if (credentialIndex >= 0) {
            state.credentialRefs[credentialIndex] = {
              ...state.credentialRefs[credentialIndex],
              ...credentialPayload,
              id: state.credentialRefs[credentialIndex].id,
              assetId,
              updatedAt,
            };
          }
        }

        state.updateCalls.push(clone(payload));
        return clone(nextAsset);
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
            case "asset_update_host_asset":
              return applyAssetUpdate(args?.payload || {});
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
      window.ResizeObserver = class {
        observe() {}
        unobserve() {}
        disconnect() {}
      };
      if (!document.fonts) {
        document.fonts = { ready: Promise.resolve() };
      }
      window.__ASSET_EDIT_STATE__ = state;
    });

    await page.goto(webAppUrl, { waitUntil: "domcontentloaded" });
    await page.getByTestId("asset-center-root").waitFor({ timeout: 15000 });
    await page.waitForFunction(
      () => document.body.innerText.includes("Editable Asset"),
      null,
      { timeout: 15000 },
    );

    const targetName = "Editable Asset";
    const updatedName = "Edited Asset";
    const updatedOwner = "platform-owner";
    const updatedLabels = "edited, qa";
    const updatedUsername = "admin";
    const updatedPassword = "rotated-secret";

    await page.getByTestId("asset-center-search").fill(targetName);
    await page.getByTestId("asset-center-search-results").waitFor({
      timeout: 10000,
    });
    await page.waitForFunction(
      (expectedName) =>
        document.querySelectorAll(
          '[data-testid="asset-center-search-result-card"]',
        ).length > 0 &&
        document.body.innerText.includes(expectedName),
      targetName,
      { timeout: 10000 },
    );

    const searchResultsHtml = await page
      .getByTestId("asset-center-search-results")
      .innerHTML();
    const searchCards = page
      .locator('[data-testid="asset-center-search-result-card"]')
      .filter({ hasText: targetName });
    const editButtons = searchCards.getByTestId("asset-center-search-result-edit");
    const editButtonCount = await editButtons.count();
    if (editButtonCount === 0) {
      throw new Error(
        `No asset edit buttons rendered for ${targetName}. Search HTML: ${searchResultsHtml.slice(0, 1600)}`,
      );
    }

    await editButtons.first().click();

    const modal = page.getByTestId("connection-modal");
    await modal.waitFor({ timeout: 10000 });

    const initialValues = {
      name: await page.getByTestId("connection-modal-name").inputValue(),
      owner: await page.getByTestId("connection-modal-owner").inputValue(),
      labels: await page.getByTestId("connection-modal-labels").inputValue(),
      username: await page.getByTestId("connection-modal-username").inputValue(),
      password: await page.getByTestId("connection-modal-password").inputValue(),
    };

    await page.getByTestId("connection-modal-name").fill(updatedName);
    await page.getByTestId("connection-modal-owner").fill(updatedOwner);
    await page.getByTestId("connection-modal-labels").fill(updatedLabels);
    await page.getByTestId("connection-modal-username").fill(updatedUsername);
    await page.getByTestId("connection-modal-password").fill(updatedPassword);
    await page.getByTestId("connection-modal-save").click();

    await modal.waitFor({ state: "hidden", timeout: 10000 });
    await page.getByTestId("asset-center-search").fill(updatedName);
    await page.getByTestId("asset-center-search-results").waitFor({
      timeout: 10000,
    });
    await page.waitForFunction(
      ({ name, username, owner }) =>
        document.body.innerText.includes(name) &&
        document.body.innerText.includes(`${username}@10.10.10.10`) &&
        document.body.innerText.includes(owner),
      {
        name: updatedName,
        username: updatedUsername,
        owner: updatedOwner,
      },
      { timeout: 10000 },
    );

    const persisted = await page.evaluate(() => window.__ASSET_EDIT_STATE__);
    const updatedAsset =
      persisted.assets.find((asset) => asset.id === 1000) ?? null;
    const updatedEndpoint =
      persisted.endpoints.find((endpoint) => endpoint.assetId === 1000) ?? null;
    const updatedCredential =
      persisted.credentialRefs.find((credential) => credential.assetId === 1000) ??
      null;
    const screenshotPath = path.resolve(".playwright-cli", "asset-edit-web.png");
    await page.screenshot({ path: screenshotPath, fullPage: true });

    const verified = {
      modalOpened: true,
      initialValuesLoaded:
        initialValues.name === "Editable Asset" &&
        initialValues.owner === "qa-owner" &&
        initialValues.labels === "legacy, web" &&
        initialValues.username === "root" &&
        initialValues.password === "secret-123",
      assetUpdated: updatedAsset?.name === updatedName,
      ownerUpdated: updatedAsset?.owner === updatedOwner,
      labelsUpdated:
        JSON.stringify(updatedAsset?.labels ?? []) ===
        JSON.stringify(["edited", "qa"]),
      endpointUpdated: updatedEndpoint?.username === updatedUsername,
      credentialUpdated: updatedCredential?.secret === updatedPassword,
      searchViewUpdated:
        (await page.locator("body").innerText()).includes(updatedName) &&
        (await page.locator("body").innerText()).includes(`${updatedUsername}@10.10.10.10`),
      updateInvoked: persisted.updateCalls.length === 1,
    };

    const failedChecks = Object.entries(verified)
      .filter(([, passed]) => !passed)
      .map(([name]) => name);
    if (failedChecks.length > 0) {
      throw new Error(
        `Asset edit verification failed: ${failedChecks.join(", ")}. Persisted: ${JSON.stringify({
          asset: updatedAsset,
          endpoint: updatedEndpoint,
          credential: updatedCredential,
          updateCalls: persisted.updateCalls,
          initialValues,
        })}`,
      );
    }

    const payload = {
      ok: true,
      verified,
      initialValues,
      updated: {
        asset: updatedAsset,
        endpoint: updatedEndpoint,
        credential: updatedCredential,
      },
      updateCalls: persisted.updateCalls,
      searchResultsHtml,
      editButtonCount,
      screenshotPath,
    };

    await writeFile(
      path.resolve(".playwright-cli", "asset-edit-web.json"),
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
