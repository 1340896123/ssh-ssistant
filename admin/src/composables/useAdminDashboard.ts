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
  type GenerateBillingCycleResponse,
  type PaymentFormState,
  type PaymentProviderFormState,
  type PersonalAccountSummary,
  type PersonalFormState,
  type PersonalSubscriptionFormState,
  type PlanFormState,
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
  planName: 'personal',
  planDisplayName: 'Personal Managed',
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
  scope: 'personal',
  pricePerSeat: 39,
  currency: 'USD',
  allowCustomEndpoint: true,
  isActive: true,
  description: '',
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

function clearFeedback() {
  error.value = ''
  notice.value = ''
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
    deletePersonalAccount,
    editPersonalAccount,
    endpointForm,
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
    resetPaymentForm,
    resetPersonalForm,
    saveAiUsagePricing,
    saveEndpoint,
    saveInvoiceStatus,
    savePayment,
    savePaymentProvider,
    savePersonalAccount,
    savePersonalSubscription,
    savePlan,
    saveSubscription,
    statusOptions,
    subscriptionForm,
    invoiceStatusOptions,
    paymentStatusOptions,
    summaryCards: computed(() => {
      return [
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
