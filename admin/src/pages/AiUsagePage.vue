<script setup lang="ts">
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'
import { money } from '../utils/adminFormat'

const session = useAdminSession()
const dashboardState = useAdminDashboard()

async function saveAiUsagePricing() {
  await dashboardState.saveAiUsagePricing(session.apiBase.value, session.authToken.value)
}
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="AI 用量与成本" description="查看请求量、Token 结构、定价表与高消耗账号。">
      <div class="admin-pill-row">
        <span class="admin-pill">本月请求 {{ dashboardState.dashboard.value?.aiUsage.totalRequests ?? 0 }}</span>
        <span class="admin-pill">托管请求 {{ dashboardState.dashboard.value?.aiUsage.managedRequests ?? 0 }}</span>
        <span class="admin-pill">Tokens {{ dashboardState.dashboard.value?.aiUsage.totalTokens ?? 0 }}</span>
        <span class="admin-pill">
          估算
          {{ money(dashboardState.dashboard.value?.aiUsage.estimatedCost ?? 0, dashboardState.dashboard.value?.aiUsage.currency ?? 'USD') }}
        </span>
      </div>
    </AdminPageIntro>

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
            <input v-model="dashboardState.aiUsagePricingForm.id" placeholder="pricing-id" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">提供方</span>
            <input v-model="dashboardState.aiUsagePricingForm.provider" placeholder="provider" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">模型名</span>
            <input v-model="dashboardState.aiUsagePricingForm.modelName" placeholder="model name" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">货币</span>
            <input v-model="dashboardState.aiUsagePricingForm.currency" placeholder="USD" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">Prompt / 1M</span>
            <input
              v-model.number="dashboardState.aiUsagePricingForm.promptTokenRatePerMillion"
              type="number"
              min="0"
              step="0.01"
              class="admin-input"
            />
          </label>
          <label class="admin-field">
            <span class="admin-label">Completion / 1M</span>
            <input
              v-model.number="dashboardState.aiUsagePricingForm.completionTokenRatePerMillion"
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
              <input v-model="dashboardState.aiUsagePricingForm.isActive" type="checkbox" class="admin-checkbox" />
              <span>启用该价格</span>
            </span>
          </label>
          <button class="admin-button-primary" @click="saveAiUsagePricing">保存 AI 定价</button>
        </div>
        <div v-if="dashboardState.dashboard.value?.aiUsagePricing.length" class="admin-list-stack mt-4">
          <article
            v-for="pricing in dashboardState.dashboard.value?.aiUsagePricing ?? []"
            :key="pricing.id"
            class="admin-entity-card"
          >
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
              <strong>{{ dashboardState.dashboard.value?.aiUsage.promptTokens ?? 0 }}</strong>
            </div>
            <div class="admin-kv-row">
              <span>Completion Tokens</span>
              <strong>{{ dashboardState.dashboard.value?.aiUsage.completionTokens ?? 0 }}</strong>
            </div>
            <div class="admin-kv-row">
              <span>Total Tokens</span>
              <strong>{{ dashboardState.dashboard.value?.aiUsage.totalTokens ?? 0 }}</strong>
            </div>
            <div class="admin-kv-row">
              <span>Estimated Cost</span>
              <strong>
                {{ money(dashboardState.dashboard.value?.aiUsage.estimatedCost ?? 0, dashboardState.dashboard.value?.aiUsage.currency ?? 'USD') }}
              </strong>
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
          <div v-if="dashboardState.dashboard.value?.aiUsage.topAccounts.length" class="admin-list-stack">
            <article
              v-for="item in dashboardState.dashboard.value?.aiUsage.topAccounts ?? []"
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
          <div v-else-if="!dashboardState.loading.value" class="admin-empty-state">当前没有 AI 用量数据。</div>
        </article>
      </div>
    </div>
  </section>
</template>
