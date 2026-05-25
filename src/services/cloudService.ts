import type { ClientSubscriptionSnapshot, Settings } from "../types";
import type { CloudAssetRecord } from "../types";

interface CloudSubscriptionPayload {
  planName: string;
  planDisplayName?: string;
  status: Settings["ai"]["subscription"]["status"];
  seats: number;
  pricePerSeat?: number;
  currency?: string;
  billingScope?: "global" | "enterprise" | "personal";
  allowCustomEndpoint: boolean;
  syncCustomEndpoint: boolean;
  renewAt?: string;
}

interface EndpointSyncPayload {
  endpointName: string;
  provider: string;
  baseUrl: string;
  modelName: string;
  syncToClients: boolean;
  updatedAt: string;
}

interface ClientLoginRequest {
  mode: string;
  identifier: string;
  secret: string;
}

interface ClientLoginResponsePayload {
  mode: string;
  accountKey: string;
  displayName: string;
  email: string;
  enterpriseId: string;
  enterpriseName: string;
  subAccountId: string;
  accessToken: string;
  refreshToken: string;
  expiresAt: string;
  refreshExpiresAt: string;
  syncEndpointUrl: string;
  aiSubscription: CloudSubscriptionPayload;
  endpointSync: EndpointSyncPayload;
  subscriptionSnapshot?: ClientSubscriptionSnapshotPayload;
}

interface ClientSyncResponsePayload {
  syncedAt: string;
  settingsJson: string;
  assetsJson: string;
  aiSubscription: CloudSubscriptionPayload;
  endpointSync: EndpointSyncPayload;
  subscriptionSnapshot?: ClientSubscriptionSnapshotPayload;
}

interface ClientSubscriptionSnapshotPayload {
  subscription: CloudSubscriptionPayload;
  currentInvoice?: ClientSubscriptionInvoicePayload | null;
  recentInvoices?: ClientSubscriptionInvoicePayload[];
  paymentProviders?: ClientSubscriptionPaymentProviderPayload[];
  usage: ClientSubscriptionUsagePayload;
}

interface ClientSubscriptionInvoiceLineItemPayload {
  id: string;
  invoiceId: string;
  itemType: string;
  description: string;
  quantity: number;
  unitPrice: number;
  amount: number;
  currency: string;
  totalTokens?: number | null;
  createdAt: string;
}

interface ClientSubscriptionPaymentPayload {
  id: string;
  invoiceId: string;
  targetType: string;
  targetId: string;
  providerKey: string;
  amount: number;
  currency: string;
  paymentMethod: string;
  status: string;
  externalReference: string;
  note: string;
  checkoutUrl: string;
  expiresAt?: string | null;
  paidAt?: string | null;
  createdAt: string;
  updatedAt: string;
}

interface ClientSubscriptionPaymentProviderPayload {
  providerKey: string;
  displayName: string;
  providerType: string;
  webhookSecret: string;
  enabled: boolean;
  metadataJson: string;
  checkoutBaseUrl: string;
  webhookMode: string;
  apiBaseUrl: string;
  secretApiKey: string;
  successUrl: string;
  cancelUrl: string;
  updatedAt: string;
}

interface ClientSubscriptionInvoicePayload {
  id: string;
  targetType: string;
  targetId: string;
  planCode: string;
  status: "open" | "paid" | "overdue" | "voided";
  seatCount: number;
  unitPrice: number;
  subscriptionAmount: number;
  aiUsageAmount: number;
  totalAmount: number;
  currency: string;
  billingMonth: string;
  dueAt: string;
  createdAt: string;
  updatedAt: string;
  paidAmount: number;
  remainingAmount: number;
  lineItems: ClientSubscriptionInvoiceLineItemPayload[];
  payments: ClientSubscriptionPaymentPayload[];
}

interface ClientSubscriptionUsageAccountPayload {
  accountId: string;
  accountMode: string;
  requests: number;
  totalTokens: number;
  estimatedCost: number;
  currency: string;
}

interface ClientSubscriptionUsagePayload {
  billingMonth: string;
  totalRequests: number;
  managedRequests: number;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
  estimatedCost: number;
  currency: string;
  topAccounts: ClientSubscriptionUsageAccountPayload[];
}

export interface ClientLoginResponse {
  mode: string;
  accountKey: string;
  displayName: string;
  email: string;
  enterpriseId: string;
  enterpriseName: string;
  subAccountId: string;
  accessToken: string;
  refreshToken: string;
  expiresAt: string;
  refreshExpiresAt: string;
  syncEndpointUrl: string;
  aiSubscription: Settings["ai"]["subscription"];
  endpointSync: EndpointSyncPayload;
  subscriptionSnapshot?: ClientSubscriptionSnapshot | null;
}

