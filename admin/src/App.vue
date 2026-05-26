<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'

const DEFAULT_CHECKOUT_RETURN_URL = 'sshstar://billing/success'
const DEFAULT_CHECKOUT_CANCEL_URL = 'sshstar://billing/cancel'

type SubscriptionStatus = 'inactive' | 'trialing' | 'active' | 'pastDue' | 'cancelled'
type BillingInvoiceStatus = 'open' | 'paid' | 'overdue' | 'voided'

interface EnterpriseSummary {
  id: string
  name: string
  seatCount: number
  activeSubAccounts: number
  subscriptionPlan: string
  subscriptionStatus: SubscriptionStatus
  renewAt: string
}

interface EnterpriseSubAccountSummary {
  id: string
  enterpriseId: string
  displayName: string
  email: string
  enabled: boolean
  assetIds: string[]
  updatedAt: string
}

interface PersonalAccountSummary {
  id: string
  displayName: string
  email: string
  subscriptionStatus: SubscriptionStatus
  planName: string
  customEndpointEnabled: boolean
  updatedAt: string
}

interface ManagedAssetSummary {
  id: string
  name: string
  host: string
  environment: string
  riskLevel: string
  ownerType: string
}

interface AiSubscriptionPlanSummary {
  code: string
  displayName: string
  scope: string
  pricePerSeat: number
  currency: string
  allowCustomEndpoint: boolean
  isActive: boolean
  description: string
  updatedAt: string
}

interface EnterpriseSubscriptionSummary {
  enterpriseId: string
  planCode: string
  planDisplayName: string
  status: SubscriptionStatus
  seatsPurchased: number
  seatsAssigned: number
  pricePerSeat: number
  currency: string
  allowCustomEndpoint: boolean
  renewAt: string
  updatedAt: string
}

interface PersonalSubscriptionSummary {
  accountId: string
  planCode: string
  planDisplayName: string
  status: SubscriptionStatus
  pricePerSeat: number
  currency: string
  allowCustomEndpoint: boolean
  renewAt: string
  updatedAt: string
}

interface AiSubscriptionOverview {
  serviceMode: string
  planName: string
  planDisplayName: string
  status: SubscriptionStatus
  seats: number
  pricePerSeat: number
  currency: string
  billingScope: string
  allowCustomEndpoint: boolean
  syncCustomEndpoint: boolean
  renewAt: string
}

interface AiEndpointSyncSettings {
  endpointName: string
  provider: string
  baseUrl: string
  apiKey: string
  modelName: string
  syncToClients: boolean
  updatedAt: string
}

interface BillingInvoiceSummary {
  id: string
  targetType: string
  targetId: string
  planCode: string
  status: BillingInvoiceStatus
  seatCount: number
  unitPrice: number
  subscriptionAmount: number
  aiUsageAmount: number
  totalAmount: number
  currency: string
  billingMonth: string
  dueAt: string
  createdAt: string
  updatedAt: string
  paidAmount: number
  remainingAmount: number
  lineItems: BillingInvoiceLineItemSummary[]
  payments: PaymentTransactionSummary[]
}

interface BillingInvoiceLineItemSummary {
  id: string
  invoiceId: string
  itemType: string
  description: string
  quantity: number
  unitPrice: number
  amount: number
  currency: string
  totalTokens?: number | null
  createdAt: string
}

interface PaymentTransactionSummary {
  id: string
  invoiceId: string
  targetType: string
  targetId: string
  providerKey: string
  amount: number
  currency: string
  paymentMethod: string
  status: string
  externalReference: string
  note: string
  checkoutUrl: string
  expiresAt?: string | null
  paidAt?: string | null
  createdAt: string
  updatedAt: string
}

interface PaymentProviderConfigSummary {
  providerKey: string
  displayName: string
  providerType: string
  webhookSecret: string
  enabled: boolean
  metadataJson: string
  checkoutBaseUrl: string
  webhookMode: string
  apiBaseUrl: string
  secretApiKey: string
  stripeApiVersion: string
  webhookToleranceSeconds: number
  successUrl: string
  cancelUrl: string
  updatedAt: string
}

interface BillingOverview {
  billingMonth: string
  estimatedMonthlyRevenue: number
  outstandingAmount: number
  openInvoiceCount: number
  recentInvoices: BillingInvoiceSummary[]
}

interface AiUsageAccountSummary {
  accountId: string
  accountMode: string
  requests: number
  totalTokens: number
  estimatedCost: number
  currency: string
}

interface AiUsageSummary {
  billingMonth: string
  totalRequests: number
  managedRequests: number
  promptTokens: number
  completionTokens: number
  totalTokens: number
  estimatedCost: number
  currency: string
  topAccounts: AiUsageAccountSummary[]
}

interface AiUsagePricingSummary {
  id: string
  provider: string
  modelName: string
  promptTokenRatePerMillion: number
  completionTokenRatePerMillion: number
  currency: string
  isActive: boolean
  updatedAt: string
}

interface GenerateBillingCycleResponse {
  billing: BillingOverview
  generatedInvoices: number
}

interface DashboardSnapshot {
  enterprises: EnterpriseSummary[]
  subAccounts: EnterpriseSubAccountSummary[]
  personalAccounts: PersonalAccountSummary[]
  assets: ManagedAssetSummary[]
  subscriptionPlans: AiSubscriptionPlanSummary[]
  enterpriseSubscriptions: EnterpriseSubscriptionSummary[]
  personalSubscriptions: PersonalSubscriptionSummary[]
  aiUsagePricing: AiUsagePricingSummary[]
  paymentProviders: PaymentProviderConfigSummary[]
  billing: BillingOverview
  aiUsage: AiUsageSummary
  aiSubscription: AiSubscriptionOverview
  endpointSync: AiEndpointSyncSettings
}

interface AdminLoginResponse {
  token: string
  refreshToken: string
  username: string
  role: string
  expiresAt: string
  refreshExpiresAt: string
}

const apiBase = ref('http://localhost:5047/api/admin')
const loading = ref(false)
const error = ref('')
const notice = ref('')
const dashboard = ref<DashboardSnapshot | null>(null)
const selectedSubAccountId = ref('')
const selectedAssetIds = ref<string[]>([])
const authToken = ref(localStorage.getItem('admin-auth-token') || '')
const refreshToken = ref(localStorage.getItem('admin-refresh-token') || '')
const adminUsername = ref(localStorage.getItem('admin-username') || '')
const authExpiresAt = ref(Number(localStorage.getItem('admin-auth-expires-at') || 0))
const refreshExpiresAt = ref(Number(localStorage.getItem('admin-refresh-expires-at') || 0))

const loginForm = reactive({
  username: adminUsername.value || 'admin',
  password: 'admin123',
})

const enterpriseForm = reactive({
  id: '',
  name: '',
  seatCount: 10,
  subscriptionPlan: 'enterprise',
  subscriptionStatus: 'active' as SubscriptionStatus,
})

const subAccountForm = reactive({
  id: '',
  enterpriseId: 'ent-acme',
  displayName: '',
  email: '',
  secret: '',
  enabled: true,
})

const personalForm = reactive({
  id: '',
  displayName: '',
  email: '',
  secret: '',
  subscriptionStatus: 'inactive' as SubscriptionStatus,
  planName: 'free',
  customEndpointEnabled: false,
})

const subscriptionForm = ref<AiSubscriptionOverview>({
  serviceMode: 'subscription',
  planName: 'enterprise',
  planDisplayName: 'Enterprise Managed',
  status: 'active',
  seats: 1,
  pricePerSeat: 49,
  currency: 'USD',
  billingScope: 'global',
  allowCustomEndpoint: true,
  syncCustomEndpoint: true,
  renewAt: new Date().toISOString(),
})

