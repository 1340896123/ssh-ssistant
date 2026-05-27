import { spawn } from "node:child_process";

const DEFAULT_BASE_URL = process.env.SSH_ASSISTANT_ADMIN_BASE_URL || "http://localhost:5047";
const MOCK_PORT = 5059;
const SUBSCRIPTION_STATUS_ACTIVE = 2;

function normalizeBaseUrl(url) {
  return (url || DEFAULT_BASE_URL).replace(/\/+$/, "");
}

async function requestJson(baseUrl, pathName, options = {}) {
  const response = await fetch(`${baseUrl}${pathName}`, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    },
  });

  const text = await response.text();
  const payload = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}: ${text}`);
  }
  return payload;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function buildSettingsPayload({
  account,
  sync,
  customEndpoint,
  settingsJson,
}) {
  return {
    mode: account.mode,
    accountKey: account.accountKey,
    displayName: account.displayName,
    email: account.email,
    enterpriseId: account.enterpriseId,
    enterpriseName: account.enterpriseName,
    subAccountId: account.subAccountId,
    accessToken: account.accessToken,
    syncEndpointUrl: sync.endpointUrl,
    organizationScope: sync.organizationScope,
    syncAssets: sync.syncAssets,
    syncSettings: sync.syncSettings,
    useCustomEndpoint: customEndpoint.useCustomEndpoint,
    endpointName: customEndpoint.endpointName,
    provider: customEndpoint.provider,
    baseUrl: customEndpoint.baseUrl,
    apiKey: customEndpoint.apiKey,
    modelName: customEndpoint.modelName,
    settingsJson: JSON.stringify(settingsJson),
  };
}

function nowSuffix() {
  return new Date().toISOString().replace(/[-:.TZ]/g, "").slice(0, 14);
}

function toIsoAfterDays(days) {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toISOString();
}

async function waitForServer(port, timeoutMs = 15000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    try {
      await fetch(`http://127.0.0.1:${port}/chat/completions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: "ping", messages: [] }),
      }).catch(() => null);
      return;
    } catch {
      // ignore
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
  throw new Error("Mock OpenAI server did not become ready in time.");
}

