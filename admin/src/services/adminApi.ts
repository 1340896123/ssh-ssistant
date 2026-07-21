import type {
  AdminLoginResponse,
  BillingInvoiceSummary,
  BillingOverview,
  DashboardSnapshot,
  GenerateBillingCycleResponse,
  PersonalAccountSummary,
  PersonalSubscriptionSummary,
  PaymentTransactionSummary,
  AiSubscriptionOverview,
  AiSubscriptionPlanSummary,
  AiUsagePricingSummary,
  AiUsageSummary,
} from '../types/admin'
import {
  normalizeBillingInvoiceStatus,
  normalizeSubscriptionStatus,
} from '../utils/adminFormat'

function withJson(init?: RequestInit) {
  const headers = new Headers(init?.headers ?? {})
  headers.set('Content-Type', 'application/json')
  return {
    ...init,
    headers,
  }
}

async function parseResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    let detail = ''
    try {
      const payload = await response.json()
      detail = typeof payload?.error === 'string' ? payload.error : JSON.stringify(payload)
    } catch {
      detail = await response.text()
    }
    throw new Error(`HTTP ${response.status}: ${detail || response.statusText}`)
  }

  if (response.status === 204) {
    return undefined as T
  }

  return (await response.json()) as T
}

function normalizePersonalAccount(entity: PersonalAccountSummary): PersonalAccountSummary {
  return {
    ...entity,
    subscriptionStatus: normalizeSubscriptionStatus(entity.subscriptionStatus),
  }
}

function normalizePlan(entity: AiSubscriptionPlanSummary): AiSubscriptionPlanSummary {
  return { ...entity }
}

function normalizePersonalSubscription(entity: PersonalSubscriptionSummary): PersonalSubscriptionSummary {
  return {
    ...entity,
    status: normalizeSubscriptionStatus(entity.status),
  }
}

function normalizeAiSubscription(entity: AiSubscriptionOverview): AiSubscriptionOverview {
  return {
    ...entity,
    status: normalizeSubscriptionStatus(entity.status),
  }
}

function normalizeBillingInvoice(invoice: BillingInvoiceSummary): BillingInvoiceSummary {
  return {
    ...invoice,
    status: normalizeBillingInvoiceStatus(invoice.status),
    lineItems: invoice.lineItems.map((item) => ({ ...item })),
    payments: invoice.payments.map((item) => ({ ...item })),
  }
}

function normalizeBillingOverview(overview: BillingOverview): BillingOverview {
  return {
    ...overview,
    recentInvoices: overview.recentInvoices.map(normalizeBillingInvoice),
  }
}

function normalizeAiUsage(entity: AiUsageSummary): AiUsageSummary {
  return {
    ...entity,
    topAccounts: entity.topAccounts.map((item) => ({ ...item })),
  }
}

function normalizeDashboard(snapshot: DashboardSnapshot): DashboardSnapshot {
  return {
    ...snapshot,
    personalAccounts: snapshot.personalAccounts.map(normalizePersonalAccount),
    assets: snapshot.assets.map((item) => ({ ...item })),
    subscriptionPlans: snapshot.subscriptionPlans.map(normalizePlan),
    personalSubscriptions: snapshot.personalSubscriptions.map(normalizePersonalSubscription),
    aiUsagePricing: snapshot.aiUsagePricing.map((item: AiUsagePricingSummary) => ({ ...item })),
    paymentProviders: snapshot.paymentProviders.map((item) => ({ ...item })),
    billing: normalizeBillingOverview(snapshot.billing),
    aiUsage: normalizeAiUsage(snapshot.aiUsage),
    aiSubscription: normalizeAiSubscription(snapshot.aiSubscription),
    endpointSync: { ...snapshot.endpointSync },
  }
}

export async function adminLogin(apiBase: string, payload: { username: string; password: string }) {
  const response = await fetch(`${apiBase}/login`, withJson({ method: 'POST', body: JSON.stringify(payload) }))
  return parseResponse<AdminLoginResponse>(response)
}

export async function adminRefresh(apiBase: string, refreshToken: string) {
  const response = await fetch(
    `${apiBase}/refresh`,
    withJson({ method: 'POST', body: JSON.stringify({ refreshToken }) }),
  )
  return parseResponse<AdminLoginResponse>(response)
}

export async function adminRequest<T>(
  apiBase: string,
  token: string,
  path: string,
  init?: RequestInit,
): Promise<T> {
  const headers = new Headers(init?.headers ?? {})
  if (token) {
    headers.set('Authorization', `Bearer ${token}`)
  }

  const response = await fetch(`${apiBase}${path}`, {
    ...init,
    headers,
  })

  return parseResponse<T>(response)
}

export function loadAdminDashboard(apiBase: string, token: string) {
  return adminRequest<DashboardSnapshot>(apiBase, token, '/dashboard').then(normalizeDashboard)
}

export function createCheckoutSession(apiBase: string, token: string, payload: Record<string, unknown>) {
  return adminRequest<PaymentTransactionSummary>(apiBase, token, '/billing/checkout-sessions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
}

export function generateBillingCycle(apiBase: string, token: string) {
  return adminRequest<GenerateBillingCycleResponse>(apiBase, token, '/billing/generate-current-cycle', {
    method: 'POST',
  })
}
