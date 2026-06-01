import type { BillingInvoiceStatus, SubscriptionStatus } from '../types/admin'

const subscriptionStatusByIndex: SubscriptionStatus[] = ['inactive', 'trialing', 'active', 'pastDue', 'cancelled']
const billingStatusByIndex: BillingInvoiceStatus[] = ['open', 'paid', 'overdue', 'voided']

export function parseApiError(error: unknown) {
  if (!(error instanceof Error)) return String(error)

  const prefix = 'HTTP '
  if (!error.message.startsWith(prefix)) return error.message

  const detail = error.message.slice(prefix.length).trim()
  const separatorIndex = detail.indexOf(':')
  if (separatorIndex === -1) return `请求失败：${detail}`

  const status = detail.slice(0, separatorIndex).trim()
  const message = detail.slice(separatorIndex + 1).trim()
  return `请求失败（${status}）：${message}`
}

export function money(value: number, currency = 'USD') {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
    maximumFractionDigits: 0,
  }).format(value)
}

export function formatDate(value?: string | null, includeTime = false) {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return includeTime ? date.toLocaleString() : date.toLocaleDateString()
}

export function riskClass(risk: string) {
  if (risk === 'critical') return 'admin-badge admin-badge-danger'
  if (risk === 'high') return 'admin-badge admin-badge-warning'
  if (risk === 'low') return 'admin-badge admin-badge-success'
  return 'admin-badge admin-badge-neutral'
}

export function normalizeSubscriptionStatus(value: unknown): SubscriptionStatus {
  if (typeof value === 'number' && subscriptionStatusByIndex[value]) {
    return subscriptionStatusByIndex[value]
  }

  const normalized = String(value ?? '')
    .trim()
    .toLowerCase()
    .replace(/[\s_-]+/g, '')

  if (normalized === 'trialing') return 'trialing'
  if (normalized === 'active') return 'active'
  if (normalized === 'pastdue') return 'pastDue'
  if (normalized === 'cancelled') return 'cancelled'
  return 'inactive'
}

export function serializeSubscriptionStatus(status: SubscriptionStatus) {
  if (status === 'trialing') return 1
  if (status === 'active') return 2
  if (status === 'pastDue') return 3
  if (status === 'cancelled') return 4
  return 0
}

export function statusClass(status: SubscriptionStatus | string) {
  if (status === 'active') return 'admin-badge admin-badge-success'
  if (status === 'trialing' || status === 'open') return 'admin-badge admin-badge-info'
  if (status === 'pastDue' || status === 'overdue') return 'admin-badge admin-badge-warning'
  if (status === 'cancelled' || status === 'voided') return 'admin-badge admin-badge-neutral'
  return 'admin-badge admin-badge-danger'
}

export function normalizeBillingInvoiceStatus(value: unknown): BillingInvoiceStatus {
  if (typeof value === 'number' && billingStatusByIndex[value]) {
    return billingStatusByIndex[value]
  }

  const normalized = String(value ?? '')
    .trim()
    .toLowerCase()
    .replace(/[\s_-]+/g, '')

  if (normalized === 'paid') return 'paid'
  if (normalized === 'overdue') return 'overdue'
  if (normalized === 'voided') return 'voided'
  return 'open'
}

export function serializeBillingInvoiceStatus(status: BillingInvoiceStatus) {
  if (status === 'paid') return 1
  if (status === 'overdue') return 2
  if (status === 'voided') return 3
  return 0
}

export function invoiceBadge(status: BillingInvoiceStatus) {
  if (status === 'paid') return 'admin-badge admin-badge-success'
  if (status === 'overdue') return 'admin-badge admin-badge-danger'
  if (status === 'voided') return 'admin-badge admin-badge-neutral'
  return 'admin-badge admin-badge-warning'
}