const planForm = reactive({
  code: 'business',
  displayName: 'Business Monthly',
  scope: 'enterprise',
  pricePerSeat: 39,
  currency: 'USD',
  allowCustomEndpoint: true,
  isActive: true,
  description: '',
})

const enterpriseSubscriptionForm = reactive({
  enterpriseId: 'ent-acme',
  planCode: 'enterprise',
  status: 'active' as SubscriptionStatus,
  seatsPurchased: 40,
})

const personalSubscriptionForm = reactive({
  accountId: 'usr-amy',
  planCode: 'personal',
  status: 'active' as SubscriptionStatus,
})

const aiUsagePricingForm = reactive({
  id: 'price-openai-gpt-4o-mini',
  provider: 'openai',
  modelName: 'gpt-4o-mini',
  promptTokenRatePerMillion: 0.15,
  completionTokenRatePerMillion: 0.60,
  currency: 'USD',
  isActive: true,
})

const endpointForm = ref<AiEndpointSyncSettings>({
  endpointName: '',
  provider: 'openai',
  baseUrl: '',
  apiKey: '',
  modelName: '',
  syncToClients: true,
  updatedAt: new Date().toISOString(),
})

const invoiceStatusForm = reactive<Record<string, BillingInvoiceStatus>>({})
const paymentForm = reactive({
  invoiceId: '',
  amount: 0,
  providerKey: 'manual',
  currency: 'USD',
  paymentMethod: 'manual',
  status: 'completed',
  externalReference: '',
  note: '',
})
const paymentProviderForm = reactive({
  providerKey: 'manual',
  displayName: 'Manual Reconciliation',
  providerType: 'manual',
  webhookSecret: '',
  enabled: true,
  metadataJson: '{}',
  checkoutBaseUrl: 'https://payments.example.com/manual-checkout',
  webhookMode: 'manual',
  apiBaseUrl: 'https://api.stripe.com',
  secretApiKey: '',
  stripeApiVersion: '2024-06-20',
  webhookToleranceSeconds: 300,
  successUrl: DEFAULT_CHECKOUT_RETURN_URL,
  cancelUrl: DEFAULT_CHECKOUT_CANCEL_URL,
})

const selectedSubAccount = computed(() =>
  dashboard.value?.subAccounts.find((item) => item.id === selectedSubAccountId.value) ?? null,
)

const isAuthenticated = computed(() => Boolean(authToken.value))
const monthlyRevenueEstimate = computed(() => {
  return dashboard.value?.billing.estimatedMonthlyRevenue ?? 0
})

const outstandingAmount = computed(() => dashboard.value?.billing.outstandingAmount ?? 0)
const paymentProviderHint = computed(() => {
  if (paymentProviderForm.providerType === 'stripe') {
    return `{"checkoutBaseUrl":"https://payments.example.com/stripe-checkout","apiBaseUrl":"https://api.stripe.com","stripeApiVersion":"2024-06-20","webhookMode":"stripe-like","webhookToleranceSeconds":300,"successUrl":"${DEFAULT_CHECKOUT_RETURN_URL}","cancelUrl":"${DEFAULT_CHECKOUT_CANCEL_URL}"}`
  }
  if (paymentProviderForm.providerType === 'manual') {
    return '{"checkoutBaseUrl":"https://payments.example.com/manual-checkout","webhookMode":"manual"}'
  }
  return '{"checkoutBaseUrl":"https://payments.example.com/provider-checkout"}'
})

function parseApiError(error: unknown) {
  if (!(error instanceof Error)) {
    return String(error)
  }

  const prefix = 'HTTP '
  if (!error.message.startsWith(prefix)) {
    return error.message
  }

  const detail = error.message.slice(prefix.length).trim()
  const separatorIndex = detail.indexOf(':')
  if (separatorIndex === -1) {
    return `请求失败：${detail}`
  }

  const status = detail.slice(0, separatorIndex).trim()
  const message = detail.slice(separatorIndex + 1).trim()
  return `请求失败（${status}）：${message}`
}

function clearFeedback() {
  error.value = ''
  notice.value = ''
}

function resetEnterpriseForm() {
  enterpriseForm.id = ''
  enterpriseForm.name = ''
  enterpriseForm.seatCount = 10
  enterpriseForm.subscriptionPlan = 'enterprise'
  enterpriseForm.subscriptionStatus = 'active'
}

function resetSubAccountForm() {
  subAccountForm.id = ''
  subAccountForm.enterpriseId = dashboard.value?.enterprises[0]?.id ?? 'ent-acme'
  subAccountForm.displayName = ''
  subAccountForm.email = ''
  subAccountForm.secret = ''
  subAccountForm.enabled = true
}

function resetPersonalForm() {
  personalForm.id = ''
  personalForm.displayName = ''
  personalForm.email = ''
  personalForm.secret = ''
  personalForm.subscriptionStatus = 'inactive'
  personalForm.planName = 'free'
  personalForm.customEndpointEnabled = false
}

function riskClass(risk: string) {
  if (risk === 'critical') return 'bg-rose-100 text-rose-700 ring-rose-200'
  if (risk === 'high') return 'bg-amber-100 text-amber-800 ring-amber-200'
  if (risk === 'low') return 'bg-emerald-100 text-emerald-700 ring-emerald-200'
  return 'bg-slate-100 text-slate-700 ring-slate-200'
}

function money(value: number, currency = 'USD') {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
    maximumFractionDigits: 0,
  }).format(value)
}

function invoiceBadge(status: BillingInvoiceStatus) {
  if (status === 'paid') return 'bg-emerald-100 text-emerald-700 ring-emerald-200'
  if (status === 'overdue') return 'bg-rose-100 text-rose-700 ring-rose-200'
  if (status === 'voided') return 'bg-slate-200 text-slate-600 ring-slate-300'
  return 'bg-amber-100 text-amber-700 ring-amber-200'
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers ?? {})
  if (authToken.value) {
    headers.set('Authorization', `Bearer ${authToken.value}`)
  }

  const response = await fetch(`${apiBase.value}${path}`, {
    ...init,
    headers,
  })

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
  if (response.status === 204) return undefined as T
  return (await response.json()) as T
}

async function login() {
  clearFeedback()
  try {
    const response = await fetch(`${apiBase.value}/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(loginForm),
    })
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
    const payload = (await response.json()) as AdminLoginResponse
    authToken.value = payload.token
    refreshToken.value = payload.refreshToken
    adminUsername.value = payload.username
    authExpiresAt.value = Date.parse(payload.expiresAt)
    refreshExpiresAt.value = Date.parse(payload.refreshExpiresAt)
    localStorage.setItem('admin-auth-token', payload.token)
    localStorage.setItem('admin-refresh-token', payload.refreshToken)
    localStorage.setItem('admin-username', payload.username)
    localStorage.setItem('admin-auth-expires-at', String(authExpiresAt.value))
    localStorage.setItem('admin-refresh-expires-at', String(refreshExpiresAt.value))
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

function logout() {
  authToken.value = ''
  refreshToken.value = ''
  adminUsername.value = ''
  authExpiresAt.value = 0
  refreshExpiresAt.value = 0
  localStorage.removeItem('admin-auth-token')
  localStorage.removeItem('admin-refresh-token')
  localStorage.removeItem('admin-username')
  localStorage.removeItem('admin-auth-expires-at')
  localStorage.removeItem('admin-refresh-expires-at')
  dashboard.value = null
}

async function refreshAdminSession() {
  if (!refreshToken.value) {
    logout()
    return
  }

  const response = await fetch(`${apiBase.value}/refresh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refreshToken: refreshToken.value }),
  })

  if (!response.ok) {
    logout()
    throw new Error(`HTTP ${response.status}`)
  }

  const payload = (await response.json()) as AdminLoginResponse
  authToken.value = payload.token
  refreshToken.value = payload.refreshToken
  adminUsername.value = payload.username
  authExpiresAt.value = Date.parse(payload.expiresAt)
  refreshExpiresAt.value = Date.parse(payload.refreshExpiresAt)
  localStorage.setItem('admin-auth-token', payload.token)
  localStorage.setItem('admin-refresh-token', payload.refreshToken)
  localStorage.setItem('admin-username', payload.username)
  localStorage.setItem('admin-auth-expires-at', String(authExpiresAt.value))
  localStorage.setItem('admin-refresh-expires-at', String(refreshExpiresAt.value))
}

