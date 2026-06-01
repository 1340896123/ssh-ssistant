<script setup lang="ts">
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'

const session = useAdminSession()
const dashboardState = useAdminDashboard()

async function saveSubscription() {
  await dashboardState.saveSubscription(session.apiBase.value, session.authToken.value)
}

async function saveEndpoint() {
  await dashboardState.saveEndpoint(session.apiBase.value, session.authToken.value)
}
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="全局策略与端点同步" description="定义客户端默认策略，并统一下发托管端点配置。" />

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
            <select v-model="dashboardState.subscriptionForm.value.planName" class="admin-input">
              <option v-for="plan in dashboardState.dashboard.value?.subscriptionPlans ?? []" :key="plan.code" :value="plan.code">
                {{ plan.displayName }}
              </option>
            </select>
          </label>
          <label class="admin-field">
            <span class="admin-label">默认席位</span>
            <input v-model.number="dashboardState.subscriptionForm.value.seats" type="number" min="1" class="admin-input" />
          </label>
          <label class="admin-check-field">
            <span class="admin-label">端点权限</span>
            <span class="admin-check-wrap">
              <input v-model="dashboardState.subscriptionForm.value.allowCustomEndpoint" type="checkbox" class="admin-checkbox" />
              <span>允许用户自定义 AI 端点</span>
            </span>
          </label>
          <label class="admin-check-field">
            <span class="admin-label">同步策略</span>
            <span class="admin-check-wrap">
              <input v-model="dashboardState.subscriptionForm.value.syncCustomEndpoint" type="checkbox" class="admin-checkbox" />
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
            <input v-model="dashboardState.endpointForm.value.endpointName" placeholder="端点名称" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">Base URL</span>
            <input v-model="dashboardState.endpointForm.value.baseUrl" placeholder="Base URL" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">Managed API Key</span>
            <input v-model="dashboardState.endpointForm.value.apiKey" placeholder="Managed API Key" class="admin-input" />
          </label>
          <label class="admin-field">
            <span class="admin-label">Model</span>
            <input v-model="dashboardState.endpointForm.value.modelName" placeholder="Model" class="admin-input" />
          </label>
          <button class="admin-button-primary w-full" @click="saveEndpoint">保存端点配置</button>
        </div>
      </article>
    </div>
  </section>
</template>