export interface ClientSyncResponse {
  syncedAt: string;
  settingsJson: string;
  assetsJson: string;
  aiSubscription: Settings["ai"]["subscription"];
  endpointSync: EndpointSyncPayload;
  subscriptionSnapshot?: ClientSubscriptionSnapshot | null;
}

export interface ClientCloudAssetPayload {
  records: CloudAssetRecord[];
}

export interface AdminBillingOverview {
  billingMonth: string;
  estimatedMonthlyRevenue: number;
  outstandingAmount: number;
  openInvoiceCount: number;
}

export interface GenerateBillingCycleResponse {
  billing: AdminBillingOverview;
  generatedInvoices: number;
}

export interface ClientAiRuntimeResponse {
  enabled: boolean;
  reason: string;
  provider: Settings["ai"]["providerType"];
  baseUrl: string;
  apiKey: string;
  modelName: string;
  usingManagedEndpoint: boolean;
}

function mapClientSubscriptionSnapshot(
  payload?: ClientSubscriptionSnapshotPayload | null,
): ClientSubscriptionSnapshot | null {
  if (!payload) {
    return null;
  }

  return {
    subscription:
      mapCloudSubscription(payload.subscription) ?? {
        plan: "free",
        planDisplayName: "Free",
        status: "inactive",
        seats: 1,
        pricePerSeat: 0,
        currency: "USD",
        billingScope: "global",
        startedAt: null,
        renewalAt: null,
        allowCustomEndpoint: true,
        useCustomEndpoint: true,
        syncToCloud: true,
      },
    currentInvoice: payload.currentInvoice
      ? {
          ...payload.currentInvoice,
          lineItems: payload.currentInvoice.lineItems ?? [],
          payments: payload.currentInvoice.payments ?? [],
        }
      : null,
    recentInvoices: payload.recentInvoices?.map((invoice) => ({
      ...invoice,
      lineItems: invoice.lineItems ?? [],
      payments: invoice.payments ?? [],
    })) ?? [],
    paymentProviders: payload.paymentProviders ?? [],
    usage: {
      ...payload.usage,
      topAccounts: payload.usage.topAccounts ?? [],
    },
  };
}

function normalizeBaseUrl(baseUrl?: string | null) {
  return (baseUrl?.trim() || "http://localhost:5047").replace(/\/+$/, "");
}

export function mapCloudSubscription(
  payload?: CloudSubscriptionPayload | null,
): Settings["ai"]["subscription"] | null {
  if (!payload) {
    return null;
  }

  return {
    plan: payload.planName as Settings["ai"]["subscription"]["plan"],
    planDisplayName: payload.planDisplayName || payload.planName,
    status: payload.status,
    seats: payload.seats,
    pricePerSeat: payload.pricePerSeat ?? 0,
    currency: payload.currency ?? "USD",
    billingScope: payload.billingScope ?? "global",
    startedAt: null,
    renewalAt: payload.renewAt ? Date.parse(payload.renewAt) : null,
    allowCustomEndpoint: payload.allowCustomEndpoint,
    useCustomEndpoint: false,
    syncToCloud: payload.syncCustomEndpoint,
  };
}

function buildMappedSubscription() {
  return {
    plan: "free" as const,
    planDisplayName: "Free",
    status: "inactive" as const,
    seats: 1,
    pricePerSeat: 0,
    currency: "USD",
    billingScope: "global" as const,
    startedAt: null,
    renewalAt: null,
    allowCustomEndpoint: true,
    useCustomEndpoint: true,
    syncToCloud: true,
  };
}

function mapLoginResponse(payload: ClientLoginResponsePayload): ClientLoginResponse {
  return {
    ...payload,
    aiSubscription: mapCloudSubscription(payload.aiSubscription) ?? buildMappedSubscription(),
    subscriptionSnapshot: mapClientSubscriptionSnapshot(payload.subscriptionSnapshot),
  };
}

function mapSyncResponse(payload: ClientSyncResponsePayload): ClientSyncResponse {
  return {
    ...payload,
    aiSubscription: mapCloudSubscription(payload.aiSubscription) ?? buildMappedSubscription(),
    subscriptionSnapshot: mapClientSubscriptionSnapshot(payload.subscriptionSnapshot),
  };
}

