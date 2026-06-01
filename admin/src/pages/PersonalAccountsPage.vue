<script setup lang="ts">
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'
import { formatDate, money, statusClass } from '../utils/adminFormat'

const session = useAdminSession()
const dashboardState = useAdminDashboard()

async function savePersonalAccount() {
  await dashboardState.savePersonalAccount(session.apiBase.value, session.authToken.value)
}

async function deletePersonalAccount(id: string) {
  await dashboardState.deletePersonalAccount(session.apiBase.value, session.authToken.value, id)
}

async function savePersonalSubscription() {
  await dashboardState.savePersonalSubscription(session.apiBase.value, session.authToken.value)
}
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="个人账号与个人订阅" description="维护个人账号资料、订阅映射和可否自定义端点。" />

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
            <input v-model="dashboardState.personalForm.id" placeholder="usr-new" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">显示名称</span>
            <input v-model="dashboardState.personalForm.displayName" placeholder="显示名称" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">邮箱</span>
            <input v-model="dashboardState.personalForm.email" placeholder="邮箱" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">登录密钥</span>
            <input v-model="dashboardState.personalForm.secret" placeholder="登录密钥" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">订阅状态</span>
            <select v-model="dashboardState.personalForm.subscriptionStatus" class="admin-input">
              <option v-for="status in dashboardState.statusOptions" :key="status" :value="status">{{ status }}</option>
            </select>
          </label>
          <label class="admin-check-field">
            <span class="admin-label">端点能力</span>
            <span class="admin-check-wrap">
              <input v-model="dashboardState.personalForm.customEndpointEnabled" type="checkbox" class="admin-checkbox" />
              <span>允许自定义端点</span>
            </span>
          </label>
        </div>
        <button class="admin-button-primary mt-4 w-full" @click="savePersonalAccount">保存个人账号</button>

        <div v-if="dashboardState.dashboard.value?.personalAccounts.length" class="admin-list-stack mt-4">
          <article v-for="account in dashboardState.dashboard.value?.personalAccounts ?? []" :key="account.id" class="admin-entity-card">
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
              <button class="admin-button-secondary" @click="dashboardState.editPersonalAccount(account)">编辑</button>
              <button class="admin-button-danger" @click="deletePersonalAccount(account.id)">删除</button>
            </div>
          </article>
        </div>
        <div v-else-if="!dashboardState.loading.value" class="admin-empty-state mt-4">暂无个人账号，保存后会显示在这里。</div>
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
            <select v-model="dashboardState.personalSubscriptionForm.accountId" class="admin-input">
              <option
                v-for="account in dashboardState.dashboard.value?.personalAccounts ?? []"
                :key="account.id"
                :value="account.id"
              >
                {{ account.displayName }}
              </option>
            </select>
          </label>
          <label class="admin-field">
            <span class="admin-label">订阅方案</span>
            <select v-model="dashboardState.personalSubscriptionForm.planCode" class="admin-input">
              <option v-for="plan in dashboardState.dashboard.value?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">
                {{ plan.displayName }}
              </option>
            </select>
          </label>
          <label class="admin-field">
            <span class="admin-label">状态</span>
            <select v-model="dashboardState.personalSubscriptionForm.status" class="admin-input">
              <option v-for="status in dashboardState.statusOptions" :key="status" :value="status">{{ status }}</option>
            </select>
          </label>
          <button class="admin-button-primary w-full" @click="savePersonalSubscription">保存个人订阅</button>
        </div>

        <div v-if="dashboardState.dashboard.value?.personalSubscriptions.length" class="admin-list-stack mt-4">
          <article
            v-for="item in dashboardState.dashboard.value?.personalSubscriptions ?? []"
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
        <div v-else-if="!dashboardState.loading.value" class="admin-empty-state mt-4">当前还没有个人订阅绑定记录。</div>
      </article>
    </div>
  </section>
</template>
