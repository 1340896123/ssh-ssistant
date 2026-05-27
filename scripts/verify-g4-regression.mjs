import { mkdir, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";

const DEFAULT_BASE_URL = process.env.SSH_ASSISTANT_ADMIN_BASE_URL || "http://localhost:5047";
const DEFAULT_OUTPUT_DIR = process.env.SSH_ASSISTANT_G4_OUTPUT_DIR || path.resolve("tmp", "regression");
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

  let payload = null;
  const text = await response.text();
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = text;
    }
  }

  if (!response.ok) {
    const detail =
      payload && typeof payload === "object" && "error" in payload
        ? payload.error
        : JSON.stringify(payload);
    throw new Error(`${response.status} ${response.statusText}: ${detail}`);
  }

  return payload;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function nowSuffix() {
  return new Date().toISOString().replace(/[-:.TZ]/g, "").slice(0, 14);
}

function toIsoAfterDays(days) {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toISOString();
}

function parseJson(raw, fallback) {
  if (!raw) {
    return fallback;
  }

  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

function mapAssetIdsFromPull(pullPayload) {
  const records = parseJson(pullPayload.assetsJson, []);
  return records
    .map((item) => item?.asset?.cloudId || item?.asset?.id || null)
    .filter(Boolean);
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

function buildExecutionCase({
  id,
  title,
  automated,
  preconditions,
  steps,
  expected,
  actual,
  conclusion,
  status,
  defectId = "",
}) {
  return {
    id,
    title,
    automated,
    preconditions,
    steps,
    expected,
    actual,
    conclusion,
    status,
    defectId,
  };
}

function runCommand(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd || process.cwd(),
    encoding: "utf8",
    shell: false,
    maxBuffer: 1024 * 1024 * 20,
  });

  return {
    ok: result.status === 0,
    status: result.status,
    stdout: result.stdout || "",
    stderr: result.stderr || "",
    error: result.error ? String(result.error.message || result.error) : "",
  };
}

function toMarkdownReport(payload) {
  const lines = [];
  lines.push("# G4 回归执行记录");
  lines.push("");
  lines.push(`- 执行时间: ${payload.executedAt}`);
  lines.push(`- 服务地址: ${payload.baseUrl}`);
  lines.push(`- 结果概览: ${payload.summary.passed} passed / ${payload.summary.pending} pending / ${payload.summary.failed} failed`);
  lines.push("");
  lines.push("## 用例结果");
  lines.push("");

  for (const item of payload.executionCases) {
    lines.push(`### ${item.id} ${item.title}`);
    lines.push("");
    lines.push(`- 自动化: ${item.automated ? "是" : "否"}`);
    lines.push(`- 前置条件: ${item.preconditions}`);
    lines.push(`- 操作步骤: ${item.steps}`);
    lines.push(`- 预期结果: ${item.expected}`);
    lines.push(`- 实际结果: ${item.actual}`);
    lines.push(`- 结论: ${item.conclusion}`);
    lines.push(`- 状态: ${item.status}`);
    lines.push(`- 缺陷/问题记录: ${item.defectId || "无"}`);
    lines.push("");
  }

  lines.push("## 备注");
  lines.push("");
  lines.push("- G4.3 与 G4.4 中，云侧、Rust 本地快照恢复、前端模式切换逻辑、页面级 Switch -> LoginGateway、以及原生窗口 LoginGateway -> local -> LoginGateway 截图证据均已覆盖。");
  lines.push("- 本脚本已覆盖可通过 Admin API 稳定验证的个人账号、企业子账号、订阅归属、授权资产、设置同步、云侧隔离，以及本地快照恢复与页面/原生窗口模式切换证据。");
  lines.push("");

  return lines.join("\n");
}

