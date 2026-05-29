<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'

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

type SectionId =
  | 'overview'
  | 'enterprises'
  | 'subaccounts'
  | 'ai-subscriptions'
  | 'billing'
  | 'ai-usage'
  | 'personal-accounts'
  | 'global-strategy'

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
const activeSectionId = ref<SectionId>('overview')

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
  completionTokenRatePerMillion: 0.6,
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

const statusOptions: SubscriptionStatus[] = ['inactive', 'trialing', 'active', 'pastDue', 'cancelled']
const invoiceStatusOptions: BillingInvoiceStatus[] = ['open', 'paid', 'overdue', 'voided']
const paymentStatusOptions = ['pending', 'completed', 'failed', 'refunded']

const sectionItems: Array<{ id: SectionId; label: string; description: string; icon: string }> = [
  { id: 'overview', label: '概览', description: '关键指标与运行状态', icon: 'grid' },
  { id: 'enterprises', label: '企业账号', description: '企业信息与席位概况', icon: 'building' },
  { id: 'subaccounts', label: '子账号与授权', description: '资产范围与成员权限', icon: 'users' },
  { id: 'ai-subscriptions', label: 'AI 订阅', description: '方案目录与席位绑定', icon: 'zap' },
  { id: 'billing', label: '账单中心', description: '回款、支付与账期', icon: 'card' },
  { id: 'ai-usage', label: 'AI 用量', description: '请求、Token 与成本', icon: 'chart' },
  { id: 'personal-accounts', label: '个人账号', description: '个人账号与订阅', icon: 'user' },
  { id: 'global-strategy', label: '全局策略', description: 'AI 策略与端点同步', icon: 'settings' },
]

const selectedSubAccount = computed(
  () => dashboard.value?.subAccounts.find((item) => item.id === selectedSubAccountId.value) ?? null,
)
const isAuthenticated = computed(() => Boolean(authToken.value))
const monthlyRevenueEstimate = computed(() => dashboard.value?.billing.estimatedMonthlyRevenue ?? 0)
const outstandingAmount = computed(() => dashboard.value?.billing.outstandingAmount ?? 0)
const assignedAssetCount = computed(() => selectedSubAccount.value?.assetIds.length ?? 0)
const totalSeats = computed(() => dashboard.value?.enterprises.reduce((sum, item) => sum + item.seatCount, 0) ?? 0)
const totalAssignedEnterpriseSeats = computed(
  () => dashboard.value?.enterprises.reduce((sum, item) => sum + item.activeSubAccounts, 0) ?? 0,
)
const refreshExpiryText = computed(() =>
  refreshExpiresAt.value ? new Date(refreshExpiresAt.value).toLocaleString() : '-',
)

const paymentProviderHint = computed(() => {
  if (paymentProviderForm.providerType === 'stripe') {
    return `{"checkoutBaseUrl":"https://payments.example.com/stripe-checkout","apiBaseUrl":"https://api.stripe.com","stripeApiVersion":"2024-06-20","webhookMode":"stripe-like","webhookToleranceSeconds":300,"successUrl":"${DEFAULT_CHECKOUT_RETURN_URL}","cancelUrl":"${DEFAULT_CHECKOUT_CANCEL_URL}"}`
  }
  if (paymentProviderForm.providerType === 'manual') {
    return '{"checkoutBaseUrl":"https://payments.example.com/manual-checkout","webhookMode":"manual"}'
  }
  return '{"checkoutBaseUrl":"https://payments.example.com/provider-checkout"}'
})

const summaryCards = computed(() => [
  {
    label: '企业账号',
    value: String(dashboard.value?.enterprises.length ?? 0),
    note: `${totalAssignedEnterpriseSeats.value}/${totalSeats.value} 已分配席位`,
  },
  {
    label: '企业子账号',
    value: String(dashboard.value?.subAccounts.length ?? 0),
    note: `当前选中授权 ${assignedAssetCount.value} 台资产`,
  },
  {
    label: '个人账号',
    value: String(dashboard.value?.personalAccounts.length ?? 0),
    note: `${dashboard.value?.personalSubscriptions.length ?? 0} 个有效订阅映射`,
  },
  {
    label: '订阅方案',
    value: String(dashboard.value?.subscriptionPlans.length ?? 0),
    note: `全局方案 ${subscriptionForm.value.planDisplayName || '-'}`,
  },
  {
    label: '月度应收',
    value: money(monthlyRevenueEstimate.value),
    note: `未收 ${money(outstandingAmount.value)}`,
  },
])

function parseApiError(error: unknown) {
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
  if (risk === 'critical') return 'admin-badge admin-badge-danger'
  if (risk === 'high') return 'admin-badge admin-badge-warning'
  if (risk === 'low') return 'admin-badge admin-badge-success'
  return 'admin-badge admin-badge-neutral'
}

function statusClass(status: SubscriptionStatus | string) {
  if (status === 'active') return 'admin-badge admin-badge-success'
  if (status === 'trialing' || status === 'open') return 'admin-badge admin-badge-info'
  if (status === 'pastDue' || status === 'overdue') return 'admin-badge admin-badge-warning'
  if (status === 'cancelled' || status === 'voided') return 'admin-badge admin-badge-neutral'
  return 'admin-badge admin-badge-danger'
}

function invoiceBadge(status: BillingInvoiceStatus) {
  if (status === 'paid') return 'admin-badge admin-badge-success'
  if (status === 'overdue') return 'admin-badge admin-badge-danger'
  if (status === 'voided') return 'admin-badge admin-badge-neutral'
  return 'admin-badge admin-badge-warning'
}

function money(value: number, currency = 'USD') {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
    maximumFractionDigits: 0,
  }).format(value)
}

function formatDate(value?: string | null, includeTime = false) {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return includeTime ? date.toLocaleString() : date.toLocaleDateString()
}

