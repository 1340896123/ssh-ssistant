import { computed, reactive, readonly, ref } from 'vue'
import { createCheckoutSession, generateBillingCycle, loadAdminDashboard, adminRequest } from '../services/adminApi'
import {
  DEFAULT_CHECKOUT_CANCEL_URL,
  DEFAULT_CHECKOUT_RETURN_URL,
  type AiEndpointSyncSettings,
  type AiSubscriptionOverview,
  type AiUsagePricingFormState,
  type BillingInvoiceStatus,
  type DashboardSnapshot,
  type EnterpriseFormState,
  type EnterpriseSubAccountSummary,
  type EnterpriseSubscriptionFormState,
  type EnterpriseSummary,
  type GenerateBillingCycleResponse,
  type PaymentFormState,
  type PaymentProviderFormState,
  type PersonalAccountSummary,
  type PersonalFormState,
  type PersonalSubscriptionFormState,
  type PlanFormState,
  type SubAccountFormState,
  type SubscriptionStatus,
} from '../types/admin'
import {
  money,
  parseApiError,
  serializeBillingInvoiceStatus,
  serializeSubscriptionStatus,
} from '../utils/adminFormat'
import { clearAdminSession } from './useAdminSession'

const dashboard = ref<DashboardSnapshot | null>(null)
const loading = ref(false)
const error = ref('')
const notice = ref('')
const selectedSubAccountId = ref('')
const selectedAssetIds = ref<string[]>([])

const enterpriseForm = reactive<EnterpriseFormState>({
  id: '',
  name: '',
  seatCount: 10,
  subscriptionPlan: 'enterprise',
  subscriptionStatus: 'active',
})

const subAccountForm = reactive<SubAccountFormState>({
  id: '',
  enterpriseId: 'ent-acme',
  displayName: '',
  email: '',
  secret: '',
  enabled: true,
})

