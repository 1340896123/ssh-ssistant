<script setup lang="ts">
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'
import { riskClass } from '../utils/adminFormat'

const session = useAdminSession()
const dashboardState = useAdminDashboard()

async function saveSubAccount() {
  await dashboardState.saveSubAccount(session.apiBase.value, session.authToken.value)
}

async function deleteSubAccount(id: string) {
  await dashboardState.deleteSubAccount(session.apiBase.value, session.authToken.value, id)
}

async function saveSubAccountAssets() {
  await dashboardState.saveSubAccountAssets(session.apiBase.value, session.authToken.value)
}

function editSubAccount(
  id: string,
  enterpriseId: string,
  displayName: string,
  email: string,
  enabled: boolean,
  assetIds: readonly string[],
  updatedAt: string,
) {
  dashboardState.editSubAccount({
    id,
    enterpriseId,
    displayName,
    email,
    enabled,
    assetIds: [...assetIds],
    updatedAt,
  })
}
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="企业子账号与资产授权" description="为企业成员分配登录身份，并限定其可同步的资产范围。" />

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
          <input v-model="dashboardState.subAccountForm.id" placeholder="sub-new" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">所属企业</span>
          <select v-model="dashboardState.subAccountForm.enterpriseId" class="admin-input">
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
          <span class="admin-label">显示名称</span>
          <input v-model="dashboardState.subAccountForm.displayName" placeholder="显示名称" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">邮箱</span>
          <input v-model="dashboardState.subAccountForm.email" placeholder="邮箱" class="admin-input" />
        </label>
        <label class="admin-field">
          <span class="admin-label">登录密钥</span>
          <input v-model="dashboardState.subAccountForm.secret" placeholder="登录密钥" class="admin-input" />
        </label>
        <label class="admin-check-field">
          <span class="admin-label">状态</span>
          <span class="admin-check-wrap">
            <input v-model="dashboardState.subAccountForm.enabled" type="checkbox" class="admin-checkbox" />
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
        <div v-if="dashboardState.dashboard.value?.subAccounts.length" class="admin-list-stack">
          <button
            v-for="subAccount in dashboardState.dashboard.value?.subAccounts ?? []"
            :key="subAccount.id"
            type="button"
            class="admin-select-card"
            :class="{ 'is-active': dashboardState.selectedSubAccountId.value === subAccount.id }"
            @click="dashboardState.selectSubAccount(subAccount.id, [...subAccount.assetIds])"
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
              <span
                class="admin-link-button"
                @click.stop="
                  editSubAccount(
                    subAccount.id,
                    subAccount.enterpriseId,
                    subAccount.displayName,
                    subAccount.email,
                    subAccount.enabled,
                    subAccount.assetIds,
                    subAccount.updatedAt,
                  )
                "
              >
                编辑
              </span>
              <span class="admin-link-button admin-link-button-danger" @click.stop="deleteSubAccount(subAccount.id)">删除</span>
            </div>
          </button>
        </div>
        <div v-else-if="!dashboardState.loading.value" class="admin-empty-state">暂无企业子账号，创建后即可在这里分配资产授权。</div>
      </article>

      <article class="admin-card">
        <div class="admin-card-head">
          <div>
            <h3 class="admin-subtitle">资产授权明细</h3>
            <p class="admin-muted">
              {{
                dashboardState.selectedSubAccount.value
                  ? `${dashboardState.selectedSubAccount.value.displayName} 当前可访问的资产列表`
                  : '请选择左侧子账号后配置授权。'
              }}
            </p>
          </div>
          <button class="admin-button-primary" :disabled="!dashboardState.selectedSubAccount.value" @click="saveSubAccountAssets">
            保存授权
          </button>
        </div>

        <div v-if="dashboardState.selectedSubAccount.value" class="admin-list-stack">
          <label v-for="asset in dashboardState.dashboard.value?.assets ?? []" :key="asset.id" class="admin-asset-row">
            <span class="admin-check-wrap">
              <input
                v-model="dashboardState.selectedAssetIds.value"
                :value="asset.id"
                type="checkbox"
                class="admin-checkbox"
              />
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
</template>