function scrollToSection(sectionId: SectionId) {
  activeSectionId.value = sectionId
  const element = document.getElementById(sectionId)
  element?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

function updateActiveSection() {
  const sections = Array.from(document.querySelectorAll<HTMLElement>('[data-section]'))
  if (sections.length === 0) return

  const scrollTop = window.scrollY + 180
  let current = sections[0].id as SectionId

  sections.forEach((section) => {
    if (section.offsetTop <= scrollTop) {
      current = section.id as SectionId
    }
  })

  activeSectionId.value = current
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
      paymentProviderForm.stripeApiVersion =
        provider.stripeApiVersion || paymentProviderForm.stripeApiVersion
      paymentProviderForm.webhookToleranceSeconds =
        provider.webhookToleranceSeconds || paymentProviderForm.webhookToleranceSeconds
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
    requestAnimationFrame(updateActiveSection)
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
    if (enterpriseForm.id === id) resetEnterpriseForm()
    if (subAccountForm.enterpriseId === id) resetSubAccountForm()
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
    if (subAccountForm.id === id) resetSubAccountForm()
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
    if (personalForm.id === id) resetPersonalForm()
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
  window.addEventListener('scroll', updateActiveSection, { passive: true })
  if (authToken.value && authExpiresAt.value > Date.now()) {
    void loadDashboard()
  } else if (refreshToken.value && refreshExpiresAt.value > Date.now()) {
    void refreshAdminSession().then(loadDashboard)
  } else if (authToken.value) {
    logout()
  }
  requestAnimationFrame(updateActiveSection)
})

onBeforeUnmount(() => {
  window.removeEventListener('scroll', updateActiveSection)
})
</script>

