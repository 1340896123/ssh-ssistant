import { createHmac } from "node:crypto";

const DEFAULT_BASE_URL = process.env.SSH_ASSISTANT_ADMIN_BASE_URL || "http://localhost:5047";

function normalizeBaseUrl(url) {
  return (url || DEFAULT_BASE_URL).replace(/\/+$/, "");
}

async function requestJson(baseUrl, path, options = {}) {
  const response = await fetch(`${baseUrl}${path}`, {
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

function toIsoAfterDays(days) {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toISOString();
}

function nowSuffix() {
  return new Date().toISOString().replace(/[-:.TZ]/g, "").slice(0, 14);
}

function asStatus(value) {
  return String(value || "").trim().toLowerCase();
}

function asInvoiceStatus(value) {
  if (typeof value === "number") {
    switch (value) {
      case 1:
        return "paid";
      case 2:
        return "overdue";
      case 3:
        return "voided";
      case 0:
      default:
        return "open";
    }
  }

  return asStatus(value);
}

function buildStripeLikeSignature(secret, payloadJson, timestamp) {
  const signedPayload = `${timestamp}.${payloadJson}`;
  const signature = createHmac("sha256", secret)
    .update(signedPayload)
    .digest("hex");
  return `t=${timestamp},v1=${signature}`;
}

function subscriptionStatusValue(name) {
  switch (asStatus(name)) {
    case "trialing":
      return 1;
    case "active":
      return 2;
    case "pastdue":
      return 3;
    case "cancelled":
      return 4;
    case "inactive":
    default:
      return 0;
  }
}

function resolveRealAiConfig() {
  const explicitBaseUrl = process.env.SSH_ASSISTANT_REAL_AI_BASE_URL?.trim();
  const explicitApiKey = process.env.SSH_ASSISTANT_REAL_AI_API_KEY?.trim();
  const explicitModel = process.env.SSH_ASSISTANT_REAL_AI_MODEL?.trim();
  const explicitProvider = process.env.SSH_ASSISTANT_REAL_AI_PROVIDER?.trim() || "openai";
  if (explicitBaseUrl && explicitApiKey && explicitModel) {
    return {
      provider: explicitProvider,
      baseUrl: explicitBaseUrl,
      apiKey: explicitApiKey,
      modelName: explicitModel,
      label: "Explicit real AI config",
    };
  }

  const zhipuKey = process.env.ZHIPU_API_KEY?.trim();
  if (zhipuKey) {
    return {
      provider: "openai",
      baseUrl: "https://open.bigmodel.cn/api/paas/v4",
      apiKey: zhipuKey,
      modelName: "glm-5.1",
      label: "Zhipu OpenAI-compatible",
    };
  }

  const minimaxKey = process.env.MINIMAX_API_KEY?.trim();
  if (minimaxKey) {
    return {
      provider: "openai",
      baseUrl: "https://api.minimaxi.com/v1",
      apiKey: minimaxKey,
      modelName: "MiniMax-M2.7",
      label: "MiniMax OpenAI-compatible",
    };
  }

  return null;
}

async function main() {
  const baseUrl = normalizeBaseUrl(process.argv[2]);
  const suffix = nowSuffix();
  const enterpriseId = `ent-g3-${suffix}`;
  const subAccountId = `sub-g3-${suffix}`;
  const personalId = `usr-g3-${suffix}`;
  const adminHeaders = {};
  const results = [];
  let currentStep = "bootstrap";

  const record = (id, status, detail) => {
    results.push({ id, status, detail });
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

    currentStep = "dashboard-before";
    const dashboardBefore = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });

    const enterprisePayload = {
      id: enterpriseId,
      name: `G3 Enterprise ${suffix}`,
      seatCount: 5,
      subscriptionPlan: "enterprise",
      subscriptionStatus: subscriptionStatusValue("active"),
      renewAt: toIsoAfterDays(30),
    };

    currentStep = "create-enterprise";
    await requestJson(baseUrl, "/api/admin/enterprises", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify(enterprisePayload),
    });

    currentStep = "create-sub-account";
    await requestJson(baseUrl, "/api/admin/sub-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: subAccountId,
        enterpriseId,
        displayName: `G3 Sub ${suffix}`,
        email: `${subAccountId}@example.com`,
        secret: "g3-pass-123",
        enabled: true,
        assetIds: [],
      }),
    });

    currentStep = "create-personal-account";
    await requestJson(baseUrl, "/api/admin/personal-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: personalId,
        displayName: `G3 Personal ${suffix}`,
        email: `${personalId}@example.com`,
        secret: "g3-personal-pass",
        subscriptionStatus: subscriptionStatusValue("inactive"),
        planName: "free",
        customEndpointEnabled: false,
      }),
    });

    currentStep = "bind-enterprise-subscription";
    const enterpriseSubscription = await requestJson(baseUrl, "/api/admin/ai/enterprise-subscriptions", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        enterpriseId,
        planCode: "enterprise",
        status: subscriptionStatusValue("active"),
        seatsPurchased: 5,
        renewAt: toIsoAfterDays(30),
      }),
    });

    assert(enterpriseSubscription.enterpriseId === enterpriseId, "Enterprise subscription enterpriseId mismatch.");
    assert(enterpriseSubscription.planCode === "enterprise", "Enterprise subscription planCode mismatch.");
    assert(enterpriseSubscription.seatsPurchased === 5, "Enterprise subscription seat count mismatch.");
    assert(enterpriseSubscription.seatsAssigned === 1, "Enterprise subscription assigned seats should reflect enabled sub accounts.");
    record("G3.1", "passed", `Enterprise ${enterpriseId} bound to plan ${enterpriseSubscription.planCode} with ${enterpriseSubscription.seatsPurchased} seats and ${enterpriseSubscription.seatsAssigned} assigned.`);

    currentStep = "bind-personal-subscription";
    const personalSubscription = await requestJson(baseUrl, "/api/admin/ai/personal-subscriptions", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        accountId: personalId,
        planCode: "personal",
        status: subscriptionStatusValue("active"),
        renewAt: toIsoAfterDays(30),
      }),
    });
    assert(personalSubscription.accountId === personalId, "Personal subscription accountId mismatch.");

    currentStep = "generate-billing-1";
    const billingCycleFirst = await requestJson(baseUrl, "/api/admin/billing/generate-current-cycle", {
      method: "POST",
      headers: adminHeaders,
    });
    currentStep = "generate-billing-2";
    const billingCycleSecond = await requestJson(baseUrl, "/api/admin/billing/generate-current-cycle", {
      method: "POST",
      headers: adminHeaders,
    });

    currentStep = "dashboard-after-billing";
    const dashboardAfterBilling = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });
    const currentMonth = dashboardAfterBilling.billing.billingMonth;
    const enterpriseInvoice = dashboardAfterBilling.billing.recentInvoices.find(
      (item) => item.targetType === "enterprise" && item.targetId === enterpriseId && item.billingMonth === currentMonth,
    );
    const personalInvoice = dashboardAfterBilling.billing.recentInvoices.find(
      (item) => item.targetType === "personal" && item.targetId === personalId && item.billingMonth === currentMonth,
    );

    assert(enterpriseInvoice, "Enterprise invoice was not generated for current billing month.");
    assert(personalInvoice, "Personal invoice was not generated for current billing month.");
    assert(enterpriseInvoice.subscriptionAmount === 5 * enterpriseSubscription.pricePerSeat, "Enterprise invoice subscription amount mismatch.");
    assert(enterpriseInvoice.currency === enterpriseSubscription.currency, "Enterprise invoice currency mismatch.");
    assert(billingCycleFirst.generatedInvoices >= 0, "Billing generation should return a non-negative generated invoice count.");
    assert(billingCycleSecond.generatedInvoices >= 0, "Repeat billing generation should return a non-negative generated invoice count.");
    const duplicateInvoices = dashboardAfterBilling.billing.recentInvoices.filter(
      (item) => item.targetType === "enterprise" && item.targetId === enterpriseId && item.billingMonth === currentMonth,
    );
    assert(duplicateInvoices.length === 1, "Repeat billing generation created duplicate dirty invoices.");
    record(
      "G3.2",
      "passed",
      `Current month ${currentMonth} invoices exist for enterprise and personal scopes; repeat generation returned counts ${billingCycleFirst.generatedInvoices}/${billingCycleSecond.generatedInvoices} and reused invoice ${enterpriseInvoice.id} without duplicates.`,
    );

    currentStep = "create-checkout";
    const checkout = await requestJson(baseUrl, "/api/admin/billing/checkout-sessions", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        invoiceId: enterpriseInvoice.id,
        providerKey: "manual",
        returnUrl: "sshstar://billing/success",
        cancelUrl: "sshstar://billing/cancel",
      }),
    });
    assert(asStatus(checkout.status) === "pending", "Checkout transaction should start in pending status.");
    assert(checkout.checkoutUrl, "Checkout transaction did not return a checkout URL.");
    assert(checkout.externalReference, "Checkout transaction did not return an external reference.");
    record("G3.3", "passed", `Checkout created for invoice ${enterpriseInvoice.id} with external reference ${checkout.externalReference}.`);

    currentStep = "manual-webhook";
    const webhook = await requestJson(baseUrl, "/api/client/billing/webhook", {
      method: "POST",
      body: JSON.stringify({
        providerKey: "manual",
        webhookSecret: "manual-secret",
        eventType: "payment.completed",
        externalReference: checkout.externalReference,
        status: "completed",
        amount: checkout.amount,
        currency: checkout.currency,
        invoiceId: enterpriseInvoice.id,
        note: "verified-by-g3-script",
        payloadJson: "{}",
      }),
    });
    assert(asStatus(webhook.status) === "completed", "Webhook should mark transaction as completed.");

    currentStep = "dashboard-after-payment";
    const dashboardAfterPayment = await requestJson(baseUrl, "/api/admin/dashboard", {
      headers: adminHeaders,
    });
    const paidEnterpriseInvoice = dashboardAfterPayment.billing.recentInvoices.find((item) => item.id === enterpriseInvoice.id);
    assert(paidEnterpriseInvoice, "Paid enterprise invoice missing after webhook.");
    assert(asInvoiceStatus(paidEnterpriseInvoice.status) === "paid", "Enterprise invoice should become paid after completed webhook.");
    assert(Math.abs(paidEnterpriseInvoice.paidAmount - paidEnterpriseInvoice.totalAmount) < 0.0001, "Paid amount should match invoice total.");
    assert(
      paidEnterpriseInvoice.payments.some(
        (item) => item.externalReference === checkout.externalReference && asStatus(item.status) === "completed",
      ),
      "Completed payment record missing from invoice payment list.",
    );
    record("G3.4", "passed", `Webhook completed payment for invoice ${enterpriseInvoice.id}; invoice status moved to paid with matching paid amount.`);

    currentStep = "personal-login";
    const personalLogin = await requestJson(baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "personal",
        identifier: `${personalId}@example.com`,
        secret: "g3-personal-pass",
      }),
    });
    currentStep = "enterprise-login";
    const enterpriseLogin = await requestJson(baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "enterpriseSubAccount",
        identifier: `${subAccountId}@example.com`,
        secret: "g3-pass-123",
      }),
    });

    currentStep = "personal-runtime";
    const personalRuntime = await requestJson(
      baseUrl,
      `/api/client/ai/runtime?${new URLSearchParams({ accessToken: personalLogin.accessToken })}`,
    );
    currentStep = "enterprise-runtime";
    const enterpriseRuntime = await requestJson(
      baseUrl,
      `/api/client/ai/runtime?${new URLSearchParams({ accessToken: enterpriseLogin.accessToken })}`,
    );
    currentStep = "personal-snapshot";
    const personalSnapshot = await requestJson(
      baseUrl,
      `/api/client/subscription?${new URLSearchParams({ accessToken: personalLogin.accessToken })}`,
    );
    currentStep = "enterprise-snapshot";
    const enterpriseSnapshot = await requestJson(
      baseUrl,
      `/api/client/subscription?${new URLSearchParams({ accessToken: enterpriseLogin.accessToken })}`,
    );

    assert(asStatus(personalSnapshot.subscription.planName) === "personal", "Personal snapshot should resolve personal plan.");
    assert(asStatus(enterpriseSnapshot.subscription.planName) === "enterprise", "Enterprise snapshot should resolve enterprise plan.");
    assert(personalRuntime.enabled === true, "Personal runtime should be enabled.");
    assert(enterpriseRuntime.enabled === true, "Enterprise runtime should be enabled.");
    assert(enterpriseRuntime.usingManagedEndpoint === true, "Enterprise runtime should default to managed endpoint when no custom endpoint exists.");
    record("G3.5", "passed", `Personal and enterprise clients both resolved scoped subscription snapshots and managed AI runtime correctly.`);

    const realAiConfig = resolveRealAiConfig();
    if (!realAiConfig) {
      record(
        "G3.6",
        "pending",
        "真实 AI 调用需要可访问的托管端点或有效第三方 API Key。当前未检测到可用真实凭据。",
      );
      record(
        "G3.7",
        "pending",
        "AI 用量记录依赖一次真实成功 AI 响应写入 usage。当前未检测到可用真实凭据。",
      );
    } else {
      currentStep = "configure-real-ai-endpoint";
      await requestJson(baseUrl, "/api/admin/ai/endpoint-sync", {
        method: "PUT",
        headers: adminHeaders,
        body: JSON.stringify({
          endpointName: `${realAiConfig.label} Managed Endpoint`,
          provider: realAiConfig.provider,
          baseUrl: realAiConfig.baseUrl,
          apiKey: realAiConfig.apiKey,
          modelName: realAiConfig.modelName,
          syncToClients: true,
        }),
      });

      currentStep = "personal-runtime-after-real-ai";
      const managedRuntime = await requestJson(
        baseUrl,
        `/api/client/ai/runtime?${new URLSearchParams({ accessToken: personalLogin.accessToken })}`,
      );
      assert(managedRuntime.enabled === true, "Managed AI runtime should stay enabled after endpoint sync.");
      assert(managedRuntime.usingManagedEndpoint === true, "Managed AI runtime should use the synced managed endpoint.");

      currentStep = "real-ai-openai-proxy";
      const aiResponse = await requestJson(
        baseUrl,
        `/api/client/ai/proxy/openai?${new URLSearchParams({ accessToken: personalLogin.accessToken })}`,
        {
          method: "POST",
          body: JSON.stringify({
            model: realAiConfig.modelName,
            temperature: 0,
            max_tokens: 64,
            messages: [
            {
              role: "system",
              content: "You are a concise assistant. Reply directly without showing internal reasoning.",
            },
              {
                role: "user",
                content: "Reply with a short confirmation that billing AI flow is working.",
              },
            ],
          }),
        },
      );

      const aiMessage = aiResponse?.choices?.[0]?.message ?? {};
      const aiContent = aiMessage?.content?.trim?.() ?? "";
      const aiReasoning = aiMessage?.reasoning_content?.trim?.() ?? "";
      const totalTokens = Number(aiResponse?.usage?.total_tokens ?? 0);
      assert(
        aiContent.length > 0 || aiReasoning.length > 0 || totalTokens > 0,
        "Real AI proxy returned an empty response.",
      );

      currentStep = "personal-snapshot-after-real-ai";
      const personalSnapshotAfterAi = await requestJson(
        baseUrl,
        `/api/client/subscription?${new URLSearchParams({ accessToken: personalLogin.accessToken })}`,
      );
      currentStep = "dashboard-after-real-ai";
      const dashboardAfterAi = await requestJson(baseUrl, "/api/admin/dashboard", {
        headers: adminHeaders,
      });

      assert((personalSnapshotAfterAi.usage?.totalRequests ?? 0) >= 1, "Personal subscription snapshot should show at least one AI request.");
      assert((personalSnapshotAfterAi.usage?.totalTokens ?? 0) > 0, "Personal subscription snapshot should show token usage after real AI call.");
      assert((dashboardAfterAi.aiUsage?.totalRequests ?? 0) >= 1, "Admin dashboard AI usage should show at least one request.");
      assert((dashboardAfterAi.aiUsage?.totalTokens ?? 0) > 0, "Admin dashboard AI usage should show token usage after real AI call.");
      const topAccount = (dashboardAfterAi.aiUsage?.topAccounts ?? []).find(
        (item) => item.accountId === personalId && asStatus(item.accountMode) === "personal",
      );
      assert(topAccount, "Admin dashboard top accounts should include the personal account that made the real AI call.");

      record(
        "G3.6",
        "passed",
        `Real AI request succeeded through ${realAiConfig.label}; model ${realAiConfig.modelName} returned usable output/usage metadata.`,
      );
      record(
        "G3.7",
        "passed",
        `Usage became observable in both personal snapshot and admin dashboard for account ${personalId}, with ${personalSnapshotAfterAi.usage.totalTokens} tokens recorded.`,
      );
    }

    console.log(JSON.stringify({
      ok: true,
      baseUrl,
      summary: {
        invoicesBefore: dashboardBefore.billing.recentInvoices.length,
        invoicesAfter: dashboardAfterPayment.billing.recentInvoices.length,
        currentMonth,
        enterpriseInvoiceId: enterpriseInvoice.id,
        checkoutReference: checkout.externalReference,
      },
      results,
    }, null, 2));
  } catch (error) {
    console.error(JSON.stringify({ ok: false, step: currentStep, error: error.message, results }, null, 2));
    process.exitCode = 1;
  }
}

main();