async function loadDashboard() {
  if (!authToken.value) return
  loading.value = true
  clearFeedback()
  try {
    const payload = await api<DashboardSnapshot>('/dashboard')
    dashboard.value = payload
    subscriptionForm.value = { ...payload.aiSubscription }
    endpointForm.value = { ...payload.endpointSync }
    if (payload.subscriptionPlans.length > 0) {
      const recommendedPlan = payload.subscriptionPlans[0]
      planForm.code = recommendedPlan.code
      planForm.displayName = recommendedPlan.displayName
      planForm.scope = recommendedPlan.scope
      planForm.pricePerSeat = recommendedPlan.pricePerSeat
      planForm.currency = recommendedPlan.currency
      planForm.allowCustomEndpoint = recommendedPlan.allowCustomEndpoint
      planForm.isActive = recommendedPlan.isActive
      planForm.description = recommendedPlan.description
    }
    if (payload.enterpriseSubscriptions.length > 0) {
      const enterpriseSubscription = payload.enterpriseSubscriptions[0]
      enterpriseSubscriptionForm.enterpriseId = enterpriseSubscription.enterpriseId
      enterpriseSubscriptionForm.planCode = enterpriseSubscription.planCode
      enterpriseSubscriptionForm.status = enterpriseSubscription.status
      enterpriseSubscriptionForm.seatsPurchased = enterpriseSubscription.seatsPurchased
    }
    if (payload.personalSubscriptions.length > 0) {
      const personalSubscription = payload.personalSubscriptions[0]
      personalSubscriptionForm.accountId = personalSubscription.accountId
      personalSubscriptionForm.planCode = personalSubscription.planCode
      personalSubscriptionForm.status = personalSubscription.status
    }
    if (payload.aiUsagePricing.length > 0) {
      const pricing = payload.aiUsagePricing[0]
      aiUsagePricingForm.id = pricing.id
      aiUsagePricingForm.provider = pricing.provider
      aiUsagePricingForm.modelName = pricing.modelName
      aiUsagePricingForm.promptTokenRatePerMillion = pricing.promptTokenRatePerMillion
      aiUsagePricingForm.completionTokenRatePerMillion = pricing.completionTokenRatePerMillion
      aiUsagePricingForm.currency = pricing.currency
      aiUsagePricingForm.isActive = pricing.isActive
    }
    if (payload.paymentProviders.length > 0) {
      const provider = payload.paymentProviders[0]
      paymentProviderForm.providerKey = provider.providerKey
      paymentProviderForm.displayName = provider.displayName
      paymentProviderForm.providerType = provider.providerType
      paymentProviderForm.webhookSecret = provider.webhookSecret
      paymentProviderForm.enabled = provider.enabled
      paymentProviderForm.metadataJson = provider.metadataJson
      paymentProviderForm.checkoutBaseUrl = provider.checkoutBaseUrl || paymentProviderForm.checkoutBaseUrl
      paymentProviderForm.webhookMode = provider.webhookMode || paymentProviderForm.webhookMode
      paymentProviderForm.apiBaseUrl = provider.apiBaseUrl || paymentProviderForm.apiBaseUrl
      paymentProviderForm.secretApiKey = provider.secretApiKey || paymentProviderForm.secretApiKey
      paymentProviderForm.stripeApiVersion = provider.stripeApiVersion || paymentProviderForm.stripeApiVersion
      paymentProviderForm.webhookToleranceSeconds = provider.webhookToleranceSeconds || paymentProviderForm.webhookToleranceSeconds
      paymentProviderForm.successUrl = provider.successUrl || paymentProviderForm.successUrl
      paymentProviderForm.cancelUrl = provider.cancelUrl || paymentProviderForm.cancelUrl
      paymentForm.providerKey = provider.providerKey
      paymentForm.paymentMethod = provider.providerType
    }
    Object.keys(invoiceStatusForm).forEach((key) => delete invoiceStatusForm[key])
    payload.billing.recentInvoices.forEach((invoice) => {
      invoiceStatusForm[invoice.id] = invoice.status
    })
    if (!selectedSubAccountId.value && payload.subAccounts.length > 0) {
      selectedSubAccountId.value = payload.subAccounts[0].id
      selectedAssetIds.value = [...payload.subAccounts[0].assetIds]
    }
  } catch (err) {
    error.value = parseApiError(err)
    if (error.value.includes('401')) {
      logout()
    }
  } finally {
    loading.value = false
  }
}

async function saveEnterprise() {
  clearFeedback()
  try {
    await api('/enterprises', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ...enterpriseForm,
        renewAt: new Date(Date.now() + 30 * 86400_000).toISOString(),
      }),
    })
    notice.value = '企业账号已保存'
    resetEnterpriseForm()
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

function editEnterprise(enterprise: EnterpriseSummary) {
  enterpriseForm.id = enterprise.id
  enterpriseForm.name = enterprise.name
  enterpriseForm.seatCount = enterprise.seatCount
  enterpriseForm.subscriptionPlan = enterprise.subscriptionPlan
  enterpriseForm.subscriptionStatus = enterprise.subscriptionStatus
}

async function deleteEnterprise(id: string) {
  clearFeedback()
  try {
    await api(`/enterprises/${id}`, { method: 'DELETE' })
    notice.value = '企业账号已删除'
    if (enterpriseForm.id === id) {
      resetEnterpriseForm()
    }
    if (subAccountForm.enterpriseId === id) {
      resetSubAccountForm()
    }
    if (selectedSubAccount.value?.enterpriseId === id) {
      selectedSubAccountId.value = ''
      selectedAssetIds.value = []
    }
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

async function saveSubAccount() {
  clearFeedback()
  try {
    const assetIds =
      selectedSubAccountId.value === subAccountForm.id
        ? selectedAssetIds.value
        : dashboard.value?.subAccounts.find((item) => item.id === subAccountForm.id)?.assetIds ?? []
    await api('/sub-accounts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        ...subAccountForm,
        assetIds,
      }),
    })
    notice.value = subAccountForm.enabled ? '企业子账号已保存' : '企业子账号已禁用'
    resetSubAccountForm()
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

function editSubAccount(subAccount: EnterpriseSubAccountSummary) {
  subAccountForm.id = subAccount.id
  subAccountForm.enterpriseId = subAccount.enterpriseId
  subAccountForm.displayName = subAccount.displayName
  subAccountForm.email = subAccount.email
  subAccountForm.enabled = subAccount.enabled
  subAccountForm.secret = ''
  selectedSubAccountId.value = subAccount.id
  selectedAssetIds.value = [...subAccount.assetIds]
}

async function deleteSubAccount(id: string) {
  clearFeedback()
  try {
    await api(`/sub-accounts/${id}`, { method: 'DELETE' })
    notice.value = '企业子账号已删除'
    if (selectedSubAccountId.value === id) {
      selectedSubAccountId.value = ''
      selectedAssetIds.value = []
    }
    if (subAccountForm.id === id) {
      resetSubAccountForm()
    }
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

async function savePersonalAccount() {
  clearFeedback()
  try {
    await api('/personal-accounts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(personalForm),
    })
    notice.value = '个人账号已保存'
    resetPersonalForm()
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

function editPersonalAccount(account: PersonalAccountSummary) {
  personalForm.id = account.id
  personalForm.displayName = account.displayName
  personalForm.email = account.email
  personalForm.subscriptionStatus = account.subscriptionStatus
  personalForm.planName = account.planName
  personalForm.customEndpointEnabled = account.customEndpointEnabled
  personalForm.secret = ''
}

async function deletePersonalAccount(id: string) {
  clearFeedback()
  try {
    await api(`/personal-accounts/${id}`, { method: 'DELETE' })
    notice.value = '个人账号已删除'
    if (personalForm.id === id) {
      resetPersonalForm()
    }
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

async function saveSubAccountAssets() {
  if (!selectedSubAccountId.value) return
  clearFeedback()
  try {
    await api(`/sub-accounts/${selectedSubAccountId.value}/assets`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ assetIds: selectedAssetIds.value }),
    })
    notice.value = '子账号资产授权已更新，并会在客户端下次同步时立即生效'
    await loadDashboard()
  } catch (err) {
    error.value = parseApiError(err)
  }
}

async function saveSubscription() {
  await api('/ai/subscription', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(subscriptionForm.value),
  })
  notice.value = 'AI 全局订阅策略已更新'
  await loadDashboard()
}

async function savePlan() {
  await api('/ai/plans', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(planForm),
  })
  notice.value = 'AI 订阅方案已保存'
  await loadDashboard()
}

async function saveEnterpriseSubscription() {
  await api('/ai/enterprise-subscriptions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      ...enterpriseSubscriptionForm,
      renewAt: new Date(Date.now() + 30 * 86400_000).toISOString(),
    }),
  })
  notice.value = '企业订阅与席位已更新'
  await loadDashboard()
}