<template>
  <div class="admin-shell">
    <div v-if="!isAuthenticated" class="admin-login-shell">
      <div class="admin-login-panel">
        <div class="space-y-6">
          <div class="admin-eyebrow">SSH Assistant Admin</div>
          <div>
            <h1 class="admin-login-title">企业后台登录</h1>
            <p class="admin-login-subtitle">
              统一管理企业账号、个人账号、订阅策略、账单与 AI 资源分配。
            </p>
          </div>
        </div>

        <div class="admin-login-grid">
          <div class="admin-card admin-card-muted">
            <div class="admin-card-head">
              <div>
                <h2 class="admin-subtitle">登录凭据</h2>
                <p class="admin-muted">默认演示账号：admin / admin123</p>
              </div>
            </div>
            <div class="admin-form-grid">
              <label class="admin-field">
                <span class="admin-label">用户名</span>
                <input v-model="loginForm.username" placeholder="请输入管理员用户名" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">密码</span>
                <input
                  v-model="loginForm.password"
                  type="password"
                  placeholder="请输入管理员密码"
                  class="admin-input"
                />
              </label>
              <button class="admin-button-primary w-full" @click="login">登录后台</button>
            </div>
            <p v-if="error" class="admin-alert admin-alert-danger">{{ error }}</p>
          </div>

          <div class="admin-card">
            <div class="admin-card-head">
              <div>
                <h2 class="admin-subtitle">本页能力</h2>
                <p class="admin-muted">一站式完成资产、订阅、账单与 AI 策略管理。</p>
              </div>
            </div>
            <div class="grid gap-3 md:grid-cols-2">
              <div class="admin-stat-tile">
                <p class="admin-stat-label">企业账号</p>
                <p class="admin-stat-note">席位、状态与子账号总览</p>
              </div>
              <div class="admin-stat-tile">
                <p class="admin-stat-label">资产授权</p>
                <p class="admin-stat-note">子账号与资产范围同步</p>
              </div>
              <div class="admin-stat-tile">
                <p class="admin-stat-label">账单回款</p>
                <p class="admin-stat-note">支付提供方、回款与状态管理</p>
              </div>
              <div class="admin-stat-tile">
                <p class="admin-stat-label">AI 策略</p>
                <p class="admin-stat-note">方案目录、Token 价格与端点同步</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="admin-app">
      <aside class="admin-sidebar">
        <div class="admin-sidebar-brand">
          <div class="admin-brand-mark">SA</div>
          <div>
            <p class="admin-sidebar-title">SSH Assistant</p>
            <p class="admin-sidebar-subtitle">Enterprise Console</p>
          </div>
        </div>

        <nav class="admin-sidebar-nav">
          <button
            v-for="section in sectionItems"
            :key="section.id"
            type="button"
            class="admin-nav-item"
            :class="{ 'is-active': activeSectionId === section.id }"
            @click="scrollToSection(section.id)"
          >
            <div class="admin-nav-row">
              <span class="admin-nav-icon" :data-icon="section.icon">
                <svg v-if="section.icon === 'grid'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>
                <svg v-else-if="section.icon === 'building'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="2" width="16" height="20" rx="2"/><path d="M9 22v-4h6v4"/><path d="M8 6h.01M16 6h.01M8 10h.01M16 10h.01M8 14h.01M16 14h.01"/></svg>
                <svg v-else-if="section.icon === 'users'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
                <svg v-else-if="section.icon === 'zap'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>
                <svg v-else-if="section.icon === 'card'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="5" width="20" height="14" rx="2"/><line x1="2" y1="10" x2="22" y2="10"/></svg>
                <svg v-else-if="section.icon === 'chart'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/></svg>
                <svg v-else-if="section.icon === 'user'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
                <svg v-else-if="section.icon === 'settings'" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/></svg>
              </span>
              <span class="admin-nav-label">{{ section.label }}</span>
            </div>
            <span class="admin-nav-desc">{{ section.description }}</span>
          </button>
        </nav>

        <div class="admin-sidebar-footer">
          <p class="admin-sidebar-caption">当前管理员</p>
          <p class="admin-sidebar-user">{{ adminUsername }}</p>
          <p class="admin-sidebar-meta">刷新令牌有效期至 {{ refreshExpiryText }}</p>
        </div>
      </aside>

      <main class="admin-main">
        <header class="admin-topbar">
          <div>
            <div class="admin-eyebrow">Enterprise Admin Workspace</div>
            <h1 class="admin-page-title">企业账号、订阅与账单管理后台</h1>
            <p class="admin-page-subtitle">
              通过统一后台管理企业账号、个人账号、AI 订阅、账单回款和平台端点策略。
            </p>
          </div>

          <div class="admin-topbar-actions">
            <label class="admin-inline-field admin-inline-field-wide">
              <span class="admin-inline-label">后台 API</span>
              <input v-model="apiBase" class="admin-input" />
            </label>
            <button class="admin-button-primary" @click="loadDashboard">刷新数据</button>
            <button class="admin-button-secondary" @click="logout">退出登录</button>
          </div>
        </header>

        <div class="admin-mobile-nav">
          <button
            v-for="section in sectionItems"
            :key="section.id"
            type="button"
            class="admin-mobile-chip"
            :class="{ 'is-active': activeSectionId === section.id }"
            @click="scrollToSection(section.id)"
          >
            {{ section.label }}
          </button>
        </div>

        <section id="overview" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">概览</h2>
              <p class="admin-section-desc">查看当前后台的核心指标、登录态与系统提示。</p>
            </div>
          </div>

          <div class="admin-feedback-stack">
            <p v-if="notice" class="admin-alert admin-alert-success">{{ notice }}</p>
            <p v-if="error" class="admin-alert admin-alert-danger">{{ error }}</p>
            <p v-if="loading" class="admin-alert admin-alert-info">正在同步后台数据，请稍候…</p>
          </div>

          <div class="admin-summary-grid">
            <article v-for="card in summaryCards" :key="card.label" class="admin-summary-card">
              <p class="admin-summary-label">{{ card.label }}</p>
              <p class="admin-summary-value">{{ card.value }}</p>
              <p class="admin-summary-note">{{ card.note }}</p>
            </article>
          </div>
        </section>

        <section id="enterprises" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">企业账号管理</h2>
              <p class="admin-section-desc">维护企业名称、席位规模和订阅状态。</p>
            </div>
            <div class="admin-pill-row">
              <span class="admin-pill">企业数 {{ dashboard?.enterprises.length ?? 0 }}</span>
              <span class="admin-pill">总席位 {{ totalSeats }}</span>
            </div>
          </div>

          <div class="admin-card">
            <div class="admin-card-head">
              <div>
                <h3 class="admin-subtitle">企业信息表单</h3>
                <p class="admin-muted">保存后会同步企业订阅状态与席位容量。</p>
              </div>
            </div>
            <div class="admin-form-grid admin-form-grid-5">
              <label class="admin-field">
                <span class="admin-label">企业 ID</span>
                <input v-model="enterpriseForm.id" placeholder="ent-new" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">企业名称</span>
                <input v-model="enterpriseForm.name" placeholder="企业名称" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">购买席位</span>
                <input
                  v-model.number="enterpriseForm.seatCount"
                  type="number"
                  min="1"
                  placeholder="购买席位数"
                  class="admin-input"
                />
              </label>
              <label class="admin-field">
                <span class="admin-label">订阅状态</span>
                <select v-model="enterpriseForm.subscriptionStatus" class="admin-input">
                  <option v-for="status in statusOptions" :key="status" :value="status">{{ status }}</option>
                </select>
              </label>
              <div class="admin-field admin-actions-end">
                <span class="admin-label">操作</span>
                <button class="admin-button-primary w-full" @click="saveEnterprise">保存企业</button>
              </div>
            </div>
          </div>

          <div class="admin-list-grid">
            <article v-for="enterprise in dashboard?.enterprises ?? []" :key="enterprise.id" class="admin-entity-card">
              <div class="admin-entity-head">
                <div>
                  <h3 class="admin-entity-title">{{ enterprise.name }}</h3>
                  <p class="admin-entity-meta">{{ enterprise.id }}</p>
                </div>
                <span :class="statusClass(enterprise.subscriptionStatus)">{{ enterprise.subscriptionStatus }}</span>
              </div>
              <div class="admin-chip-row">
                <span class="admin-chip">{{ enterprise.subscriptionPlan }}</span>
                <span class="admin-chip">{{ enterprise.activeSubAccounts }}/{{ enterprise.seatCount }} seats</span>
                <span class="admin-chip">续期 {{ formatDate(enterprise.renewAt) }}</span>
              </div>
              <div class="admin-entity-actions">
                <button class="admin-button-secondary" @click="editEnterprise(enterprise)">编辑</button>
                <button class="admin-button-danger" @click="deleteEnterprise(enterprise.id)">删除</button>
              </div>
            </article>
          </div>
        </section>

        <section id="subaccounts" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">企业子账号与资产授权</h2>
              <p class="admin-section-desc">为企业成员分配登录身份，并限定其可同步的资产范围。</p>
            </div>
          </div>

          <div class="admin-card">
            <div class="admin-card-head">
              <div>
                <h3 class="admin-subtitle">子账号表单</h3>
                <p class="admin-muted">保存时会保留当前授权资产映射。</p>
              </div>
            </div>
            <div class="admin-form-grid admin-form-grid-6">
              <label class="admin-field">
                <span class="admin-label">子账号 ID</span>
                <input v-model="subAccountForm.id" placeholder="sub-new" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">所属企业</span>
                <select v-model="subAccountForm.enterpriseId" class="admin-input">
                  <option v-for="enterprise in dashboard?.enterprises ?? []" :key="enterprise.id" :value="enterprise.id">
                    {{ enterprise.name }}
                  </option>
                </select>
              </label>
              <label class="admin-field">
                <span class="admin-label">显示名称</span>
                <input v-model="subAccountForm.displayName" placeholder="显示名称" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">邮箱</span>
                <input v-model="subAccountForm.email" placeholder="邮箱" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">登录密钥</span>
                <input v-model="subAccountForm.secret" placeholder="登录密钥" class="admin-input" />
              </label>
              <label class="admin-check-field">
                <span class="admin-label">状态</span>
                <span class="admin-check-wrap">
                  <input v-model="subAccountForm.enabled" type="checkbox" class="admin-checkbox" />
                  <span>启用子账号</span>
                </span>
              </label>
              <div class="admin-field admin-actions-end">
                <span class="admin-label">操作</span>
                <button class="admin-button-primary w-full" @click="saveSubAccount">保存子账号</button>
              </div>
            </div>
          </div>

          <div class="admin-two-column">
            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">子账号列表</h3>
                  <p class="admin-muted">点击列表项切换当前授权视图。</p>
                </div>
              </div>
              <div class="admin-list-stack">
                <button
                  v-for="subAccount in dashboard?.subAccounts ?? []"
                  :key="subAccount.id"
                  type="button"
                  class="admin-select-card"
                  :class="{ 'is-active': selectedSubAccountId === subAccount.id }"
                  @click="
                    selectedSubAccountId = subAccount.id;
                    selectedAssetIds = [...subAccount.assetIds]
                  "
                >
                  <div class="admin-entity-head">
                    <div>
                      <h4 class="admin-entity-title">{{ subAccount.displayName }}</h4>
                      <p class="admin-entity-meta">{{ subAccount.email }}</p>
                    </div>
                    <span :class="subAccount.enabled ? 'admin-badge admin-badge-success' : 'admin-badge admin-badge-neutral'">
                      {{ subAccount.enabled ? 'enabled' : 'disabled' }}
                    </span>
                  </div>
                  <p class="admin-entity-meta">{{ subAccount.enterpriseId }} · 已授权 {{ subAccount.assetIds.length }} 台资产</p>
                  <div class="admin-entity-actions">
                    <span class="admin-link-button" @click.stop="editSubAccount(subAccount)">编辑</span>
                    <span class="admin-link-button admin-link-button-danger" @click.stop="deleteSubAccount(subAccount.id)">
                      删除
                    </span>
                  </div>
                </button>
              </div>
            </article>

            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">资产授权明细</h3>
                  <p class="admin-muted">
                    {{ selectedSubAccount ? `${selectedSubAccount.displayName} 当前可访问的资产列表` : '请选择左侧子账号后配置授权。' }}
                  </p>
                </div>
                <button
                  class="admin-button-primary"
                  :disabled="!selectedSubAccount"
                  @click="saveSubAccountAssets"
                >
                  保存授权
                </button>
              </div>

              <div v-if="selectedSubAccount" class="admin-list-stack">
                <label v-for="asset in dashboard?.assets ?? []" :key="asset.id" class="admin-asset-row">
                  <span class="admin-check-wrap">
                    <input v-model="selectedAssetIds" :value="asset.id" type="checkbox" class="admin-checkbox" />
                    <span class="sr-only">{{ asset.name }}</span>
                  </span>
                  <div class="min-w-0 flex-1">
                    <div class="admin-entity-head">
                      <div>
                        <h4 class="admin-entity-title">{{ asset.name }}</h4>
                        <p class="admin-entity-meta">{{ asset.host }} · {{ asset.environment }}</p>
                      </div>
                      <span :class="riskClass(asset.riskLevel)">{{ asset.riskLevel }}</span>
                    </div>
                    <div class="admin-chip-row">
                      <span class="admin-chip">{{ asset.ownerType }}</span>
                      <span class="admin-chip">{{ asset.id }}</span>
                    </div>
                  </div>
                </label>
              </div>

              <div v-else class="admin-empty-state">从左侧选择一个子账号后再分配资产授权。</div>
            </article>
          </div>
        </section>

        <section id="ai-subscriptions" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">AI 订阅与席位配置</h2>
              <p class="admin-section-desc">维护平台方案目录，并将方案映射到企业席位。</p>
            </div>
            <div class="admin-pill-row">
              <span class="admin-pill">
                当前全局策略 {{ subscriptionForm.planDisplayName }} ·
                {{ money(subscriptionForm.pricePerSeat, subscriptionForm.currency) }}/seat
              </span>
            </div>
          </div>

          <div class="admin-two-column">
            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">订阅方案</h3>
                  <p class="admin-muted">定义按人按月的方案目录与端点权限策略。</p>
                </div>
              </div>
              <div class="admin-form-grid">
                <label class="admin-field">
                  <span class="admin-label">方案编码</span>
                  <input v-model="planForm.code" placeholder="plan-code" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">显示名称</span>
                  <input v-model="planForm.displayName" placeholder="显示名称" class="admin-input" />
                </label>
                <div class="admin-form-grid admin-form-grid-3">
                  <label class="admin-field">
                    <span class="admin-label">作用域</span>
                    <input v-model="planForm.scope" placeholder="enterprise / personal" class="admin-input" />
                  </label>
                  <label class="admin-field">
                    <span class="admin-label">单席位价格</span>
                    <input v-model.number="planForm.pricePerSeat" type="number" min="0" class="admin-input" />
                  </label>
                  <label class="admin-field">
                    <span class="admin-label">货币</span>
                    <input v-model="planForm.currency" placeholder="USD" class="admin-input" />
                  </label>
                </div>
                <label class="admin-field">
                  <span class="admin-label">方案说明</span>
                  <textarea v-model="planForm.description" rows="3" class="admin-input admin-textarea" />
                </label>
                <label class="admin-check-field">
                  <span class="admin-label">扩展策略</span>
                  <span class="admin-check-wrap">
                    <input v-model="planForm.allowCustomEndpoint" type="checkbox" class="admin-checkbox" />
                    <span>允许用户自定义 AI 端点</span>
                  </span>
                </label>
                <button class="admin-button-primary w-full" @click="savePlan">保存方案</button>
              </div>
            </article>

            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">企业席位绑定</h3>
                  <p class="admin-muted">将企业与方案绑定，并维护购买席位数量。</p>
                </div>
              </div>
              <div class="admin-form-grid">
                <label class="admin-field">
                  <span class="admin-label">企业</span>
                  <select v-model="enterpriseSubscriptionForm.enterpriseId" class="admin-input">
                    <option v-for="enterprise in dashboard?.enterprises ?? []" :key="enterprise.id" :value="enterprise.id">
                      {{ enterprise.name }}
                    </option>
                  </select>
                </label>
                <label class="admin-field">
                  <span class="admin-label">订阅方案</span>
                  <select v-model="enterpriseSubscriptionForm.planCode" class="admin-input">
                    <option v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">
                      {{ plan.displayName }}
                    </option>
                  </select>
                </label>
                <div class="admin-form-grid admin-form-grid-2">
                  <label class="admin-field">
                    <span class="admin-label">状态</span>
                    <select v-model="enterpriseSubscriptionForm.status" class="admin-input">
                      <option v-for="status in statusOptions" :key="status" :value="status">{{ status }}</option>
                    </select>
                  </label>
                  <label class="admin-field">
                    <span class="admin-label">购买席位</span>
                    <input
                      v-model.number="enterpriseSubscriptionForm.seatsPurchased"
                      type="number"
                      min="1"
                      class="admin-input"
                    />
                  </label>
                </div>
                <button class="admin-button-primary w-full" @click="saveEnterpriseSubscription">保存企业席位</button>
              </div>
            </article>
          </div>

          <div class="admin-two-column">
            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">方案目录</h3>
                  <p class="admin-muted">当前所有可用的 AI 订阅方案。</p>
                </div>
              </div>
              <div class="admin-list-stack">
                <article v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" class="admin-entity-card">
                  <div class="admin-entity-head">
                    <div>
                      <h4 class="admin-entity-title">{{ plan.displayName }}</h4>
                      <p class="admin-entity-meta">{{ plan.code }} · {{ plan.scope }}</p>
                    </div>
                    <span :class="plan.isActive ? 'admin-badge admin-badge-success' : 'admin-badge admin-badge-neutral'">
                      {{ plan.isActive ? 'active' : 'inactive' }}
                    </span>
                  </div>
                  <div class="admin-chip-row">
                    <span class="admin-chip">{{ money(plan.pricePerSeat, plan.currency) }}/seat</span>
                    <span class="admin-chip">{{ plan.allowCustomEndpoint ? '支持自定义端点' : '托管端点' }}</span>
                  </div>
                  <p class="admin-description">{{ plan.description || '未填写方案说明。' }}</p>
                </article>
              </div>
            </article>

            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">企业订阅列表</h3>
                  <p class="admin-muted">查看企业当前方案、席位使用与续期信息。</p>
                </div>
              </div>
              <div class="admin-list-stack">
                <article
                  v-for="item in dashboard?.enterpriseSubscriptions ?? []"
                  :key="item.enterpriseId"
                  class="admin-entity-card"
                >
                  <div class="admin-entity-head">
                    <div>
                      <h4 class="admin-entity-title">{{ item.enterpriseId }}</h4>
                      <p class="admin-entity-meta">{{ item.planDisplayName }}</p>
                    </div>
                    <span :class="statusClass(item.status)">{{ item.status }}</span>
                  </div>
                  <div class="admin-chip-row">
                    <span class="admin-chip">{{ item.seatsAssigned }}/{{ item.seatsPurchased }} seats</span>
                    <span class="admin-chip">{{ money(item.pricePerSeat, item.currency) }}/seat</span>
                    <span class="admin-chip">续期 {{ formatDate(item.renewAt) }}</span>
                  </div>
                </article>
              </div>
            </article>
          </div>
        </section>

        <section id="billing" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">账单中心</h2>
              <p class="admin-section-desc">跟踪账期、支付提供方、支付记录与应收状态。</p>
            </div>
            <div class="admin-pill-row">
              <span class="admin-pill">账期 {{ dashboard?.billing.billingMonth ?? '-' }}</span>
              <span class="admin-pill">未收 {{ money(outstandingAmount) }}</span>
              <span class="admin-pill">Open {{ dashboard?.billing.openInvoiceCount ?? 0 }}</span>
            </div>
          </div>

          <div class="admin-card">
            <div class="admin-card-head">
              <div>
                <h3 class="admin-subtitle">支付提供方</h3>
                <p class="admin-muted">配置对账方式、Checkout URL 与 Stripe 风格回调参数。</p>
              </div>
              <button class="admin-button-primary" @click="generateCurrentBillingCycle">生成本月账单</button>
            </div>
            <div class="admin-form-grid admin-form-grid-3">
              <label class="admin-field">
                <span class="admin-label">Provider Key</span>
                <input v-model="paymentProviderForm.providerKey" placeholder="provider key" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">显示名称</span>
                <input v-model="paymentProviderForm.displayName" placeholder="display name" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">Provider Type</span>
                <input
                  v-model="paymentProviderForm.providerType"
                  placeholder="manual / stripe / alipay"
                  class="admin-input"
                />
              </label>
              <label class="admin-field">
                <span class="admin-label">Webhook Secret</span>
                <input v-model="paymentProviderForm.webhookSecret" placeholder="webhook secret" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">Checkout Base URL</span>
                <input v-model="paymentProviderForm.checkoutBaseUrl" placeholder="checkout base url" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">Webhook Mode</span>
                <input v-model="paymentProviderForm.webhookMode" placeholder="manual / stripe-like" class="admin-input" />
              </label>
            </div>

            <div v-if="paymentProviderForm.providerType === 'stripe'" class="admin-form-grid admin-form-grid-3 mt-4">
              <label class="admin-field">
                <span class="admin-label">Stripe API Base URL</span>
                <input v-model="paymentProviderForm.apiBaseUrl" placeholder="stripe api base url" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">Secret API Key</span>
                <input v-model="paymentProviderForm.secretApiKey" placeholder="stripe secret api key" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">API Version</span>
                <input v-model="paymentProviderForm.stripeApiVersion" placeholder="stripe api version" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">Webhook Tolerance</span>
                <input
                  v-model.number="paymentProviderForm.webhookToleranceSeconds"
                  type="number"
                  min="30"
                  step="30"
                  class="admin-input"
                />
              </label>
              <label class="admin-field">
                <span class="admin-label">Success URL</span>
                <input v-model="paymentProviderForm.successUrl" placeholder="success url" class="admin-input" />
              </label>
              <label class="admin-field">
                <span class="admin-label">Cancel URL</span>
                <input v-model="paymentProviderForm.cancelUrl" placeholder="cancel url" class="admin-input" />
              </label>
            </div>

            <div class="admin-card-toolbar">
              <label class="admin-check-field">
                <span class="admin-label">启用状态</span>
                <span class="admin-check-wrap">
                  <input v-model="paymentProviderForm.enabled" type="checkbox" class="admin-checkbox" />
                  <span>启用支付提供方</span>
                </span>
              </label>
              <button class="admin-button-primary" @click="savePaymentProvider">保存支付提供方</button>
            </div>
            <p class="admin-hint">Metadata template: {{ paymentProviderHint }}</p>
          </div>

          <div class="admin-list-stack">
            <article v-for="invoice in dashboard?.billing.recentInvoices ?? []" :key="invoice.id" class="admin-card">
              <div class="admin-card-head">
                <div>
                  <div class="admin-entity-head">
                    <div>
                      <h3 class="admin-subtitle">{{ invoice.targetId }}</h3>
                      <p class="admin-muted">{{ invoice.targetType }} · {{ invoice.planCode }} · {{ invoice.billingMonth }}</p>
                    </div>
                    <span :class="invoiceBadge(invoice.status)">{{ invoice.status }}</span>
                  </div>
                  <div class="admin-chip-row">
                    <span class="admin-chip">{{ invoice.seatCount }} seat × {{ money(invoice.unitPrice, invoice.currency) }}</span>
                    <span class="admin-chip">订阅 {{ money(invoice.subscriptionAmount, invoice.currency) }}</span>
                    <span class="admin-chip">AI 用量 {{ money(invoice.aiUsageAmount, invoice.currency) }}</span>
                    <span class="admin-chip">到期 {{ formatDate(invoice.dueAt) }}</span>
                  </div>
                </div>
                <div class="admin-inline-actions">
                  <select v-model="invoiceStatusForm[invoice.id]" class="admin-input admin-input-compact">
                    <option v-for="status in invoiceStatusOptions" :key="status" :value="status">{{ status }}</option>
                  </select>
                  <button class="admin-button-secondary" @click="saveInvoiceStatus(invoice.id)">更新状态</button>
                </div>
              </div>

              <div class="admin-billing-grid">
                <div class="admin-card admin-card-muted">
                  <div class="admin-card-head">
                    <div>
                      <h4 class="admin-subtitle">账单明细</h4>
                      <p class="admin-muted">
                        实收 {{ money(invoice.paidAmount, invoice.currency) }} · 剩余
                        {{ money(invoice.remainingAmount, invoice.currency) }}
                      </p>
                    </div>
                  </div>
                  <div class="admin-list-stack">
                    <div v-for="lineItem in invoice.lineItems ?? []" :key="lineItem.id" class="admin-line-item">
                      <div class="admin-entity-head">
                        <span class="font-medium text-slate-900">{{ lineItem.description }}</span>
                        <span class="text-sm font-semibold text-slate-900">{{ money(lineItem.amount, lineItem.currency) }}</span>
                      </div>
                      <div class="admin-chip-row">
                        <span class="admin-chip">{{ lineItem.itemType }}</span>
                        <span class="admin-chip">qty {{ lineItem.quantity }}</span>
                        <span class="admin-chip">{{ money(lineItem.unitPrice, lineItem.currency) }}</span>
                        <span v-if="lineItem.totalTokens" class="admin-chip">tokens {{ lineItem.totalTokens }}</span>
                      </div>
                    </div>
                  </div>
                </div>

                <div class="admin-card admin-card-muted">
                  <div class="admin-card-head">
                    <div>
                      <h4 class="admin-subtitle">支付登记</h4>
                      <p class="admin-muted">创建支付链接或手动登记线下回款。</p>
                    </div>
                  </div>
                  <button class="admin-button-secondary w-full" @click="createCheckout(invoice)">创建支付链接</button>
                  <div class="admin-form-grid admin-form-grid-2 mt-4">
                    <label class="admin-field">
                      <span class="admin-label">支付金额</span>
                      <input
                        v-model.number="paymentForm.amount"
                        type="number"
                        min="0"
                        step="0.01"
                        placeholder="payment amount"
                        class="admin-input"
                      />
                    </label>
                    <label class="admin-field">
                      <span class="admin-label">外部流水</span>
                      <input
                        v-model="paymentForm.externalReference"
                        placeholder="external reference"
                        class="admin-input"
                      />
                    </label>
                    <label class="admin-field">
                      <span class="admin-label">支付方式</span>
                      <input
                        v-model="paymentForm.paymentMethod"
                        placeholder="manual / stripe / bank"
                        class="admin-input"
                      />
                    </label>
                    <label class="admin-field">
                      <span class="admin-label">备注</span>
                      <input v-model="paymentForm.note" placeholder="note" class="admin-input" />
                    </label>
                  </div>
                  <label class="admin-field mt-4">
                    <span class="admin-label">支付状态</span>
                    <select v-model="paymentForm.status" class="admin-input">
                      <option v-for="status in paymentStatusOptions" :key="status" :value="status">{{ status }}</option>
                    </select>
                  </label>
                  <button class="admin-button-primary mt-4 w-full" @click="savePayment(invoice)">登记付款</button>
                </div>
              </div>

              <div v-if="invoice.payments?.length" class="admin-card admin-card-muted mt-4">
                <div class="admin-card-head">
                  <div>
                    <h4 class="admin-subtitle">已登记支付记录</h4>
                    <p class="admin-muted">用于查看回款流水、支付方式和备注信息。</p>
                  </div>
                </div>
                <div class="admin-list-stack">
                  <div v-for="payment in invoice.payments" :key="payment.id" class="admin-line-item">
                    <div class="admin-entity-head">
                      <span class="font-medium text-slate-900">{{ payment.paymentMethod }} · {{ payment.status }}</span>
                      <span class="text-sm font-semibold text-slate-900">{{ money(payment.amount, payment.currency) }}</span>
                    </div>
                    <div class="admin-chip-row">
                      <span class="admin-chip">{{ payment.externalReference || 'manual-entry' }}</span>
                      <span v-if="payment.paidAt" class="admin-chip">{{ formatDate(payment.paidAt, true) }}</span>
                      <span v-if="payment.note" class="admin-chip">{{ payment.note }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </article>
          </div>
        </section>

        <section id="ai-usage" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">AI 用量与成本</h2>
              <p class="admin-section-desc">查看请求量、Token 结构、定价表与高消耗账号。</p>
            </div>
            <div class="admin-pill-row">
              <span class="admin-pill">本月请求 {{ dashboard?.aiUsage.totalRequests ?? 0 }}</span>
              <span class="admin-pill">托管请求 {{ dashboard?.aiUsage.managedRequests ?? 0 }}</span>
              <span class="admin-pill">Tokens {{ dashboard?.aiUsage.totalTokens ?? 0 }}</span>
              <span class="admin-pill">
                估算 {{ money(dashboard?.aiUsage.estimatedCost ?? 0, dashboard?.aiUsage.currency ?? 'USD') }}
              </span>
            </div>
          </div>

          <div class="admin-two-column">
            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">AI 定价表</h3>
                  <p class="admin-muted">维护提供方、模型和百万 Token 计费标准。</p>
                </div>
              </div>
              <div class="admin-form-grid admin-form-grid-2">
                <label class="admin-field">
                  <span class="admin-label">定价 ID</span>
                  <input v-model="aiUsagePricingForm.id" placeholder="pricing-id" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">提供方</span>
                  <input v-model="aiUsagePricingForm.provider" placeholder="provider" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">模型名</span>
                  <input v-model="aiUsagePricingForm.modelName" placeholder="model name" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">货币</span>
                  <input v-model="aiUsagePricingForm.currency" placeholder="USD" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">Prompt / 1M</span>
                  <input
                    v-model.number="aiUsagePricingForm.promptTokenRatePerMillion"
                    type="number"
                    min="0"
                    step="0.01"
                    class="admin-input"
                  />
                </label>
                <label class="admin-field">
                  <span class="admin-label">Completion / 1M</span>
                  <input
                    v-model.number="aiUsagePricingForm.completionTokenRatePerMillion"
                    type="number"
                    min="0"
                    step="0.01"
                    class="admin-input"
                  />
                </label>
              </div>
              <div class="admin-card-toolbar">
                <label class="admin-check-field">
                  <span class="admin-label">启用状态</span>
                  <span class="admin-check-wrap">
                    <input v-model="aiUsagePricingForm.isActive" type="checkbox" class="admin-checkbox" />
                    <span>启用该价格</span>
                  </span>
                </label>
                <button class="admin-button-primary" @click="saveAiUsagePricing">保存 AI 定价</button>
              </div>
              <div class="admin-list-stack mt-4">
                <article v-for="pricing in dashboard?.aiUsagePricing ?? []" :key="pricing.id" class="admin-entity-card">
                  <div class="admin-entity-head">
                    <div>
                      <h4 class="admin-entity-title">{{ pricing.provider }} · {{ pricing.modelName }}</h4>
                      <p class="admin-entity-meta">
                        Prompt {{ money(pricing.promptTokenRatePerMillion, pricing.currency) }}/1M · Completion
                        {{ money(pricing.completionTokenRatePerMillion, pricing.currency) }}/1M
                      </p>
                    </div>
                    <span :class="pricing.isActive ? 'admin-badge admin-badge-success' : 'admin-badge admin-badge-neutral'">
                      {{ pricing.isActive ? 'active' : 'inactive' }}
                    </span>
                  </div>
                </article>
              </div>
            </article>

            <div class="admin-stack">
              <article class="admin-card">
                <div class="admin-card-head">
                  <div>
                    <h3 class="admin-subtitle">Token 结构</h3>
                    <p class="admin-muted">按 Prompt、Completion 和总消耗观察本月趋势。</p>
                  </div>
                </div>
                <div class="admin-list-stack">
                  <div class="admin-kv-row">
                    <span>Prompt Tokens</span>
                    <strong>{{ dashboard?.aiUsage.promptTokens ?? 0 }}</strong>
                  </div>
                  <div class="admin-kv-row">
                    <span>Completion Tokens</span>
                    <strong>{{ dashboard?.aiUsage.completionTokens ?? 0 }}</strong>
                  </div>
                  <div class="admin-kv-row">
                    <span>Total Tokens</span>
                    <strong>{{ dashboard?.aiUsage.totalTokens ?? 0 }}</strong>
                  </div>
                  <div class="admin-kv-row">
                    <span>Estimated Cost</span>
                    <strong>{{ money(dashboard?.aiUsage.estimatedCost ?? 0, dashboard?.aiUsage.currency ?? 'USD') }}</strong>
                  </div>
                </div>
              </article>

              <article class="admin-card">
                <div class="admin-card-head">
                  <div>
                    <h3 class="admin-subtitle">Top Accounts</h3>
                    <p class="admin-muted">定位高消耗账号与请求热点。</p>
                  </div>
                </div>
                <div class="admin-list-stack">
                  <article
                    v-for="item in dashboard?.aiUsage.topAccounts ?? []"
                    :key="`${item.accountMode}-${item.accountId}`"
                    class="admin-entity-card"
                  >
                    <div class="admin-entity-head">
                      <div>
                        <h4 class="admin-entity-title">{{ item.accountId }}</h4>
                        <p class="admin-entity-meta">{{ item.accountMode }} · {{ item.requests }} requests</p>
                      </div>
                      <span class="admin-badge admin-badge-info">{{ item.totalTokens }} tokens</span>
                    </div>
                    <p class="admin-description">{{ money(item.estimatedCost, item.currency) }}</p>
                  </article>
                </div>
              </article>
            </div>
          </div>
        </section>

        <section id="personal-accounts" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">个人账号与个人订阅</h2>
              <p class="admin-section-desc">维护个人账号资料、订阅映射和可否自定义端点。</p>
            </div>
          </div>

          <div class="admin-two-column">
            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">个人账号表单</h3>
                  <p class="admin-muted">适用于个人版用户的账号开通与订阅初始化。</p>
                </div>
              </div>
              <div class="admin-form-grid admin-form-grid-2">
                <label class="admin-field">
                  <span class="admin-label">账号 ID</span>
                  <input v-model="personalForm.id" placeholder="usr-new" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">显示名称</span>
                  <input v-model="personalForm.displayName" placeholder="显示名称" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">邮箱</span>
                  <input v-model="personalForm.email" placeholder="邮箱" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">登录密钥</span>
                  <input v-model="personalForm.secret" placeholder="登录密钥" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">订阅状态</span>
                  <select v-model="personalForm.subscriptionStatus" class="admin-input">
                    <option v-for="status in statusOptions" :key="status" :value="status">{{ status }}</option>
                  </select>
                </label>
                <label class="admin-check-field">
                  <span class="admin-label">端点能力</span>
                  <span class="admin-check-wrap">
                    <input v-model="personalForm.customEndpointEnabled" type="checkbox" class="admin-checkbox" />
                    <span>允许自定义端点</span>
                  </span>
                </label>
              </div>
              <button class="admin-button-primary mt-4 w-full" @click="savePersonalAccount">保存个人账号</button>

              <div class="admin-list-stack mt-4">
                <article v-for="account in dashboard?.personalAccounts ?? []" :key="account.id" class="admin-entity-card">
                  <div class="admin-entity-head">
                    <div>
                      <h4 class="admin-entity-title">{{ account.displayName }}</h4>
                      <p class="admin-entity-meta">{{ account.email }}</p>
                    </div>
                    <span :class="statusClass(account.subscriptionStatus)">{{ account.subscriptionStatus }}</span>
                  </div>
                  <div class="admin-chip-row">
                    <span class="admin-chip">{{ account.planName }}</span>
                    <span class="admin-chip">{{ account.customEndpointEnabled ? '允许自定义端点' : '仅托管端点' }}</span>
                  </div>
                  <div class="admin-entity-actions">
                    <button class="admin-button-secondary" @click="editPersonalAccount(account)">编辑</button>
                    <button class="admin-button-danger" @click="deletePersonalAccount(account.id)">删除</button>
                  </div>
                </article>
              </div>
            </article>

            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">个人订阅绑定</h3>
                  <p class="admin-muted">为个人账号指定方案，并同步订阅状态。</p>
                </div>
              </div>
              <div class="admin-form-grid">
                <label class="admin-field">
                  <span class="admin-label">个人账号</span>
                  <select v-model="personalSubscriptionForm.accountId" class="admin-input">
                    <option v-for="account in dashboard?.personalAccounts ?? []" :key="account.id" :value="account.id">
                      {{ account.displayName }}
                    </option>
                  </select>
                </label>
                <label class="admin-field">
                  <span class="admin-label">订阅方案</span>
                  <select v-model="personalSubscriptionForm.planCode" class="admin-input">
                    <option v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">
                      {{ plan.displayName }}
                    </option>
                  </select>
                </label>
                <label class="admin-field">
                  <span class="admin-label">状态</span>
                  <select v-model="personalSubscriptionForm.status" class="admin-input">
                    <option v-for="status in statusOptions" :key="status" :value="status">{{ status }}</option>
                  </select>
                </label>
                <button class="admin-button-primary w-full" @click="savePersonalSubscription">保存个人订阅</button>
              </div>

              <div class="admin-list-stack mt-4">
                <article
                  v-for="item in dashboard?.personalSubscriptions ?? []"
                  :key="item.accountId"
                  class="admin-entity-card"
                >
                  <div class="admin-entity-head">
                    <div>
                      <h4 class="admin-entity-title">{{ item.accountId }}</h4>
                      <p class="admin-entity-meta">{{ item.planDisplayName }}</p>
                    </div>
                    <span :class="statusClass(item.status)">{{ item.status }}</span>
                  </div>
                  <div class="admin-chip-row">
                    <span class="admin-chip">{{ money(item.pricePerSeat, item.currency) }}/month</span>
                    <span class="admin-chip">续期 {{ formatDate(item.renewAt) }}</span>
                  </div>
                </article>
              </div>
            </article>
          </div>
        </section>

        <section id="global-strategy" data-section class="admin-section">
          <div class="admin-section-head">
            <div>
              <h2 class="admin-section-title">全局策略与端点同步</h2>
              <p class="admin-section-desc">定义客户端默认策略，并统一下发托管端点配置。</p>
            </div>
          </div>

          <div class="admin-two-column">
            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">AI 全局策略</h3>
                  <p class="admin-muted">控制服务模式、默认方案与自定义端点策略。</p>
                </div>
              </div>
              <div class="admin-form-grid">
                <label class="admin-field">
                  <span class="admin-label">默认方案</span>
                  <select v-model="subscriptionForm.planName" class="admin-input">
                    <option v-for="plan in dashboard?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">
                      {{ plan.displayName }}
                    </option>
                  </select>
                </label>
                <label class="admin-field">
                  <span class="admin-label">默认席位</span>
                  <input v-model.number="subscriptionForm.seats" type="number" min="1" class="admin-input" />
                </label>
                <label class="admin-check-field">
                  <span class="admin-label">端点权限</span>
                  <span class="admin-check-wrap">
                    <input v-model="subscriptionForm.allowCustomEndpoint" type="checkbox" class="admin-checkbox" />
                    <span>允许用户自定义 AI 端点</span>
                  </span>
                </label>
                <label class="admin-check-field">
                  <span class="admin-label">同步策略</span>
                  <span class="admin-check-wrap">
                    <input v-model="subscriptionForm.syncCustomEndpoint" type="checkbox" class="admin-checkbox" />
                    <span>将平台端点同步给客户端</span>
                  </span>
                </label>
                <button class="admin-button-primary w-full" @click="saveSubscription">保存全局策略</button>
              </div>
            </article>

            <article class="admin-card">
              <div class="admin-card-head">
                <div>
                  <h3 class="admin-subtitle">自定义端点同步</h3>
                  <p class="admin-muted">将托管端点信息保存到后台，并同步至客户端。</p>
                </div>
              </div>
              <div class="admin-form-grid">
                <label class="admin-field">
                  <span class="admin-label">端点名称</span>
                  <input v-model="endpointForm.endpointName" placeholder="端点名称" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">Base URL</span>
                  <input v-model="endpointForm.baseUrl" placeholder="Base URL" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">Managed API Key</span>
                  <input v-model="endpointForm.apiKey" placeholder="Managed API Key" class="admin-input" />
                </label>
                <label class="admin-field">
                  <span class="admin-label">Model</span>
                  <input v-model="endpointForm.modelName" placeholder="Model" class="admin-input" />
                </label>
                <button class="admin-button-primary w-full" @click="saveEndpoint">保存端点配置</button>
              </div>
            </article>
          </div>
        </section>
      </main>
    </div>
  </div>
</template>
