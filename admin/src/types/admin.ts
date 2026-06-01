export const DEFAULT_ADMIN_API_BASE = 'http://localhost:5047/api/admin'
export const DEFAULT_CHECKOUT_RETURN_URL = 'sshstar://billing/success'
export const DEFAULT_CHECKOUT_CANCEL_URL = 'sshstar://billing/cancel'

export type SubscriptionStatus = 'inactive' | 'trialing' | 'active' | 'pastDue' | 'cancelled'
export type BillingInvoiceStatus = 'open' | 'paid' | 'overdue' | 'voided'

export interface EnterpriseSummary {
  id: string
  name: string
  seatCount: number
  activeSubAccounts: number
  subscriptionPlan: string
  subscriptionStatus: SubscriptionStatus
  renewAt: string
}

export interface EnterpriseSubAccountSummary {
  id: string
  enterpriseId: string
  displayName: string
  email: string
  enabled: boolean
  assetIds: string[]
  updatedAt: string
}

export interface PersonalAccountSummary {
  id: string
  displayName: string
  email: string
  subscriptionStatus: SubscriptionStatus
  planName: string
  customEndpointEnabled: boolean
  updatedAt: string
}

export interface ManagedAssetSummary {
  id: string
  name: string
  host: string
  environment: string
  riskLevel: string
  ownerType: string
}

export interface AiSubscriptionPlanSummary {
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

export interface EnterpriseSubscriptionSummary {
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

export interface PersonalSubscriptionSummary {
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

export interface AiSubscriptionOverview {
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

export interface AiEndpointSyncSettings {
  endpointName: string
  provider: string
  baseUrl: string
  apiKey: string
  modelName: string
  syncToClients: boolean
  updatedAt: string
}

export interface BillingInvoiceLineItemSummary {
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

export interface PaymentTransactionSummary {
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

export interface BillingInvoiceSummary {
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

export interface PaymentProviderConfigSummary {
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

export interface BillingOverview {
  billingMonth: string
  estimatedMonthlyRevenue: number
  outstandingAmount: number
  openInvoiceCount: number
  recentInvoices: BillingInvoiceSummary[]
}

export interface AiUsageAccountSummary {
  accountId: string
  accountMode: string
  requests: number
  totalTokens: number
  estimatedCost: number
  currency: string
}

export interface AiUsageSummary {
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

export interface AiUsagePricingSummary {
  id: string
  provider: string
  modelName: string
  promptTokenRatePerMillion: number
  completionTokenRatePerMillion: number
  currency: string
  isActive: boolean
  updatedAt: string
}

export interface GenerateBillingCycleResponse {
  billing: BillingOverview
  generatedInvoices: number
}

export interface DashboardSnapshot {
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

export interface AdminLoginResponse {
  token: string
  refreshToken: string
  username: string
  role: string
  expiresAt: string
  refreshExpiresAt: string
}

export interface LoginFormState {
  username: string
  password: string
}

export interface EnterpriseFormState {
  id: string
  name: string
  seatCount: number
  subscriptionPlan: string
  subscriptionStatus: SubscriptionStatus
}

export interface SubAccountFormState {
  id: string
  enterpriseId: string
  displayName: string
  email: string
  secret: string
  enabled: boolean
}

export interface PersonalFormState {
  id: string
  displayName: string
  email: string
  secret: string
  subscriptionStatus: SubscriptionStatus
  planName: string
  customEndpointEnabled: boolean
}

export interface PlanFormState {
  code: string
  displayName: string
  scope: string
  pricePerSeat: number
  currency: string
  allowCustomEndpoint: boolean
  isActive: boolean
  description: string
}

export interface EnterpriseSubscriptionFormState {
  enterpriseId: string
  planCode: string
  status: SubscriptionStatus
  seatsPurchased: number
}

export interface PersonalSubscriptionFormState {
  accountId: string
  planCode: string
  status: SubscriptionStatus
}

export interface AiUsagePricingFormState {
  id: string
  provider: string
  modelName: string
  promptTokenRatePerMillion: number
  completionTokenRatePerMillion: number
  currency: string
  isActive: boolean
}

export interface PaymentFormState {
  invoiceId: string
  amount: number
  providerKey: string
  currency: string
  paymentMethod: string
  status: string
  externalReference: string
  note: string
}

export interface PaymentProviderFormState {
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
}

export type AdminRouteName =
  | 'login'
  | 'overview'
  | 'enterprises'
  | 'subaccounts'
  | 'ai-subscriptions'
  | 'billing'
  | 'ai-usage'
  | 'personal-accounts'
  | 'global-strategy'

export interface AdminPageMeta {
  title: string
  description: string
}
