<script setup lang="ts">
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'
import { formatDate, statusClass } from '../utils/adminFormat'

const session = useAdminSession()
const dashboardState = useAdminDashboard()

async function saveEnterprise() {
  await dashboardState.saveEnterprise(session.apiBase.value, session.authToken.value)
}

async function deleteEnterprise(id: string) {
  await dashboardState.deleteEnterprise(session.apiBase.value, session.authToken.value, id)
}
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="企业账号管理" description="维护企业名称、席位规模和订阅状态。">
      <div class="admin-pill-row">
        <span class="admin-pill">企业数 {{ dashboardState.dashboard.value?.enterprises.length ?? 0 }}</span>
        <span class="admin-pill">
          总席位 {{ dashboardState.dashboard.value?.enterprises.reduce((sum, item) => sum + item.seatCount, 0) ?? 0 }}
        </span>
      </div>
    </AdminPageIntro>

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
          <input v-model="dashboardState.enterpriseForm.id" placeholder="ent-new" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">企业名称</span>
          <input v-model="dashboardState.enterpriseForm.name" placeholder="企业名称" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">购买席位</span>
          <input
            v-model.number="dashboardState.enterpriseForm.seatCount"
            type="number"
            min="1"
            placeholder="购买席位数"
            class="admin-input"
          />
        </label>
        <label class="admin-field">
          <span class="admin-label">订阅状态</span>
          <select v-model="dashboardState.enterpriseForm.subscriptionStatus" class="admin-input">
            <option v-for="status in dashboardState.statusOptions" :key="status" :value="status">{{ status }}</option>
          </select>
        </label>
        <div class="admin-field admin-actions-end">
          <span class="admin-label">操作</span>
          <button class="admin-button-primary w-full" @click="saveEnterprise">保存企业</button>
        </div>
      </div>
    </div>

    <div v-if="dashboardState.dashboard.value?.enterprises.length" class="admin-list-grid">
      <article
        v-for="enterprise in dashboardState.dashboard.value?.enterprises ?? []"
        :key="enterprise.id"
        class="admin-entity-card"
      >
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
          <button class="admin-button-secondary" @click="dashboardState.editEnterprise(enterprise)">编辑</button>
          <button class="admin-button-danger" @click="deleteEnterprise(enterprise.id)">删除</button>
        </div>
      </article>
    </div>

    <div v-else-if="!dashboardState.loading.value" class="admin-empty-state">暂无企业账号，保存第一条企业记录后会显示在这里。</div>
  </section>
</template>