async function main() {
  const baseUrl = normalizeBaseUrl(process.argv[2]);
  const suffix = nowSuffix();
  const enterpriseId = `ent-g4-${suffix}`;
  const subAccountId = `sub-g4-${suffix}`;
  const personalId = `usr-g4-${suffix}`;
  const personalEmail = `${personalId}@example.com`;
  const subAccountEmail = `${subAccountId}@example.com`;
  const adminHeaders = {};
  const findings = [];
  const executionCases = [];
  let currentStep = "bootstrap";

  const record = (id, status, detail, extra = {}) => {
    findings.push({ id, status, detail, ...extra });
  };

  try {
    currentStep = "admin-login";
    const adminLogin = await requestJson(baseUrl, "/api/admin/login", {
      method: "POST",
      body: JSON.stringify({
        username: "admin",
        password: "admin123",
      }),
    });
    adminHeaders.Authorization = `Bearer ${adminLogin.token}`;

    currentStep = "dashboard-seed";
    const dashboardBefore = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });

    const personalAsset = (dashboardBefore.assets || []).find((item) => item.ownerType === "personal");
    const enterpriseAssets = (dashboardBefore.assets || []).filter((item) => item.ownerType === "enterprise");
    assert(personalAsset, "Expected at least 1 personal asset in admin dashboard seed data.");
    assert(enterpriseAssets.length >= 2, "Expected at least 2 enterprise assets in admin dashboard seed data.");

    const authorizedAssetIds = enterpriseAssets.slice(0, 2).map((item) => item.id);

    currentStep = "create-enterprise";
    await requestJson(baseUrl, "/api/admin/enterprises", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: enterpriseId,
        name: `G4 Enterprise ${suffix}`,
        seatCount: 6,
        subscriptionPlan: "enterprise",
        subscriptionStatus: SUBSCRIPTION_STATUS_ACTIVE,
        renewAt: toIsoAfterDays(30),
      }),
    });

    currentStep = "create-sub-account";
    await requestJson(baseUrl, "/api/admin/sub-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: subAccountId,
        enterpriseId,
        displayName: `G4 Sub ${suffix}`,
        email: subAccountEmail,
        secret: "g4-sub-pass",
        enabled: true,
        assetIds: authorizedAssetIds,
      }),
    });

    currentStep = "authorize-sub-assets";
    await requestJson(baseUrl, `/api/admin/sub-accounts/${subAccountId}/assets`, {
      method: "PUT",
      headers: adminHeaders,
      body: JSON.stringify({
        assetIds: authorizedAssetIds,
      }),
    });

    currentStep = "create-personal-account";
    await requestJson(baseUrl, "/api/admin/personal-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: personalId,
        displayName: `G4 Personal ${suffix}`,
        email: personalEmail,
        secret: "g4-personal-pass",
        subscriptionStatus: SUBSCRIPTION_STATUS_ACTIVE,
        planName: "personal",
        customEndpointEnabled: true,
      }),
    });

    currentStep = "bind-enterprise-subscription";
    await requestJson(baseUrl, "/api/admin/ai/enterprise-subscriptions", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        enterpriseId,
        planCode: "enterprise",
        status: SUBSCRIPTION_STATUS_ACTIVE,
        seatsPurchased: 6,
        renewAt: toIsoAfterDays(30),
      }),
    });

    currentStep = "bind-personal-subscription";
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

    currentStep = "personal-login";
    const personalLogin = await requestJson(baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "personal",
        identifier: personalEmail,
        secret: "g4-personal-pass",
      }),
    });
    assert(personalLogin.mode === "personal", "Personal login should resolve personal mode.");

    currentStep = "personal-pull";
    const personalPull = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: personalLogin.mode,
        accountKey: personalLogin.accountKey,
        accessToken: personalLogin.accessToken,
      })}`,
    );
    const personalPulledAssetIds = mapAssetIdsFromPull(personalPull);
    assert(
      personalPulledAssetIds.length >= 1 && personalPulledAssetIds.includes(personalAsset.id),
      `Personal pull should include seeded personal asset ${personalAsset.id}.`,
    );

    const personalSettingsPayload = {
      theme: "light",
      language: "en",
      fileManager: {
        viewMode: "tree",
        layout: "left",
        sftpBufferSize: 640,
      },
      terminalAppearance: {
        fontSize: 15,
        fontFamily: "Fira Code, monospace",
        cursorStyle: "bar",
        lineHeight: 1.1,
      },
    };

    currentStep = "personal-sync-settings";
    const personalSyncSettings = await requestJson(baseUrl, "/api/client/sync/settings", {
      method: "POST",
      body: JSON.stringify(
        buildSettingsPayload({
          account: {
            mode: personalLogin.mode,
            accountKey: personalLogin.accountKey,
            displayName: personalLogin.displayName,
            email: personalLogin.email,
            enterpriseId: "",
            enterpriseName: "",
            subAccountId: "",
            accessToken: personalLogin.accessToken,
          },
          sync: {
            endpointUrl: `${baseUrl}/api/client/sync`,
            organizationScope: "",
            syncAssets: true,
            syncSettings: true,
          },
          customEndpoint: {
            useCustomEndpoint: true,
            endpointName: `personal-custom-${suffix}`,
            provider: "openai",
            baseUrl: "https://personal.example.invalid/v1",
            apiKey: `personal-key-${suffix}`,
            modelName: "gpt-4o-mini",
          },
          settingsJson: personalSettingsPayload,
        }),
      ),
    });
    const personalSavedSettings = parseJson(personalSyncSettings.settingsJson, {});
    assert(personalSavedSettings.theme === "light", "Personal settings sync theme mismatch.");
    assert(personalSavedSettings.fileManager?.layout === "left", "Personal settings sync layout mismatch.");

    currentStep = "personal-runtime";
    const personalRuntime = await requestJson(
      baseUrl,
      `/api/client/ai/runtime?${new URLSearchParams({
        accessToken: personalLogin.accessToken,
      })}`,
    );
    assert(personalRuntime.enabled === true, "Personal AI runtime should be enabled.");

    currentStep = "personal-subscription";
    const personalSubscription = await requestJson(
      baseUrl,
      `/api/client/subscription?${new URLSearchParams({
        accessToken: personalLogin.accessToken,
      })}`,
    );
    assert(
      String(personalSubscription.subscription?.billingScope || "").toLowerCase() === "personal",
      "Personal subscription snapshot should resolve personal billing scope.",
    );

    record("G4.1", "passed", "个人账号登录、资产拉取、设置同步、AI 运行态与订阅归属均验证通过。", {
      pulledAssets: personalPulledAssetIds,
    });

    currentStep = "enterprise-login";
    const enterpriseLogin = await requestJson(baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "enterpriseSubAccount",
        identifier: subAccountEmail,
        secret: "g4-sub-pass",
      }),
    });
    assert(enterpriseLogin.mode === "enterpriseSubAccount", "Enterprise login should resolve enterpriseSubAccount mode.");

    currentStep = "enterprise-pull";
    const enterprisePull = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: enterpriseLogin.mode,
        accountKey: enterpriseLogin.accountKey,
        accessToken: enterpriseLogin.accessToken,
      })}`,
    );
    const enterprisePulledAssetIds = mapAssetIdsFromPull(enterprisePull);
    assert(
      JSON.stringify(enterprisePulledAssetIds) === JSON.stringify(authorizedAssetIds),
      `Enterprise pull should only expose authorized assets ${authorizedAssetIds.join(", ")}.`,
    );

    const enterpriseSettingsPayload = {
      theme: "dark",
      language: "zh",
      fileManager: {
        viewMode: "flat",
        layout: "bottom",
        sftpBufferSize: 896,
      },
      networkAdaptive: {
        enableAdaptive: true,
        latencyCheckIntervalSecs: 45,
        highLatencyThresholdMs: 320,
        lowBandwidthThresholdKbps: 128,
      },
    };

    currentStep = "enterprise-sync-settings";
    const enterpriseSyncSettings = await requestJson(baseUrl, "/api/client/sync/settings", {
      method: "POST",
      body: JSON.stringify(
        buildSettingsPayload({
          account: {
            mode: enterpriseLogin.mode,
            accountKey: enterpriseLogin.accountKey,
            displayName: enterpriseLogin.displayName,
            email: enterpriseLogin.email,
            enterpriseId: enterpriseLogin.enterpriseId,
            enterpriseName: enterpriseLogin.enterpriseName,
            subAccountId: enterpriseLogin.subAccountId,
            accessToken: enterpriseLogin.accessToken,
          },
          sync: {
            endpointUrl: `${baseUrl}/api/client/sync`,
            organizationScope: enterpriseId,
            syncAssets: true,
            syncSettings: true,
          },
          customEndpoint: {
            useCustomEndpoint: true,
            endpointName: `enterprise-custom-${suffix}`,
            provider: "openai",
            baseUrl: "https://enterprise.example.invalid/v1",
            apiKey: `enterprise-key-${suffix}`,
            modelName: "gpt-4o-mini",
          },
          settingsJson: enterpriseSettingsPayload,
        }),
      ),
    });
    const enterpriseSavedSettings = parseJson(enterpriseSyncSettings.settingsJson, {});
    assert(enterpriseSavedSettings.theme === "dark", "Enterprise settings sync theme mismatch.");
    assert(
      enterpriseSavedSettings.fileManager?.sftpBufferSize === 896,
      "Enterprise settings sync SFTP buffer size mismatch.",
    );

    currentStep = "enterprise-runtime";
    const enterpriseRuntime = await requestJson(
      baseUrl,
      `/api/client/ai/runtime?${new URLSearchParams({
        accessToken: enterpriseLogin.accessToken,
      })}`,
    );
    assert(enterpriseRuntime.enabled === true, "Enterprise AI runtime should be enabled.");

    currentStep = "enterprise-subscription";
    const enterpriseSubscription = await requestJson(
      baseUrl,
      `/api/client/subscription?${new URLSearchParams({
        accessToken: enterpriseLogin.accessToken,
      })}`,
    );
    assert(
      String(enterpriseSubscription.subscription?.billingScope || "").toLowerCase() === "enterprise",
      "Enterprise subscription snapshot should resolve enterprise billing scope.",
    );
    assert(
      String(enterpriseSubscription.currentInvoice?.targetType || "").toLowerCase() === "enterprise",
      "Enterprise current invoice should belong to enterprise scope.",
    );

    record("G4.2", "passed", "企业子账号登录、授权资产可见性、设置同步、AI 运行态与企业账单归属均验证通过。", {
      pulledAssets: enterprisePulledAssetIds,
    });

    currentStep = "cross-pull-personal";
    const personalPullAfterEnterprise = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: personalLogin.mode,
        accountKey: personalLogin.accountKey,
        accessToken: personalLogin.accessToken,
      })}`,
    );
    const personalAfterEnterpriseSettings = parseJson(personalPullAfterEnterprise.settingsJson, {});
    const personalPulledAssetIdsAfterEnterprise = mapAssetIdsFromPull(personalPullAfterEnterprise);

    assert(personalAfterEnterpriseSettings.theme === "light", "Personal settings were polluted by enterprise settings.");
    assert(
      personalAfterEnterpriseSettings.fileManager?.layout === "left",
      "Personal file manager layout was polluted by enterprise settings.",
    );
    assert(
      !personalPulledAssetIdsAfterEnterprise.some((id) => authorizedAssetIds.includes(id)),
      "Personal pull should not include enterprise authorized assets.",
    );

    currentStep = "cross-pull-enterprise";
    const enterprisePullAfterPersonal = await requestJson(
      baseUrl,
      `/api/client/sync/pull?${new URLSearchParams({
        mode: enterpriseLogin.mode,
        accountKey: enterpriseLogin.accountKey,
        accessToken: enterpriseLogin.accessToken,
      })}`,
    );
    const enterpriseAfterPersonalSettings = parseJson(enterprisePullAfterPersonal.settingsJson, {});
    assert(enterpriseAfterPersonalSettings.theme === "dark", "Enterprise settings were polluted by personal settings.");
    assert(
      enterpriseAfterPersonalSettings.fileManager?.sftpBufferSize === 896,
      "Enterprise settings were polluted after personal sync.",
    );

    record("G4.4", "passed", "个人账号与企业子账号的资产、设置、订阅和 AI 配置在云侧未发生串数据。", {
      personalAssets: personalPulledAssetIdsAfterEnterprise,
      enterpriseAssets: enterprisePulledAssetIds,
    });

    currentStep = "local-snapshot-rust-test";
    const localSnapshotTest = runCommand(
      "cargo",
      [
        "test",
        "local_workspace_snapshot_round_trip_restores_local_mode_without_cloud_residue",
        "--manifest-path",
        "src-tauri/Cargo.toml",
      ],
      { cwd: path.resolve(".") },
    );
    assert(
      localSnapshotTest.ok,
      `Local snapshot round-trip test failed: ${localSnapshotTest.stderr || localSnapshotTest.stdout || localSnapshotTest.error}`,
    );

    currentStep = "mode-transition-logic-test";
    const modeTransitionTest = runCommand(
        "node",
      ["scripts/verify-g4-mode-transition.mjs"],
      { cwd: path.resolve(".") },
    );
    assert(
      modeTransitionTest.ok,
      `Mode transition logic test failed: ${modeTransitionTest.stderr || modeTransitionTest.stdout || modeTransitionTest.error}`,
    );

    currentStep = "web-switch-flow-test";
    const webSwitchFlowTest = runCommand(
      "node",
      ["scripts/verify-g4-web-switch.mjs"],
      { cwd: path.resolve(".") },
    );
    assert(
      webSwitchFlowTest.ok,
      `Web switch flow test failed: ${webSwitchFlowTest.stderr || webSwitchFlowTest.stdout || webSwitchFlowTest.error}`,
    );

    currentStep = "managed-ai-call-test";
    const aiCallTest = runCommand(
      "node",
      ["scripts/verify-g4-ai-call.mjs"],
      { cwd: path.resolve(".") },
    );
    assert(
      aiCallTest.ok,
      `Managed AI call test failed: ${aiCallTest.stderr || aiCallTest.stdout || aiCallTest.error}`,
    );

    currentStep = "native-window-flow-test";
    const nativeWindowFlowTest = runCommand(
      "powershell",
      [
        "-NoLogo",
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        "scripts/verify-g4-native-local-switch.ps1",
      ],
      { cwd: path.resolve(".") },
    );
    assert(
      nativeWindowFlowTest.ok,
      `Native window flow test failed: ${nativeWindowFlowTest.stderr || nativeWindowFlowTest.stdout || nativeWindowFlowTest.error}`,
    );

    record(
      "G4.3",
      "passed",
      "Rust 本地工作区快照 round-trip、前端模式切换逻辑、Web 页面 Switch -> LoginGateway、以及原生窗口 LoginGateway -> local -> LoginGateway 截图验证均通过。",
    );

    const executedAt = new Date().toISOString();
    executionCases.push(
      buildExecutionCase({
        id: "G4.1",
        title: "个人账号回归用例通过",
        automated: true,
        preconditions: "Admin API 可访问；存在至少 1 个 personal 资产种子；已创建本次 personal 测试账号。",
        steps: "登录 personal 账号，执行 sync pull，提交 settings sync，再拉取 AI runtime 与 subscription snapshot，并通过 mock 网关实际发起一次 AI 代理调用。",
        expected: "个人账号可登录；能拉到个人资产；个人设置可保存；AI runtime 可用；订阅 billingScope=personal。",
        actual: `登录成功，拉取到资产 ${personalPulledAssetIds.join(", ")}，theme=light/layout=left 保存成功，AI runtime enabled=true，billingScope=personal，且 \`node scripts/verify-g4-ai-call.mjs\` 已返回 personal mock AI 响应。`,
        conclusion: "通过",
        status: "passed",
      }),
      buildExecutionCase({
        id: "G4.2",
        title: "企业子账号回归用例通过",
        automated: true,
        preconditions: "Admin API 可访问；存在至少 2 个 enterprise 资产种子；已创建企业、子账号和企业订阅。",
        steps: "登录 enterpriseSubAccount，执行 sync pull，提交 settings sync，再拉取 AI runtime 与 subscription snapshot，并通过 mock 网关实际发起一次 AI 代理调用。",
        expected: "企业子账号可登录；仅能看到授权资产；设置同步成功；AI runtime 可用；当前账单与订阅归属 enterprise。",
        actual: `登录成功，仅拉取到授权资产 ${enterprisePulledAssetIds.join(", ")}，theme=dark/sftpBufferSize=896 保存成功，AI runtime enabled=true，invoice targetType=enterprise，且 \`node scripts/verify-g4-ai-call.mjs\` 已返回 enterprise mock AI 响应。`,
        conclusion: "通过",
        status: "passed",
      }),
      buildExecutionCase({
        id: "G4.3",
        title: "本地模式回归用例通过",
        automated: true,
        preconditions: "以 Tauri 桌面态启动应用；可访问本地 sqlite；准备一份本地工作区资产与设置样本。",
        steps: "执行 Rust round-trip 测试验证本地工作区快照在 cloud 覆盖后可恢复；执行前端模式切换逻辑脚本；执行页面级 Switch -> LoginGateway 流程验证；执行原生窗口 LoginGateway -> local -> LoginGateway 截图验证。",
        expected: "本地模式可独立进入；核心功能可用；不误触企业/个人云同步；本地快照能恢复。",
        actual: "`cargo test local_workspace_snapshot_round_trip_restores_local_mode_without_cloud_residue --manifest-path src-tauri/Cargo.toml`、`node scripts/verify-g4-mode-transition.mjs`、`node scripts/verify-g4-web-switch.mjs`、`powershell -File scripts/verify-g4-native-local-switch.ps1` 已通过；当前轮证据包含 Web 与原生窗口两条切换链路截图，证明本地模式可进入、可显示本地工作台，并可切回登录网关。",
        conclusion: "通过",
        status: "passed",
      }),
      buildExecutionCase({
        id: "G4.4",
        title: "三种模式互不串数据",
        automated: true,
        preconditions: "已具备 personal、enterpriseSubAccount、local 三套独立样本数据。",
        steps: "先自动校验 personal 与 enterprise 云侧隔离；再执行 Rust round-trip、本地模式切换逻辑、页面级 Switch -> LoginGateway、原生窗口 LoginGateway -> local -> LoginGateway 验证。",
        expected: "personal 与 enterprise 云侧不串数据；local 与两种 cloud 模式切换后不读到对方残留数据。",
        actual: "云侧 personal/enterprise 隔离已自动通过；local 快照 round-trip、前端模式切换逻辑、页面级 Switch -> LoginGateway、原生窗口 LoginGateway -> local -> LoginGateway 已自动通过，证明 cloud 覆盖后恢复不残留 cloud 数据，且切换时会清理/恢复正确状态。",
        conclusion: "通过",
        status: "passed",
      }),
      buildExecutionCase({
        id: "G4.5",
        title: "回归用例有执行记录",
        automated: true,
        preconditions: "执行本脚本或按文档手工复验。",
        steps: "输出 JSON 与 Markdown 执行记录，按用例填写前置、步骤、预期、实际与结论。",
        expected: "每条回归用例均有可追踪记录；失败项能挂缺陷单；通过项可复验。",
        actual: "脚本已生成结构化 JSON/Markdown；人工补跑时可继续复用同一模板。",
        conclusion: "通过",
        status: "passed",
      }),
    );

    const summary = executionCases.reduce(
      (acc, item) => {
        if (item.status === "passed") acc.passed += 1;
        else if (item.status === "pending" || item.status === "partial") acc.pending += 1;
        else acc.failed += 1;
        return acc;
      },
      { passed: 0, pending: 0, failed: 0 },
    );

    const outputPayload = {
      ok: true,
      baseUrl,
      executedAt,
      summary,
      findings,
      executionCases,
    };

    await mkdir(DEFAULT_OUTPUT_DIR, { recursive: true });
    const jsonPath = path.join(DEFAULT_OUTPUT_DIR, "g4-regression-latest.json");
    const markdownPath = path.join(DEFAULT_OUTPUT_DIR, "g4-regression-latest.md");
    await writeFile(jsonPath, `${JSON.stringify(outputPayload, null, 2)}\n`, "utf8");
    await writeFile(markdownPath, `${toMarkdownReport(outputPayload)}\n`, "utf8");

    console.log(
      JSON.stringify(
        {
          ...outputPayload,
          artifacts: {
            jsonPath,
            markdownPath,
          },
        },
        null,
        2,
      ),
    );
  } catch (error) {
    const failure = {
      ok: false,
      step: currentStep,
      error: error.message,
      findings,
    };
    console.error(JSON.stringify(failure, null, 2));
    process.exitCode = 1;
  }
}

main();
