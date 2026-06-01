<script setup lang="ts">
import { computed } from 'vue'
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'
import { formatDate, invoiceBadge, money } from '../utils/adminFormat'

const session = useAdminSession()
const dashboardState = useAdminDashboard()

const paymentProviderHint = computed(() => {
  const form = dashboardState.paymentProviderForm
  if (form.providerType === 'stripe') {
    return `{"checkoutBaseUrl":"https://payments.example.com/stripe-checkout","apiBaseUrl":"https://api.stripe.com","stripeApiVersion":"2024-06-20","webhookMode":"stripe-like","webhookToleranceSeconds":300,"successUrl":"sshstar://billing/success","cancelUrl":"sshstar://billing/cancel"}`
  }
  if (form.providerType === 'manual') {
    return '{"checkoutBaseUrl":"https://payments.example.com/manual-checkout","webhookMode":"manual"}'
  }
  return '{"checkoutBaseUrl":"https://payments.example.com/provider-checkout"}'
})

async function generateCurrentBillingCycle() {
  await dashboardState.generateCurrentBillingCycle(session.apiBase.value, session.authToken.value)
}

async function savePaymentProvider() {
  await dashboardState.savePaymentProvider(session.apiBase.value, session.authToken.value)
}

async function saveInvoiceStatus(invoiceId: string) {
  await dashboardState.saveInvoiceStatus(session.apiBase.value, session.authToken.value, invoiceId)
}

async function savePayment(invoiceId: string) {
  await dashboardState.savePayment(session.apiBase.value, session.authToken.value, invoiceId)
}

async function createCheckout(invoiceId: string) {
  await dashboardState.createCheckout(session.apiBase.value, session.authToken.value, invoiceId)
}
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="账单中心" description="跟踪账期、支付提供方、支付记录与应收状态。">
      <div class="admin-pill-row">
        <span class="admin-pill">账期 {{ dashboardState.dashboard.value?.billing.billingMonth ?? '-' }}</span>
        <span class="admin-pill">未收 {{ money(dashboardState.dashboard.value?.billing.outstandingAmount ?? 0) }}</span>
        <span class="admin-pill">Open {{ dashboardState.dashboard.value?.billing.openInvoiceCount ?? 0 }}</span>
      </div>
    </AdminPageIntro>

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
          <input v-model="dashboardState.paymentProviderForm.providerKey" placeholder="provider key" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">显示名称</span>
          <input v-model="dashboardState.paymentProviderForm.displayName" placeholder="display name" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">Provider Type</span>
          <input
            v-model="dashboardState.paymentProviderForm.providerType"
            placeholder="manual / stripe / alipay"
            class="admin-input"
          />
        </label>
        <label class="admin-field">
          <span class="admin-label">Webhook Secret</span>
          <input v-model="dashboardState.paymentProviderForm.webhookSecret" placeholder="webhook secret" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">Checkout Base URL</span>
          <input
            v-model="dashboardState.paymentProviderForm.checkoutBaseUrl"
            placeholder="checkout base url"
            class="admin-input"
          />
        </label>
        <label class="admin-field">
          <span class="admin-label">Webhook Mode</span>
          <input
            v-model="dashboardState.paymentProviderForm.webhookMode"
            placeholder="manual / stripe-like"
            class="admin-input"
          />
        </label>
      </div>

      <div v-if="dashboardState.paymentProviderForm.providerType === 'stripe'" class="admin-form-grid admin-form-grid-3 mt-4">
        <label class="admin-field">
          <span class="admin-label">Stripe API Base URL</span>
          <input v-model="dashboardState.paymentProviderForm.apiBaseUrl" placeholder="stripe api base url" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">Secret API Key</span>
          <input v-model="dashboardState.paymentProviderForm.secretApiKey" placeholder="stripe secret api key" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">API Version</span>
          <input v-model="dashboardState.paymentProviderForm.stripeApiVersion" placeholder="stripe api version" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">Webhook Tolerance</span>
          <input
            v-model.number="dashboardState.paymentProviderForm.webhookToleranceSeconds"
            type="number"
            min="30"
            step="30"
            class="admin-input"
          />
        </label>
        <label class="admin-field">
          <span class="admin-label">Success URL</span>
          <input v-model="dashboardState.paymentProviderForm.successUrl" placeholder="success url" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">Cancel URL</span>
          <input v-model="dashboardState.paymentProviderForm.cancelUrl" placeholder="cancel url" class="admin-input" />
        </label>
      </div>

      <div class="admin-card-toolbar">
        <label class="admin-check-field">
          <span class="admin-label">启用状态</span>
          <span class="admin-check-wrap">
            <input v-model="dashboardState.paymentProviderForm.enabled" type="checkbox" class="admin-checkbox" />
            <span>启用支付提供方</span>
          </span>
        </label>
        <button class="admin-button-primary" @click="savePaymentProvider">保存支付提供方</button>
      </div>
      <p class="admin-hint">Metadata template: {{ paymentProviderHint }}</p>
    </div>

    <div v-if="dashboardState.dashboard.value?.billing.recentInvoices.length" class="admin-list-stack">
      <article v-for="invoice in dashboardState.dashboard.value?.billing.recentInvoices ?? []" :key="invoice.id" class="admin-card">
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
            <select v-model="dashboardState.invoiceStatusForm[invoice.id]" class="admin-input admin-input-compact">
              <option v-for="status in dashboardState.invoiceStatusOptions" :key="status" :value="status">{{ status }}</option>
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
            <button class="admin-button-secondary w-full" @click="createCheckout(invoice.id)">创建支付链接</button>
            <div class="admin-form-grid admin-form-grid-2 mt-4">
              <label class="admin-field">
                <span class="admin-label">支付金额</span>
                <input
                  v-model.number="dashboardState.paymentForm.amount"
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
                  v-model="dashboardState.paymentForm.externalReference"
                  placeholder="external reference"
                  class="admin-input"
                />
              </label>
              <label class="admin-field">
                <span class="admin-label">支付方式</span>
                <input
                  v-model="dashboardState.paymentForm.paymentMethod"
                  placeholder="manual / stripe / bank"
                  class="admin-input"
                />
              </label>
              <label class="admin-field">
                <span class="admin-label">备注</span>
                <input v-model="dashboardState.paymentForm.note" placeholder="note" class="admin-input" />
              </label>
            </div>
            <label class="admin-field mt-4">
              <span class="admin-label">支付状态</span>
              <select v-model="dashboardState.paymentForm.status" class="admin-input">
                <option v-for="status in dashboardState.paymentStatusOptions" :key="status" :value="status">{{ status }}</option>
              </select>
            </label>
            <button class="admin-button-primary mt-4 w-full" @click="savePayment(invoice.id)">登记付款</button>
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

    <div v-else-if="!dashboardState.loading.value" class="admin-empty-state">当前还没有账单记录，生成本月账期后会显示在这里。</div>
  </section>
</template>
