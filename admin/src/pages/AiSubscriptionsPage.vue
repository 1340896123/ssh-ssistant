<script setup lang="ts">
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'
import { formatDate, money, statusClass } from '../utils/adminFormat'

const session = useAdminSession()
const dashboardState = useAdminDashboard()

async function savePlan() {
  await dashboardState.savePlan(session.apiBase.value, session.authToken.value)
}

async function saveEnterpriseSubscription() {
  await dashboardState.saveEnterpriseSubscription(session.apiBase.value, session.authToken.value)
}
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="AI 订阅与席位配置" description="维护平台方案目录，并将方案映射到企业席位。">
      <div class="admin-pill-row">
        <span class="admin-pill">
          当前全局策略 {{ dashboardState.subscriptionForm.value.planDisplayName }} ·
          {{ money(dashboardState.subscriptionForm.value.pricePerSeat, dashboardState.subscriptionForm.value.currency) }}/seat
        </span>
      </div>
    </AdminPageIntro>

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
            <input v-model="dashboardState.planForm.code" placeholder="plan-code" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">显示名称</span>
            <input v-model="dashboardState.planForm.displayName" placeholder="显示名称" class="admin-input" />
          </label>
          <div class="admin-form-grid admin-form-grid-3">
            <label class="admin-field">
              <span class="admin-label">作用域</span>
              <input v-model="dashboardState.planForm.scope" placeholder="enterprise / personal" class="admin-input" />
            </label>
            <label class="admin-field">
              <span class="admin-label">单席位价格</span>
              <input v-model.number="dashboardState.planForm.pricePerSeat" type="number" min="0" class="admin-input" />
            </label>
            <label class="admin-field">
              <span class="admin-label">货币</span>
              <input v-model="dashboardState.planForm.currency" placeholder="USD" class="admin-input" />
            </label>
          </div>
          <label class="admin-field">
            <span class="admin-label">方案说明</span>
            <textarea v-model="dashboardState.planForm.description" rows="3" class="admin-input admin-textarea" />
          </label>
          <label class="admin-check-field">
            <span class="admin-label">扩展策略</span>
            <span class="admin-check-wrap">
              <input v-model="dashboardState.planForm.allowCustomEndpoint" type="checkbox" class="admin-checkbox" />
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
            <select v-model="dashboardState.enterpriseSubscriptionForm.enterpriseId" class="admin-input">
              <option
                v-for="enterprise in dashboardState.dashboard.value?.enterprises ?? []"
                :key="enterprise.id"
                :value="enterprise.id"
              >
                {{ enterprise.name }}
              </option>
            </select>
          </label>
          <label class="admin-field">
            <span class="admin-label">订阅方案</span>
            <select v-model="dashboardState.enterpriseSubscriptionForm.planCode" class="admin-input">
              <option v-for="plan in dashboardState.dashboard.value?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">
                {{ plan.displayName }}
              </option>
            </select>
          </label>
          <div class="admin-form-grid admin-form-grid-2">
            <label class="admin-field">
              <span class="admin-label">状态</span>
              <select v-model="dashboardState.enterpriseSubscriptionForm.status" class="admin-input">
                <option v-for="status in dashboardState.statusOptions" :key="status" :value="status">{{ status }}</option>
              </select>
            </label>
            <label class="admin-field">
              <span class="admin-label">购买席位</span>
              <input
                v-model.number="dashboardState.enterpriseSubscriptionForm.seatsPurchased"
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
        <div v-if="dashboardState.dashboard.value?.subscriptionPlans.length" class="admin-list-stack">
          <article v-for="plan in dashboardState.dashboard.value?.subscriptionPlans ?? []" :key="plan.code" class="admin-entity-card">
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
        <div v-else-if="!dashboardState.loading.value" class="admin-empty-state">当前还没有 AI 订阅方案，请先创建方案。</div>
      </article>

      <article class="admin-card">
        <div class="admin-card-head">
          <div>
            <h3 class="admin-subtitle">企业订阅列表</h3>
            <p class="admin-muted">查看企业当前方案、席位使用与续期信息。</p>
          </div>
        </div>
        <div v-if="dashboardState.dashboard.value?.enterpriseSubscriptions.length" class="admin-list-stack">
          <article
            v-for="item in dashboardState.dashboard.value?.enterpriseSubscriptions ?? []"
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
        <div v-else-if="!dashboardState.loading.value" class="admin-empty-state">当前还没有企业订阅绑定记录。</div>
      </article>
    </div>
  </section>
</template>