async function savePersonalSubscription() {
  await api('/ai/personal-subscriptions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      ...personalSubscriptionForm,
      renewAt: new Date(Date.now() + 30 * 86400_000).toISOString(),
    }),
  })
  notice.value = '个人订阅已更新'
  await loadDashboard()
}

async function saveAiUsagePricing() {
  await api('/ai/usage-pricing', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(aiUsagePricingForm),
  })
  notice.value = 'AI 定价表已更新'
  await loadDashboard()
}

async function saveEndpoint() {
  await api('/ai/endpoint-sync', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(endpointForm.value),
  })
  notice.value = 'AI 自定义端点同步配置已更新'
  await loadDashboard()
}

async function saveInvoiceStatus(invoiceId: string) {
  await api(`/billing/invoices/${invoiceId}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ status: invoiceStatusForm[invoiceId] }),
  })
  notice.value = '账单状态已更新'
  await loadDashboard()
}

async function savePayment(invoice: BillingInvoiceSummary) {
  await api('/billing/payments', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      invoiceId: invoice.id,
      providerKey: paymentForm.providerKey,
      amount: paymentForm.amount > 0 ? paymentForm.amount : invoice.totalAmount,
      currency: paymentForm.currency || invoice.currency,
      paymentMethod: paymentForm.paymentMethod,
      status: paymentForm.status,
      externalReference: paymentForm.externalReference,
      note: paymentForm.note,
      paidAt: new Date().toISOString(),
    }),
  })
  notice.value = '支付记录已登记'
  paymentForm.invoiceId = ''
  paymentForm.amount = 0
  paymentForm.currency = 'USD'
  paymentForm.paymentMethod = 'manual'
  paymentForm.status = 'completed'
  paymentForm.externalReference = ''
  paymentForm.note = ''
  await loadDashboard()
}

async function createCheckout(invoice: BillingInvoiceSummary) {
  const transaction = await api<PaymentTransactionSummary>('/billing/checkout-sessions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      invoiceId: invoice.id,
      providerKey: paymentProviderForm.providerKey,
      returnUrl: paymentProviderForm.successUrl || DEFAULT_CHECKOUT_RETURN_URL,
      cancelUrl: paymentProviderForm.cancelUrl || DEFAULT_CHECKOUT_CANCEL_URL,
    }),
  })
  notice.value = '支付链接已创建'
  if (transaction.checkoutUrl) {
    window.open(transaction.checkoutUrl, '_blank', 'noopener,noreferrer')
  }
  await loadDashboard()
}

async function savePaymentProvider() {
  await api('/billing/payment-providers', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      providerKey: paymentProviderForm.providerKey,
      displayName: paymentProviderForm.displayName,
      providerType: paymentProviderForm.providerType,
      webhookSecret: paymentProviderForm.webhookSecret,
      enabled: paymentProviderForm.enabled,
      metadataJson: paymentProviderForm.metadataJson,
      checkoutBaseUrl: paymentProviderForm.checkoutBaseUrl,
      webhookMode: paymentProviderForm.webhookMode,
      apiBaseUrl: paymentProviderForm.apiBaseUrl,
      secretApiKey: paymentProviderForm.secretApiKey,
      stripeApiVersion: paymentProviderForm.stripeApiVersion,
      webhookToleranceSeconds: paymentProviderForm.webhookToleranceSeconds,
      successUrl: paymentProviderForm.successUrl,
      cancelUrl: paymentProviderForm.cancelUrl,
    }),
  })
  notice.value = '支付提供方配置已更新'
  await loadDashboard()
}

async function generateCurrentBillingCycle() {
  const response = await api<GenerateBillingCycleResponse>('/billing/generate-current-cycle', {
    method: 'POST',
  })
  notice.value = `本月账单已刷新，共生成 ${response.generatedInvoices} 条订阅账单`
  await loadDashboard()
}

onMounted(() => {
  if (authToken.value && authExpiresAt.value > Date.now()) {
    void loadDashboard()
  } else if (refreshToken.value && refreshExpiresAt.value > Date.now()) {
    void refreshAdminSession().then(loadDashboard)
  } else if (authToken.value) {
    logout()
  }
})
</script>

<template>
  <div class="min-h-screen bg-[radial-gradient(circle_at_top_left,_#d9efe4,_#f7f5ef_38%,_#efe1d2_100%)] text-slate-900">
    <div v-if="!isAuthenticated" class="mx-auto flex min-h-screen max-w-3xl items-center px-6 py-12">
      <div class="w-full rounded-[32px] border border-white/70 bg-white/85 p-8 shadow-[0_30px_80px_rgba(87,69,42,0.12)] backdrop-blur">
        <p class="text-sm font-semibold uppercase tracking-[0.28em] text-emerald-700">SSH Assistant Admin</p>
        <h1 class="mt-4 text-4xl font-black tracking-tight text-slate-900">管理员登录</h1>
        <p class="mt-3 text-sm text-slate-500">默认演示账号：`admin / admin123`</p>
        <div class="mt-8 space-y-4">
          <input v-model="loginForm.username" placeholder="用户名" class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
          <input v-model="loginForm.password" type="password" placeholder="密码" class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
          <button class="w-full rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="login">
            登录后台
          </button>
          <p v-if="error" class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">{{ error }}</p>
        </div>
      </div>
    </div>

    <div v-else class="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
      <header class="rounded-[32px] border border-white/70 bg-white/80 p-8 shadow-[0_30px_80px_rgba(87,69,42,0.12)] backdrop-blur">
        <div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
          <div class="max-w-3xl">
            <p class="text-sm font-semibold uppercase tracking-[0.28em] text-emerald-700">SSH Assistant Admin</p>
            <h1 class="mt-3 text-4xl font-black tracking-tight text-slate-900 sm:text-5xl">企业账号、个人账号与 AI 订阅后台</h1>
            <p class="mt-4 text-base leading-7 text-slate-600">当前管理员：{{ adminUsername }}</p>
          </div>
          <div class="w-full max-w-xl rounded-3xl border border-emerald-100 bg-emerald-50/80 p-5">
            <div class="flex gap-3">
              <input v-model="apiBase" class="min-w-0 flex-1 rounded-2xl border border-emerald-200 bg-white px-4 py-3 text-sm outline-none ring-0" />
              <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="loadDashboard">刷新</button>
              <button class="rounded-2xl border border-slate-300 bg-white px-5 py-3 text-sm font-semibold text-slate-700" @click="logout">退出</button>
            </div>
            <p v-if="notice" class="mt-3 text-sm text-emerald-700">{{ notice }}</p>
            <p v-if="error" class="mt-3 text-sm text-rose-600">{{ error }}</p>
          </div>
        </div>
      </header>

      <section class="mt-8 grid gap-4 md:grid-cols-5">
        <article class="rounded-3xl border border-white/70 bg-white/85 p-6 shadow-[0_20px_50px_rgba(87,69,42,0.08)]"><p class="text-sm uppercase tracking-[0.18em] text-slate-500">企业</p><p class="mt-3 text-3xl font-black">{{ dashboard?.enterprises.length ?? 0 }}</p></article>
        <article class="rounded-3xl border border-white/70 bg-white/85 p-6 shadow-[0_20px_50px_rgba(87,69,42,0.08)]"><p class="text-sm uppercase tracking-[0.18em] text-slate-500">企业子账号</p><p class="mt-3 text-3xl font-black">{{ dashboard?.subAccounts.length ?? 0 }}</p></article>
        <article class="rounded-3xl border border-white/70 bg-white/85 p-6 shadow-[0_20px_50px_rgba(87,69,42,0.08)]"><p class="text-sm uppercase tracking-[0.18em] text-slate-500">个人账号</p><p class="mt-3 text-3xl font-black">{{ dashboard?.personalAccounts.length ?? 0 }}</p></article>
        <article class="rounded-3xl border border-white/70 bg-white/85 p-6 shadow-[0_20px_50px_rgba(87,69,42,0.08)]"><p class="text-sm uppercase tracking-[0.18em] text-slate-500">订阅方案</p><p class="mt-3 text-3xl font-black">{{ dashboard?.subscriptionPlans.length ?? 0 }}</p></article>
        <article class="rounded-3xl border border-white/70 bg-white/85 p-6 shadow-[0_20px_50px_rgba(87,69,42,0.08)]"><p class="text-sm uppercase tracking-[0.18em] text-slate-500">月度应收</p><p class="mt-3 text-3xl font-black">{{ money(monthlyRevenueEstimate) }}</p></article>
      </section>

      <div class="mt-8 grid gap-8 xl:grid-cols-[1.1fr_0.9fr]">
        <section class="space-y-8">
          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <h2 class="text-2xl font-black">企业账号管理</h2>
            <div class="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-[1fr_1.2fr_0.8fr_0.8fr_auto]">
              <input v-model="enterpriseForm.id" placeholder="ent-new" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="enterpriseForm.name" placeholder="企业名称" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model.number="enterpriseForm.seatCount" type="number" min="1" placeholder="购买席位数" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <select v-model="enterpriseForm.subscriptionStatus" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <option value="inactive">inactive</option>
                <option value="trialing">trialing</option>
                <option value="active">active</option>
                <option value="pastDue">pastDue</option>
                <option value="cancelled">cancelled</option>
              </select>
              <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="saveEnterprise">保存企业</button>
            </div>
            <div class="mt-5 space-y-3">
              <div v-for="enterprise in dashboard?.enterprises ?? []" :key="enterprise.id" class="rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
                <div class="flex items-start justify-between gap-4">
                  <div>
                    <p class="text-lg font-bold">{{ enterprise.name }}</p>
                    <p class="mt-1 text-sm text-slate-500">{{ enterprise.id }}</p>
                    <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-600">
                      <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ enterprise.subscriptionPlan }}</span>
                      <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ enterprise.subscriptionStatus }}</span>
                      <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ enterprise.activeSubAccounts }}/{{ enterprise.seatCount }} seats</span>
                    </div>
                  </div>
                  <div class="flex gap-2">
                    <button class="rounded-xl border border-slate-300 px-3 py-2 text-xs font-semibold text-slate-700" @click="editEnterprise(enterprise)">编辑</button>
                    <button class="rounded-xl border border-rose-200 px-3 py-2 text-xs font-semibold text-rose-600" @click="deleteEnterprise(enterprise.id)">删除</button>
                  </div>
                </div>
              </div>
            </div>
          </article>

          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <h2 class="text-2xl font-black">企业子账号与资产授权</h2>
            <div class="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-[1fr_1fr_1fr_1fr_0.9fr_auto]">
              <input v-model="subAccountForm.id" placeholder="sub-new" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <select v-model="subAccountForm.enterpriseId" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <option v-for="enterprise in dashboard?.enterprises ?? []" :key="enterprise.id" :value="enterprise.id">{{ enterprise.name }}</option>
              </select>
              <input v-model="subAccountForm.displayName" placeholder="显示名称" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="subAccountForm.email" placeholder="邮箱" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="subAccountForm.secret" placeholder="登录密钥" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <label class="flex items-center gap-3 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <input v-model="subAccountForm.enabled" type="checkbox" class="h-4 w-4 rounded border-slate-300" />
                <span>启用子账号</span>
              </label>
              <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="saveSubAccount">保存子账号</button>
            </div>

            <div class="mt-5 grid gap-5 lg:grid-cols-[0.7fr_1.3fr]">
              <div class="space-y-3">
                <div v-for="subAccount in dashboard?.subAccounts ?? []" :key="subAccount.id" class="rounded-3xl border p-4 text-left" :class="selectedSubAccountId === subAccount.id ? 'border-slate-900 bg-slate-900 text-white' : 'border-slate-200 bg-slate-50/70'">
                  <button class="w-full text-left" @click="selectedSubAccountId = subAccount.id; selectedAssetIds = [...subAccount.assetIds]">
                    <p class="font-bold">{{ subAccount.displayName }}</p>
                    <p class="mt-1 text-sm opacity-80">{{ subAccount.email }}</p>
                    <p class="mt-2 text-xs opacity-70">{{ subAccount.enterpriseId }} · {{ subAccount.enabled ? 'enabled' : 'disabled' }}</p>
                    <p class="mt-1 text-xs opacity-70">已授权 {{ subAccount.assetIds.length }} 台资产</p>
                  </button>
                  <div class="mt-3 flex gap-2">
                    <button class="rounded-xl border border-slate-300 px-3 py-2 text-xs font-semibold text-slate-700" @click="editSubAccount(subAccount)">编辑</button>
                    <button class="rounded-xl border border-rose-200 px-3 py-2 text-xs font-semibold text-rose-500" @click="deleteSubAccount(subAccount.id)">删除子账号</button>
                  </div>
                </div>
              </div>

              <div v-if="selectedSubAccount" class="rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
                <div class="flex items-center justify-between gap-4">
                  <div><p class="text-lg font-black">{{ selectedSubAccount.displayName }}</p><p class="text-sm text-slate-500">{{ selectedSubAccount.enterpriseId }}</p></div>
                  <button class="rounded-2xl bg-slate-900 px-4 py-2 text-sm font-semibold text-white" @click="saveSubAccountAssets">保存授权</button>
                </div>
                <div class="mt-5 grid gap-3">
                  <label v-for="asset in dashboard?.assets ?? []" :key="asset.id" class="flex items-start gap-3 rounded-2xl border border-slate-200 bg-white px-4 py-3">
                    <input v-model="selectedAssetIds" :value="asset.id" type="checkbox" class="mt-1 h-4 w-4 rounded border-slate-300" />
                    <div class="min-w-0 flex-1">
                      <div class="flex flex-wrap items-center gap-2"><p class="font-semibold">{{ asset.name }}</p><span class="rounded-full px-2 py-0.5 text-xs ring-1" :class="riskClass(asset.riskLevel)">{{ asset.riskLevel }}</span></div>
                      <p class="mt-1 text-sm text-slate-500">{{ asset.host }} · {{ asset.environment }}</p>
                    </div>
                  </label>
                </div>
              </div>
            </div>
          </article>

          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <div class="flex items-center justify-between gap-3">
              <div>
                <h2 class="text-2xl font-black">AI 方案目录与企业席位</h2>
                <p class="mt-1 text-sm text-slate-500">定义每人每月价格，再把方案绑定到企业席位上。</p>
              </div>
              <div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2 text-sm text-slate-600">
                当前全局策略：{{ subscriptionForm.planDisplayName }} · {{ money(subscriptionForm.pricePerSeat, subscriptionForm.currency) }}/seat
              </div>
            </div>

            <div class="mt-5 grid gap-4 xl:grid-cols-2">
              <div class="rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
                <h3 class="text-lg font-black">订阅方案</h3>
                <div class="mt-4 grid gap-3">
                  <input v-model="planForm.code" placeholder="plan-code" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                  <input v-model="planForm.displayName" placeholder="显示名称" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                  <div class="grid gap-3 md:grid-cols-3">
                    <input v-model="planForm.scope" placeholder="scope" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                    <input v-model.number="planForm.pricePerSeat" type="number" min="0" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                    <input v-model="planForm.currency" placeholder="USD" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                  </div>
                  <textarea v-model="planForm.description" rows="3" placeholder="方案说明" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                  <label class="flex items-center gap-3 rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm"><input v-model="planForm.allowCustomEndpoint" type="checkbox" class="h-4 w-4 rounded border-slate-300" /><span>允许用户自定义 AI 端点</span></label>
                  <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="savePlan">保存方案</button>
                </div>
              </div>

              <div class="rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
                <h3 class="text-lg font-black">企业订阅绑定</h3>
                <div class="mt-4 grid gap-3">
                  <select v-model="enterpriseSubscriptionForm.enterpriseId" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm">
                    <option v-for="enterprise in dashboard?.enterprises ?? []" :key="enterprise.id" :value="enterprise.id">{{ enterprise.name }}</option>
                  </select>
                  <select v-model="enterpriseSubscriptionForm.planCode" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm">
                    <option v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">{{ plan.displayName }}</option>
                  </select>
                  <div class="grid gap-3 md:grid-cols-2">
                    <select v-model="enterpriseSubscriptionForm.status" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm">
                      <option value="inactive">inactive</option>
                      <option value="trialing">trialing</option>
                      <option value="active">active</option>
                      <option value="pastDue">pastDue</option>
                      <option value="cancelled">cancelled</option>
                    </select>
                    <input v-model.number="enterpriseSubscriptionForm.seatsPurchased" type="number" min="1" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                  </div>
                  <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="saveEnterpriseSubscription">保存企业席位</button>
                </div>
              </div>
            </div>

            <div class="mt-5 grid gap-4 xl:grid-cols-2">
              <div class="rounded-3xl border border-slate-200 bg-white/80 p-5">
                <h3 class="text-lg font-black">方案目录</h3>
                <div class="mt-4 space-y-3">
                  <div v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" class="rounded-2xl border border-slate-200 bg-slate-50 p-4">
                    <div class="flex items-start justify-between gap-3">
                      <div>
                        <p class="font-bold">{{ plan.displayName }}</p>
                        <p class="mt-1 text-sm text-slate-500">{{ plan.code }} · {{ plan.scope }}</p>
                      </div>
                      <span class="rounded-full bg-white px-3 py-1 text-xs ring-1 ring-slate-200">{{ money(plan.pricePerSeat, plan.currency) }}/seat</span>
                    </div>
                    <p class="mt-3 text-sm text-slate-600">{{ plan.description }}</p>
                  </div>
                </div>
              </div>

              <div class="rounded-3xl border border-slate-200 bg-white/80 p-5">
                <h3 class="text-lg font-black">企业订阅列表</h3>
                <div class="mt-4 space-y-3">
                  <div v-for="item in dashboard?.enterpriseSubscriptions ?? []" :key="item.enterpriseId" class="rounded-2xl border border-slate-200 bg-slate-50 p-4">
                    <div class="flex items-start justify-between gap-3">
                      <div>
                        <p class="font-bold">{{ item.enterpriseId }}</p>
                        <p class="mt-1 text-sm text-slate-500">{{ item.planDisplayName }} · {{ item.status }}</p>
                      </div>
                      <span class="rounded-full bg-white px-3 py-1 text-xs ring-1 ring-slate-200">{{ item.seatsAssigned }}/{{ item.seatsPurchased }} seats</span>
                    </div>
                    <p class="mt-3 text-sm text-slate-600">{{ money(item.pricePerSeat, item.currency) }}/seat · renew {{ new Date(item.renewAt).toLocaleDateString() }}</p>
                  </div>
                </div>
              </div>
            </div>
          </article>

          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <div class="flex items-center justify-between gap-3">
              <div>
                <h2 class="text-2xl font-black">月度账单</h2>
                <p class="mt-1 text-sm text-slate-500">跟踪企业和个人账号的每月订阅应收、未收与回款状态。</p>
              </div>
              <div class="flex flex-wrap gap-3 text-sm text-slate-600">
                <button class="rounded-2xl bg-slate-900 px-4 py-2 text-sm font-semibold text-white" @click="generateCurrentBillingCycle">生成本月账单</button>
                <span class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2">账期 {{ dashboard?.billing.billingMonth ?? '-' }}</span>
                <span class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2">未收 {{ money(outstandingAmount) }}</span>
                <span class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2">Open {{ dashboard?.billing.openInvoiceCount ?? 0 }}</span>
              </div>
            </div>
            <div class="mt-5 rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
              <h3 class="text-lg font-black">支付提供方</h3>
              <div class="mt-4 grid gap-3 md:grid-cols-2">
                <input v-model="paymentProviderForm.providerKey" placeholder="provider key" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.displayName" placeholder="display name" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.providerType" placeholder="manual / stripe / alipay" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.webhookSecret" placeholder="webhook secret" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.checkoutBaseUrl" placeholder="checkout base url" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.webhookMode" placeholder="manual / stripe-like" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
              </div>
              <div v-if="paymentProviderForm.providerType === 'stripe'" class="mt-3 grid gap-3 md:grid-cols-2">
                <input v-model="paymentProviderForm.apiBaseUrl" placeholder="stripe api base url" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.secretApiKey" placeholder="stripe secret api key" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.stripeApiVersion" placeholder="stripe api version" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model.number="paymentProviderForm.webhookToleranceSeconds" type="number" min="30" step="30" placeholder="webhook tolerance seconds" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.successUrl" placeholder="success url" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="paymentProviderForm.cancelUrl" placeholder="cancel url" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
              </div>
              <p class="mt-2 text-xs text-slate-500">Metadata template: {{ paymentProviderHint }}</p>
              <label class="mt-3 flex items-center gap-3 rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm">
                <input v-model="paymentProviderForm.enabled" type="checkbox" class="h-4 w-4 rounded border-slate-300" />
                <span>启用支付提供方</span>
              </label>
              <button class="mt-3 rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="savePaymentProvider">保存支付提供方</button>
            </div>
            <div class="mt-5 space-y-3">
              <div v-for="invoice in dashboard?.billing.recentInvoices ?? []" :key="invoice.id" class="rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
                <div class="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                  <div>
                    <div class="flex flex-wrap items-center gap-2">
                      <p class="font-bold">{{ invoice.targetId }}</p>
                      <span class="rounded-full px-3 py-1 text-xs ring-1" :class="invoiceBadge(invoice.status)">{{ invoice.status }}</span>
                    </div>
                    <p class="mt-1 text-sm text-slate-500">{{ invoice.targetType }} · {{ invoice.planCode }} · {{ invoice.billingMonth }}</p>
                    <p class="mt-3 text-sm text-slate-600">{{ invoice.seatCount }} seat × {{ money(invoice.unitPrice, invoice.currency) }}</p>
                    <p class="mt-1 text-xs text-slate-500">Subscription {{ money(invoice.subscriptionAmount, invoice.currency) }} + AI Usage {{ money(invoice.aiUsageAmount, invoice.currency) }} = {{ money(invoice.totalAmount, invoice.currency) }}</p>
                    <p class="mt-1 text-xs text-slate-500">Paid {{ money(invoice.paidAmount, invoice.currency) }} · Remaining {{ money(invoice.remainingAmount, invoice.currency) }}</p>
                    <div class="mt-3 space-y-2">
                      <div v-for="lineItem in invoice.lineItems ?? []" :key="lineItem.id" class="rounded-2xl border border-slate-200 bg-white px-3 py-2 text-xs text-slate-600">
                        <div class="flex items-center justify-between gap-3">
                          <span>{{ lineItem.description }}</span>
                          <span>{{ money(lineItem.amount, lineItem.currency) }}</span>
                        </div>
                        <div class="mt-1 flex flex-wrap gap-2 text-[11px] text-slate-400">
                          <span>{{ lineItem.itemType }}</span>
                          <span>qty {{ lineItem.quantity }}</span>
                          <span>{{ money(lineItem.unitPrice, lineItem.currency) }}</span>
                          <span v-if="lineItem.totalTokens">tokens {{ lineItem.totalTokens }}</span>
                        </div>
                      </div>
                    </div>
                    <div class="mt-3 rounded-2xl border border-slate-200 bg-white px-3 py-3 text-xs text-slate-600">
                      <button class="rounded-xl border border-slate-300 px-4 py-2 text-xs font-semibold text-slate-700" @click="createCheckout(invoice)">创建支付链接</button>
                      <div class="grid gap-2 md:grid-cols-2">
                        <input v-model.number="paymentForm.amount" type="number" min="0" step="0.01" class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs" placeholder="payment amount" />
                        <input v-model="paymentForm.externalReference" class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs" placeholder="external reference" />
                        <input v-model="paymentForm.paymentMethod" class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs" placeholder="manual / stripe / bank" />
                        <input v-model="paymentForm.note" class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs" placeholder="note" />
                      </div>
                      <select v-model="paymentForm.status" class="mt-2 rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-xs">
                        <option value="pending">pending</option>
                        <option value="completed">completed</option>
                        <option value="failed">failed</option>
                        <option value="refunded">refunded</option>
                      </select>
                      <button class="mt-3 rounded-xl bg-slate-900 px-4 py-2 text-xs font-semibold text-white" @click="savePayment(invoice)">登记付款</button>
                    </div>
                    <div v-if="invoice.payments?.length" class="mt-3 space-y-2">
                      <div v-for="payment in invoice.payments" :key="payment.id" class="rounded-2xl border border-slate-200 bg-white px-3 py-2 text-xs text-slate-600">
                        <div class="flex items-center justify-between gap-3">
                          <span>{{ payment.paymentMethod }} · {{ payment.status }}</span>
                          <span>{{ money(payment.amount, payment.currency) }}</span>
                        </div>
                        <div class="mt-1 flex flex-wrap gap-2 text-[11px] text-slate-400">
                          <span>{{ payment.externalReference || 'manual-entry' }}</span>
                          <span v-if="payment.paidAt">{{ new Date(payment.paidAt).toLocaleString() }}</span>
                          <span v-if="payment.note">{{ payment.note }}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                  <div class="flex flex-col gap-3 lg:items-end">
                    <p class="text-sm text-slate-500">到期 {{ new Date(invoice.dueAt).toLocaleDateString() }}</p>
                    <div class="flex gap-2">
                      <select v-model="invoiceStatusForm[invoice.id]" class="rounded-2xl border border-slate-200 bg-white px-4 py-2 text-sm">
                        <option value="open">open</option>
                        <option value="paid">paid</option>
                        <option value="overdue">overdue</option>
                        <option value="voided">voided</option>
                      </select>
                      <button class="rounded-2xl bg-slate-900 px-4 py-2 text-sm font-semibold text-white" @click="saveInvoiceStatus(invoice.id)">更新</button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </article>

          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <div class="flex items-center justify-between gap-3">
              <div>
                <h2 class="text-2xl font-black">AI 用量</h2>
                <p class="mt-1 text-sm text-slate-500">统计平台代理 AI 调用次数、Token 消耗和活跃账号分布。</p>
              </div>
              <div class="flex flex-wrap gap-3 text-sm text-slate-600">
                <span class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2">本月请求 {{ dashboard?.aiUsage.totalRequests ?? 0 }}</span>
                <span class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2">托管请求 {{ dashboard?.aiUsage.managedRequests ?? 0 }}</span>
                <span class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2">Tokens {{ dashboard?.aiUsage.totalTokens ?? 0 }}</span>
                <span class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-2">估算费用 {{ money(dashboard?.aiUsage.estimatedCost ?? 0, dashboard?.aiUsage.currency ?? 'USD') }}</span>
              </div>
            </div>

            <div class="mt-5 rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
              <h3 class="text-lg font-black">AI 定价表</h3>
              <div class="mt-4 grid gap-3 md:grid-cols-2">
                <input v-model="aiUsagePricingForm.id" placeholder="pricing-id" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="aiUsagePricingForm.provider" placeholder="provider" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="aiUsagePricingForm.modelName" placeholder="model name" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model.number="aiUsagePricingForm.promptTokenRatePerMillion" type="number" min="0" step="0.01" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model.number="aiUsagePricingForm.completionTokenRatePerMillion" type="number" min="0" step="0.01" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
                <input v-model="aiUsagePricingForm.currency" placeholder="USD" class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm" />
              </div>
              <label class="mt-3 flex items-center gap-3 rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm">
                <input v-model="aiUsagePricingForm.isActive" type="checkbox" class="h-4 w-4 rounded border-slate-300" />
                <span>启用该价格</span>
              </label>
              <button class="mt-3 rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="saveAiUsagePricing">保存 AI 定价</button>
              <div class="mt-4 space-y-3">
                <div v-for="pricing in dashboard?.aiUsagePricing ?? []" :key="pricing.id" class="rounded-2xl border border-slate-200 bg-white px-4 py-3">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <p class="font-bold">{{ pricing.provider }} · {{ pricing.modelName }}</p>
                      <p class="mt-1 text-sm text-slate-500">Prompt {{ money(pricing.promptTokenRatePerMillion, pricing.currency) }}/1M · Completion {{ money(pricing.completionTokenRatePerMillion, pricing.currency) }}/1M</p>
                    </div>
                    <span class="rounded-full bg-slate-50 px-3 py-1 text-xs ring-1 ring-slate-200">{{ pricing.isActive ? 'active' : 'inactive' }}</span>
                  </div>
                </div>
              </div>
            </div>

            <div class="mt-5 grid gap-4 xl:grid-cols-2">
              <div class="rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
                <h3 class="text-lg font-black">Token 结构</h3>
                <div class="mt-4 space-y-3 text-sm text-slate-600">
                  <div class="flex items-center justify-between rounded-2xl border border-slate-200 bg-white px-4 py-3">
                    <span>Prompt Tokens</span>
                    <span class="font-semibold text-slate-900">{{ dashboard?.aiUsage.promptTokens ?? 0 }}</span>
                  </div>
                  <div class="flex items-center justify-between rounded-2xl border border-slate-200 bg-white px-4 py-3">
                    <span>Completion Tokens</span>
                    <span class="font-semibold text-slate-900">{{ dashboard?.aiUsage.completionTokens ?? 0 }}</span>
                  </div>
                  <div class="flex items-center justify-between rounded-2xl border border-slate-200 bg-white px-4 py-3">
                    <span>Total Tokens</span>
                    <span class="font-semibold text-slate-900">{{ dashboard?.aiUsage.totalTokens ?? 0 }}</span>
                  </div>
                  <div class="flex items-center justify-between rounded-2xl border border-slate-200 bg-white px-4 py-3">
                    <span>Estimated Cost</span>
                    <span class="font-semibold text-slate-900">{{ money(dashboard?.aiUsage.estimatedCost ?? 0, dashboard?.aiUsage.currency ?? 'USD') }}</span>
                  </div>
                </div>
              </div>

              <div class="rounded-3xl border border-slate-200 bg-slate-50/70 p-5">
                <h3 class="text-lg font-black">Top Accounts</h3>
                <div class="mt-4 space-y-3">
                  <div v-for="item in dashboard?.aiUsage.topAccounts ?? []" :key="`${item.accountMode}-${item.accountId}`" class="rounded-2xl border border-slate-200 bg-white px-4 py-3">
                    <div class="flex items-center justify-between gap-3">
                      <div>
                        <p class="font-bold">{{ item.accountId }}</p>
                        <p class="mt-1 text-sm text-slate-500">{{ item.accountMode }} · {{ item.requests }} requests</p>
                        <p class="mt-1 text-xs text-slate-400">{{ money(item.estimatedCost, item.currency) }}</p>
                      </div>
                      <span class="rounded-full bg-slate-50 px-3 py-1 text-xs ring-1 ring-slate-200">{{ item.totalTokens }} tokens</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </article>
        </section>

        <section class="space-y-8">
          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <h2 class="text-2xl font-black">个人账号管理</h2>
            <div class="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-[1fr_1fr_1fr_1fr_0.9fr_0.9fr_auto]">
              <input v-model="personalForm.id" placeholder="usr-new" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="personalForm.displayName" placeholder="显示名称" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="personalForm.email" placeholder="邮箱" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="personalForm.secret" placeholder="登录密钥" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <select v-model="personalForm.subscriptionStatus" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <option value="inactive">inactive</option>
                <option value="trialing">trialing</option>
                <option value="active">active</option>
                <option value="pastDue">pastDue</option>
                <option value="cancelled">cancelled</option>
              </select>
              <label class="flex items-center gap-3 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <input v-model="personalForm.customEndpointEnabled" type="checkbox" class="h-4 w-4 rounded border-slate-300" />
                <span>允许自定义端点</span>
              </label>
              <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="savePersonalAccount">保存个人账号</button>
            </div>

            <div class="mt-5 space-y-3">
              <div v-for="account in dashboard?.personalAccounts ?? []" :key="account.id" class="rounded-3xl border border-slate-200 bg-slate-50/70 p-4">
                <div class="flex items-start justify-between gap-4">
                  <div><p class="font-bold">{{ account.displayName }}</p><p class="mt-1 text-sm text-slate-500">{{ account.email }}</p></div>
                  <div class="flex gap-2">
                    <button class="rounded-xl border border-slate-300 px-3 py-2 text-xs font-semibold text-slate-700" @click="editPersonalAccount(account)">编辑</button>
                    <button class="rounded-xl border border-rose-200 px-3 py-2 text-xs font-semibold text-rose-500" @click="deletePersonalAccount(account.id)">删除</button>
                  </div>
                </div>
                <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-600">
                  <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ account.subscriptionStatus }}</span>
                  <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ account.planName }}</span>
                </div>
              </div>
            </div>
          </article>

          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <div class="flex items-center justify-between gap-3">
              <div>
                <h2 class="text-2xl font-black">个人订阅绑定</h2>
                <p class="mt-1 text-sm text-slate-500">为个人账号指定包月方案，并控制是否允许同步自定义端点。</p>
              </div>
              <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="savePersonalSubscription">保存个人订阅</button>
            </div>
            <div class="mt-5 grid gap-4">
              <select v-model="personalSubscriptionForm.accountId" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <option v-for="account in dashboard?.personalAccounts ?? []" :key="account.id" :value="account.id">{{ account.displayName }}</option>
              </select>
              <select v-model="personalSubscriptionForm.planCode" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <option v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">{{ plan.displayName }}</option>
              </select>
              <select v-model="personalSubscriptionForm.status" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <option value="inactive">inactive</option>
                <option value="trialing">trialing</option>
                <option value="active">active</option>
                <option value="pastDue">pastDue</option>
                <option value="cancelled">cancelled</option>
              </select>
            </div>
            <div class="mt-5 space-y-3">
              <div v-for="item in dashboard?.personalSubscriptions ?? []" :key="item.accountId" class="rounded-2xl border border-slate-200 bg-slate-50 p-4">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <p class="font-bold">{{ item.accountId }}</p>
                    <p class="mt-1 text-sm text-slate-500">{{ item.planDisplayName }} · {{ item.status }}</p>
                  </div>
                  <span class="rounded-full bg-white px-3 py-1 text-xs ring-1 ring-slate-200">{{ money(item.pricePerSeat, item.currency) }}/month</span>
                </div>
              </div>
            </div>
          </article>

          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <div class="flex items-center justify-between gap-3">
              <div>
                <h2 class="text-2xl font-black">AI 全局策略</h2>
                <p class="mt-1 text-sm text-slate-500">控制客户端默认看到的服务模式、自定义端点同步和全局策略展示。</p>
              </div>
              <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="saveSubscription">保存全局策略</button>
            </div>
            <div class="mt-5 grid gap-4">
              <select v-model="subscriptionForm.planName" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm">
                <option v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">{{ plan.displayName }}</option>
              </select>
              <input v-model.number="subscriptionForm.seats" type="number" min="1" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <label class="flex items-center gap-3 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm"><input v-model="subscriptionForm.allowCustomEndpoint" type="checkbox" class="h-4 w-4 rounded border-slate-300" /><span>允许用户自定义 AI 端点</span></label>
              <label class="flex items-center gap-3 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm"><input v-model="subscriptionForm.syncCustomEndpoint" type="checkbox" class="h-4 w-4 rounded border-slate-300" /><span>将平台端点同步给客户端</span></label>
            </div>
          </article>

          <article class="rounded-[28px] border border-white/70 bg-white/85 p-6 shadow-[0_24px_60px_rgba(87,69,42,0.08)]">
            <div class="flex items-center justify-between gap-3">
              <div><h2 class="text-2xl font-black">自定义端点同步</h2><p class="mt-1 text-sm text-slate-500">将用户自定义 AI 端点保存到系统后台并向客户端同步。</p></div>
              <button class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white" @click="saveEndpoint">保存端点</button>
            </div>
            <div class="mt-5 grid gap-4">
              <input v-model="endpointForm.endpointName" placeholder="端点名称" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="endpointForm.baseUrl" placeholder="Base URL" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="endpointForm.apiKey" placeholder="Managed API Key" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
              <input v-model="endpointForm.modelName" placeholder="Model" class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm" />
            </div>
          </article>
        </section>
      </div>
    </div>
  </div>
</template>