export const cloudService = {
  async login(baseUrl: string | null | undefined, request: ClientLoginRequest) {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw new Error(`Cloud login failed with status ${response.status}`);
    }
    const payload = (await response.json()) as ClientLoginResponsePayload;
    return mapLoginResponse(payload);
  },

  async refresh(baseUrl: string | null | undefined, refreshToken: string) {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refreshToken }),
    });
    if (!response.ok) {
      throw new Error(`Cloud refresh failed with status ${response.status}`);
    }
    const payload = (await response.json()) as ClientLoginResponsePayload;
    return mapLoginResponse(payload);
  },

  async syncSettings(baseUrl: string | null | undefined, request: Record<string, unknown>) {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/sync/settings`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw new Error(`Settings sync failed with status ${response.status}`);
    }
    const payload = (await response.json()) as ClientSyncResponsePayload;
    return mapSyncResponse(payload);
  },

  async syncAssets(baseUrl: string | null | undefined, request: Record<string, unknown>) {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/sync/assets`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw new Error(`Asset sync failed with status ${response.status}`);
    }
    const payload = (await response.json()) as ClientSyncResponsePayload;
    return mapSyncResponse(payload);
  },

  async pull(
    baseUrl: string | null | undefined,
    mode: string,
    accountKey: string,
    accessToken: string,
  ) {
    const params = new URLSearchParams({
      mode,
      accountKey,
      accessToken,
    });
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/sync/pull?${params.toString()}`);
    if (!response.ok) {
      throw new Error(`Cloud pull failed with status ${response.status}`);
    }
    const payload = (await response.json()) as ClientSyncResponsePayload;
    return mapSyncResponse(payload);
  },

  async generateCurrentBillingCycle(baseUrl: string | null | undefined, accessToken: string) {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/admin/billing/generate-current-cycle`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });
    if (!response.ok) {
      throw new Error(`Billing generation failed with status ${response.status}`);
    }
    return (await response.json()) as GenerateBillingCycleResponse;
  },

  async getClientAiRuntime(baseUrl: string | null | undefined, accessToken: string) {
    const params = new URLSearchParams({ accessToken });
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/ai/runtime?${params.toString()}`);
    if (!response.ok) {
      throw new Error(`AI runtime fetch failed with status ${response.status}`);
    }
    return (await response.json()) as ClientAiRuntimeResponse;
  },

  async getClientSubscriptionSnapshot(baseUrl: string | null | undefined, accessToken: string) {
    const params = new URLSearchParams({ accessToken });
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/subscription?${params.toString()}`);
    if (!response.ok) {
      throw new Error(`Subscription snapshot fetch failed with status ${response.status}`);
    }
    return mapClientSubscriptionSnapshot((await response.json()) as ClientSubscriptionSnapshotPayload);
  },

  async createClientCheckoutSession(
    baseUrl: string | null | undefined,
    accessToken: string,
    request: {
      invoiceId: string;
      providerKey: string;
      returnUrl: string;
      cancelUrl: string;
    },
  ) {
    const params = new URLSearchParams({ accessToken });
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/billing/checkout-sessions?${params.toString()}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw new Error(`Client checkout creation failed with status ${response.status}`);
    }
    return (await response.json()) as ClientSubscriptionPaymentPayload;
  },

  async proxyManagedOpenAi(
    baseUrl: string | null | undefined,
    accessToken: string,
    payload: Record<string, unknown>,
  ) {
    const params = new URLSearchParams({ accessToken });
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/ai/proxy/openai?${params.toString()}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      throw new Error(`Managed OpenAI proxy failed with status ${response.status} - ${await response.text()}`);
    }
    return response.json();
  },

  async proxyManagedAnthropic(
    baseUrl: string | null | undefined,
    accessToken: string,
    payload: Record<string, unknown>,
    anthropicVersion: string,
    anthropicBeta?: string,
    codingToolHeader?: string,
  ) {
    const params = new URLSearchParams({ accessToken });
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "anthropic-version": anthropicVersion,
    };
    if (anthropicBeta) {
      headers["anthropic-beta"] = anthropicBeta;
    }
    if (codingToolHeader) {
      headers["x-coding-tool"] = codingToolHeader;
    }

    const response = await fetch(`${normalizeBaseUrl(baseUrl)}/api/client/ai/proxy/anthropic?${params.toString()}`, {
      method: "POST",
      headers,
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      throw new Error(`Managed Anthropic proxy failed with status ${response.status} - ${await response.text()}`);
    }
    return response.json();
  },
};