async function main() {
  const baseUrl = normalizeBaseUrl(process.argv[2]);
  const suffix = nowSuffix();
  const personalId = `usr-g4-ai-${suffix}`;
  const enterpriseId = `ent-g4-ai-${suffix}`;
  const subAccountId = `sub-g4-ai-${suffix}`;
  const personalEmail = `${personalId}@example.com`;
  const subAccountEmail = `${subAccountId}@example.com`;

  const mockServer = spawn("node", ["scripts/mock-openai-server.mjs"], {
    cwd: process.cwd(),
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });

  try {
    await waitForServer(MOCK_PORT);

    const adminLogin = await requestJson(baseUrl, "/api/admin/login", {
      method: "POST",
      body: JSON.stringify({
        username: "admin",
        password: "admin123",
      }),
    });
    const adminHeaders = {
      Authorization: `Bearer ${adminLogin.token}`,
    };

    const dashboard = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });
    const enterpriseAssets = (dashboard.assets || [])
      .filter((item) => item.ownerType === "enterprise")
      .slice(0, 2)
      .map((item) => item.id);
    assert(enterpriseAssets.length >= 1, "Expected at least one enterprise asset for enterprise AI verification.");

    await requestJson(baseUrl, "/api/admin/personal-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: personalId,
        displayName: `G4 AI Personal ${suffix}`,
        email: personalEmail,
        secret: "g4-ai-personal-pass",
        subscriptionStatus: SUBSCRIPTION_STATUS_ACTIVE,
        planName: "personal",
        customEndpointEnabled: true,
      }),
    });

    await requestJson(baseUrl, "/api/admin/enterprises", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: enterpriseId,
        name: `G4 AI Enterprise ${suffix}`,
        seatCount: 5,
        subscriptionPlan: "enterprise",
        subscriptionStatus: SUBSCRIPTION_STATUS_ACTIVE,
        renewAt: toIsoAfterDays(30),
      }),
    });

    await requestJson(baseUrl, "/api/admin/sub-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: subAccountId,
        enterpriseId,
        displayName: `G4 AI Sub ${suffix}`,
        email: subAccountEmail,
        secret: "g4-ai-sub-pass",
        enabled: true,
        assetIds: enterpriseAssets,
      }),
    });

    await requestJson(baseUrl, "/api/admin/ai/personal-subscriptions", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        accountId: personalId,
        planCode: "personal",
        status: SUBSCRIPTION_STATUS_ACTIVE,
        renewAt: toIsoAfterDays(30),
      }),
    });

    await requestJson(baseUrl, "/api/admin/ai/enterprise-subscriptions", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        enterpriseId,
        planCode: "enterprise",
        status: SUBSCRIPTION_STATUS_ACTIVE,
        seatsPurchased: 5,
        renewAt: toIsoAfterDays(30),
      }),
    });

    await requestJson(baseUrl, "/api/admin/ai/endpoint-sync", {
      method: "PUT",
      headers: adminHeaders,
      body: JSON.stringify({
        endpointName: "Mock OpenAI Gateway",
        provider: "openai",
        baseUrl: `http://127.0.0.1:${MOCK_PORT}`,
        apiKey: "mock-key",
        modelName: "mock-model",
        syncToClients: true,
      }),
    });

    const personalLogin = await requestJson(baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "personal",
        identifier: personalEmail,
        secret: "g4-ai-personal-pass",
      }),
    });

    const enterpriseLogin = await requestJson(baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "enterpriseSubAccount",
        identifier: subAccountEmail,
        secret: "g4-ai-sub-pass",
      }),
    });

    const disableCustomEndpoint = async (login, scope) => {
      await requestJson(baseUrl, "/api/client/sync/settings", {
        method: "POST",
        body: JSON.stringify(
          buildSettingsPayload({
            account: {
              mode: login.mode,
              accountKey: login.accountKey,
              displayName: login.displayName,
              email: login.email,
              enterpriseId: login.enterpriseId,
              enterpriseName: login.enterpriseName,
              subAccountId: login.subAccountId,
              accessToken: login.accessToken,
            },
            sync: {
              endpointUrl: `${baseUrl}/api/client/sync`,
              organizationScope: scope,
              syncAssets: true,
              syncSettings: true,
            },
            customEndpoint: {
              useCustomEndpoint: false,
              endpointName: "",
              provider: "openai",
              baseUrl: "",
              apiKey: "",
              modelName: "",
            },
            settingsJson: {},
          }),
        ),
      });
    };

    await disableCustomEndpoint(personalLogin, "");
    await disableCustomEndpoint(enterpriseLogin, enterpriseId);

    const makePayload = (content) => ({
      model: "mock-model",
      temperature: 0,
      messages: [
        { role: "system", content: "Return a concise reply." },
        { role: "user", content },
      ],
    });

    const personalAi = await requestJson(
      baseUrl,
      `/api/client/ai/proxy/openai?${new URLSearchParams({ accessToken: personalLogin.accessToken })}`,
      {
        method: "POST",
        body: JSON.stringify(makePayload("personal regression check")),
      },
    );

    const enterpriseAi = await requestJson(
      baseUrl,
      `/api/client/ai/proxy/openai?${new URLSearchParams({ accessToken: enterpriseLogin.accessToken })}`,
      {
        method: "POST",
        body: JSON.stringify(makePayload("enterprise regression check")),
      },
    );

    assert(personalAi.choices?.[0]?.message?.content, "Personal AI call returned no assistant content.");
    assert(enterpriseAi.choices?.[0]?.message?.content, "Enterprise AI call returned no assistant content.");
    assert((personalAi.usage?.total_tokens ?? 0) > 0, "Personal AI call returned no token usage.");
    assert((enterpriseAi.usage?.total_tokens ?? 0) > 0, "Enterprise AI call returned no token usage.");

    console.log(
      JSON.stringify(
        {
          ok: true,
          personal: {
            accountId: personalId,
            content: personalAi.choices[0].message.content,
            totalTokens: personalAi.usage.total_tokens,
          },
          enterprise: {
            enterpriseId,
            subAccountId,
            content: enterpriseAi.choices[0].message.content,
            totalTokens: enterpriseAi.usage.total_tokens,
          },
        },
        null,
        2,
      ),
    );
  } finally {
    mockServer.kill();
  }
}

main().catch((error) => {
  console.error(
    JSON.stringify(
      {
        ok: false,
        error: error.message,
      },
      null,
      2,
    ),
  );
  process.exitCode = 1;
});