const personalForm = reactive<PersonalFormState>({
  id: '',
  displayName: '',
  email: '',
  secret: '',
  subscriptionStatus: 'inactive',
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

const planForm = reactive<PlanFormState>({
  code: 'business',
  displayName: 'Business Monthly',
  scope: 'enterprise',
  pricePerSeat: 39,
  currency: 'USD',
  allowCustomEndpoint: true,
  isActive: true,
  description: '',
})

const enterpriseSubscriptionForm = reactive<EnterpriseSubscriptionFormState>({
  enterpriseId: 'ent-acme',
  planCode: 'enterprise',
  status: 'active',
  seatsPurchased: 40,
})

const personalSubscriptionForm = reactive<PersonalSubscriptionFormState>({
  accountId: 'usr-amy',
  planCode: 'personal',
  status: 'active',
})

const aiUsagePricingForm = reactive<AiUsagePricingFormState>({
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

const paymentForm = reactive<PaymentFormState>({
  invoiceId: '',
  amount: 0,
  providerKey: 'manual',
  currency: 'USD',
  paymentMethod: 'manual',
  status: 'completed',
  externalReference: '',
  note: '',
})

const paymentProviderForm = reactive<PaymentProviderFormState>({
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

export const statusOptions: SubscriptionStatus[] = ['inactive', 'trialing', 'active', 'pastDue', 'cancelled']
export const invoiceStatusOptions: BillingInvoiceStatus[] = ['open', 'paid', 'overdue', 'voided']
export const paymentStatusOptions = ['pending', 'completed', 'failed', 'refunded']

const selectedSubAccount = computed(
  () => dashboard.value?.subAccounts.find((item) => item.id === selectedSubAccountId.value) ?? null,
)

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

function resetPaymentForm(defaultProviderKey = paymentProviderForm.providerKey, defaultMethod = paymentProviderForm.providerType) {
  paymentForm.invoiceId = ''
  paymentForm.amount = 0
  paymentForm.currency = 'USD'
  paymentForm.providerKey = defaultProviderKey
  paymentForm.paymentMethod = defaultMethod
  paymentForm.status = 'completed'
  paymentForm.externalReference = ''
  paymentForm.note = ''
}

function syncDashboardForms(payload: DashboardSnapshot) {
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

  if (selectedSubAccountId.value) {
    const selected = payload.subAccounts.find((item) => item.id === selectedSubAccountId.value)
    selectedAssetIds.value = selected ? [...selected.assetIds] : []
  }
}

async function runDashboardAction(task: () => Promise<void>) {
  clearFeedback()
  try {
    await task()
  } catch (reason) {
    error.value = parseApiError(reason)
    if (error.value.includes('401')) {
      clearAdminSession()
    }
    throw reason
  }
}

export function useAdminDashboard() {
  async function loadDashboard(apiBase: string, token: string) {
    if (!token) return
    loading.value = true
    clearFeedback()
    try {
      const payload = await loadAdminDashboard(apiBase, token)
      dashboard.value = payload
      syncDashboardForms(payload)
    } catch (reason) {
      error.value = parseApiError(reason)
      if (error.value.includes('401')) {
        clearAdminSession()
      }
      throw reason
    } finally {
      loading.value = false
    }
  }

  async function saveEnterprise(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/enterprises', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...enterpriseForm,
          subscriptionStatus: serializeSubscriptionStatus(enterpriseForm.subscriptionStatus),
          renewAt: new Date(Date.now() + 30 * 86400_000).toISOString(),
        }),
      })
      resetEnterpriseForm()
      await loadDashboard(apiBase, token)
      notice.value = '企业账号已保存'
    })
  }

  function editEnterprise(enterprise: EnterpriseSummary) {
    enterpriseForm.id = enterprise.id
    enterpriseForm.name = enterprise.name
    enterpriseForm.seatCount = enterprise.seatCount
    enterpriseForm.subscriptionPlan = enterprise.subscriptionPlan
    enterpriseForm.subscriptionStatus = enterprise.subscriptionStatus
  }

  async function deleteEnterprise(apiBase: string, token: string, id: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, `/enterprises/${id}`, { method: 'DELETE' })
      if (enterpriseForm.id === id) resetEnterpriseForm()
      if (subAccountForm.enterpriseId === id) resetSubAccountForm()
      if (selectedSubAccount.value?.enterpriseId === id) {
        selectedSubAccountId.value = ''
        selectedAssetIds.value = []
      }
      await loadDashboard(apiBase, token)
      notice.value = '企业账号已删除'
    })
  }

  async function saveSubAccount(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      const assetIds =
        selectedSubAccountId.value === subAccountForm.id
          ? selectedAssetIds.value
          : dashboard.value?.subAccounts.find((item) => item.id === subAccountForm.id)?.assetIds ?? []

      await adminRequest(apiBase, token, '/sub-accounts', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...subAccountForm,
          assetIds,
        }),
      })
      resetSubAccountForm()
      await loadDashboard(apiBase, token)
      notice.value = subAccountForm.enabled ? '企业子账号已保存' : '企业子账号已禁用'
    })
  }

  function selectSubAccount(id: string, assetIds: string[]) {
    selectedSubAccountId.value = id
    selectedAssetIds.value = [...assetIds]
  }

  function editSubAccount(subAccount: EnterpriseSubAccountSummary) {
    subAccountForm.id = subAccount.id
    subAccountForm.enterpriseId = subAccount.enterpriseId
    subAccountForm.displayName = subAccount.displayName
    subAccountForm.email = subAccount.email
    subAccountForm.enabled = subAccount.enabled
    subAccountForm.secret = ''
    selectSubAccount(subAccount.id, subAccount.assetIds)
  }

  async function deleteSubAccount(apiBase: string, token: string, id: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, `/sub-accounts/${id}`, { method: 'DELETE' })
      if (selectedSubAccountId.value === id) {
        selectedSubAccountId.value = ''
        selectedAssetIds.value = []
      }
      if (subAccountForm.id === id) resetSubAccountForm()
      await loadDashboard(apiBase, token)
      notice.value = '企业子账号已删除'
    })
  }

  async function saveSubAccountAssets(apiBase: string, token: string) {
    if (!selectedSubAccountId.value) return

    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, `/sub-accounts/${selectedSubAccountId.value}/assets`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ assetIds: selectedAssetIds.value }),
      })
      await loadDashboard(apiBase, token)
      notice.value = '子账号资产授权已更新，并会在客户端下次同步时立即生效'
    })
  }

  async function savePersonalAccount(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/personal-accounts', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...personalForm,
          subscriptionStatus: serializeSubscriptionStatus(personalForm.subscriptionStatus),
        }),
      })
      resetPersonalForm()
      await loadDashboard(apiBase, token)
      notice.value = '个人账号已保存'
    })
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

  async function deletePersonalAccount(apiBase: string, token: string, id: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, `/personal-accounts/${id}`, { method: 'DELETE' })
      if (personalForm.id === id) resetPersonalForm()
      await loadDashboard(apiBase, token)
      notice.value = '个人账号已删除'
    })
  }

  async function saveSubscription(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/ai/subscription', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...subscriptionForm.value,
          status: serializeSubscriptionStatus(subscriptionForm.value.status),
        }),
      })
      await loadDashboard(apiBase, token)
      notice.value = 'AI 全局订阅策略已更新'
    })
  }

  async function savePlan(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/ai/plans', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(planForm),
      })
      await loadDashboard(apiBase, token)
      notice.value = 'AI 订阅方案已保存'
    })
  }

  async function saveEnterpriseSubscription(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/ai/enterprise-subscriptions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...enterpriseSubscriptionForm,
          status: serializeSubscriptionStatus(enterpriseSubscriptionForm.status),
          renewAt: new Date(Date.now() + 30 * 86400_000).toISOString(),
        }),
      })
      await loadDashboard(apiBase, token)
      notice.value = '企业订阅与席位已更新'
    })
  }

  async function savePersonalSubscription(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/ai/personal-subscriptions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...personalSubscriptionForm,
          status: serializeSubscriptionStatus(personalSubscriptionForm.status),
          renewAt: new Date(Date.now() + 30 * 86400_000).toISOString(),
        }),
      })
      await loadDashboard(apiBase, token)
      notice.value = '个人订阅已更新'
    })
  }

  async function saveAiUsagePricing(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/ai/usage-pricing', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(aiUsagePricingForm),
      })
      await loadDashboard(apiBase, token)
      notice.value = 'AI 定价表已更新'
    })
  }

  async function saveEndpoint(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/ai/endpoint-sync', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(endpointForm.value),
      })
      await loadDashboard(apiBase, token)
      notice.value = 'AI 自定义端点同步配置已更新'
    })
  }

  async function saveInvoiceStatus(apiBase: string, token: string, invoiceId: string) {
    const previousStatus = invoiceStatusForm[invoiceId]
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, `/billing/invoices/${invoiceId}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ status: serializeBillingInvoiceStatus(invoiceStatusForm[invoiceId]) }),
      })
      await loadDashboard(apiBase, token)
      notice.value = '账单状态已更新'
    }).catch(async () => {
      const invoice = dashboard.value?.billing.recentInvoices.find((item) => item.id === invoiceId)
      invoiceStatusForm[invoiceId] = invoice?.status ?? previousStatus
    })
  }

  async function savePayment(apiBase: string, token: string, invoiceId: string) {
    const invoice = dashboard.value?.billing.recentInvoices.find((item) => item.id === invoiceId)
    if (!invoice) return

    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/billing/payments', {
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
      resetPaymentForm()
      await loadDashboard(apiBase, token)
      notice.value = '支付记录已登记'
    })
  }

  async function createCheckout(apiBase: string, token: string, invoiceId: string) {
    const invoice = dashboard.value?.billing.recentInvoices.find((item) => item.id === invoiceId)
    if (!invoice) return

    await runDashboardAction(async () => {
      const transaction = await createCheckoutSession(apiBase, token, {
        invoiceId: invoice.id,
        providerKey: paymentProviderForm.providerKey,
        returnUrl: paymentProviderForm.successUrl || DEFAULT_CHECKOUT_RETURN_URL,
        cancelUrl: paymentProviderForm.cancelUrl || DEFAULT_CHECKOUT_CANCEL_URL,
      })
      if (transaction.checkoutUrl) {
        window.open(transaction.checkoutUrl, '_blank', 'noopener,noreferrer')
      }
      await loadDashboard(apiBase, token)
      notice.value = '支付链接已创建'
    })
  }

  async function savePaymentProvider(apiBase: string, token: string) {
    await runDashboardAction(async () => {
      await adminRequest(apiBase, token, '/billing/payment-providers', {
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
      await loadDashboard(apiBase, token)
      notice.value = '支付提供方配置已更新'
    })
  }

  async function generateCurrentBillingCycle(apiBase: string, token: string) {
    let result: GenerateBillingCycleResponse | null = null
    await runDashboardAction(async () => {
      result = await generateBillingCycle(apiBase, token)
      await loadDashboard(apiBase, token)
      notice.value = `本月账单已刷新，共生成 ${result?.generatedInvoices ?? 0} 条订阅账单`
    })
    return result
  }

  return {
    aiUsagePricingForm,
    clearFeedback,
    createCheckout,
    dashboard: readonly(dashboard),
    deleteEnterprise,
    deletePersonalAccount,
    deleteSubAccount,
    editEnterprise,
    editPersonalAccount,
    editSubAccount,
    endpointForm,
    enterpriseForm,
    enterpriseSubscriptionForm,
    error,
    generateCurrentBillingCycle,
    invoiceStatusForm,
    loadDashboard,
    loading,
    notice,
    paymentForm,
    paymentProviderForm,
    personalForm,
    personalSubscriptionForm,
    planForm,
    resetEnterpriseForm,
    resetPaymentForm,
    resetPersonalForm,
    resetSubAccountForm,
    saveAiUsagePricing,
    saveEndpoint,
    saveEnterprise,
    saveEnterpriseSubscription,
    saveInvoiceStatus,
    savePayment,
    savePaymentProvider,
    savePersonalAccount,
    savePersonalSubscription,
    savePlan,
    saveSubAccount,
    saveSubAccountAssets,
    saveSubscription,
    selectedAssetIds,
    selectedSubAccount,
    selectedSubAccountId,
    selectSubAccount,
    statusOptions,
    subscriptionForm,
    subAccountForm,
    invoiceStatusOptions,
    paymentStatusOptions,
    summaryCards: computed(() => {
      const selected = dashboard.value?.subAccounts.find((item) => item.id === selectedSubAccountId.value)
      const totalSeats = dashboard.value?.enterprises.reduce((sum, item) => sum + item.seatCount, 0) ?? 0
      const totalAssignedEnterpriseSeats =
        dashboard.value?.enterprises.reduce((sum, item) => sum + item.activeSubAccounts, 0) ?? 0
      return [
        {
          label: '企业账号',
          value: String(dashboard.value?.enterprises.length ?? 0),
          note: `${totalAssignedEnterpriseSeats}/${totalSeats} 已分配席位`,
        },
        {
          label: '企业子账号',
          value: String(dashboard.value?.subAccounts.length ?? 0),
          note: `当前选中授权 ${selected?.assetIds.length ?? 0} 台资产`,
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
          value: money(dashboard.value?.billing.estimatedMonthlyRevenue ?? 0),
          note: `未收 ${money(dashboard.value?.billing.outstandingAmount ?? 0)}`,
        },
      ]
    }),
  }
}
