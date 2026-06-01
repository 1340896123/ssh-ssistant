<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminSession } from '../composables/useAdminSession'

const router = useRouter()
const route = useRoute()
const session = useAdminSession()
const dashboardState = useAdminDashboard()

const redirectTarget = computed(() => {
  const redirect = route.query.redirect
  return typeof redirect === 'string' && redirect.startsWith('/') ? redirect : '/overview'
})

async function handleLogin() {
  const success = await session.login()
  if (!success) return

  try {
    await dashboardState.loadDashboard(session.apiBase.value, session.authToken.value)
    await router.replace(redirectTarget.value)
  } catch {
    if (!session.isAuthenticated.value) {
      await router.replace({ name: 'login' })
    }
  }
}
</script>

<template>
  <div class="admin-shell">
    <div class="admin-login-shell">
      <div class="admin-login-panel">
        <div class="space-y-6">
          <div class="admin-eyebrow">SSH Assistant Admin</div>
          <div>
            <h1 class="admin-login-title">企业后台登录</h1>
            <p class="admin-login-subtitle">
              统一管理企业账号、个人账号、订阅策略、账单与 AI 资源分配。
            </p>
          </div>
        </div>

        <div class="admin-login-grid">
          <div class="admin-card admin-card-muted">
            <div class="admin-card-head">
              <div>
                <h2 class="admin-subtitle">登录凭据</h2>
                <p class="admin-muted">默认演示账号：admin / admin123</p>
              </div>
            </div>
            <div class="admin-form-grid">
              <label class="admin-field">
                <span class="admin-label">用户名</span>
                <input
                  v-model="session.loginForm.username"
                  placeholder="请输入管理员用户名"
                  class="admin-input"
                />
              </label>
              <label class="admin-field">
                <span class="admin-label">密码</span>
                <input
                  v-model="session.loginForm.password"
                  type="password"
                  placeholder="请输入管理员密码"
                  class="admin-input"
                  @keyup.enter="handleLogin"
                />
              </label>
              <label class="admin-field">
                <span class="admin-label">后台 API</span>
                <input
                  :value="session.apiBase.value"
                  class="admin-input"
                  @input="session.updateApiBase(($event.target as HTMLInputElement).value)"
                />
              </label>
              <button class="admin-button-primary w-full" :disabled="session.authBusy.value" @click="handleLogin">
                {{ session.authBusy.value ? '登录中…' : '登录后台' }}
              </button>
            </div>
            <p v-if="session.authError.value" class="admin-alert admin-alert-danger">{{ session.authError.value }}</p>
          </div>

          <div class="admin-card">
            <div class="admin-card-head">
              <div>
                <h2 class="admin-subtitle">本页能力</h2>
                <p class="admin-muted">一站式完成资产、订阅、账单与 AI 策略管理。</p>
              </div>
            </div>
            <div class="grid gap-3 md:grid-cols-2">
              <div class="admin-stat-tile">
                <p class="admin-stat-label">企业账号</p>
                <p class="admin-stat-note">席位、状态与子账号总览</p>
              </div>
              <div class="admin-stat-tile">
                <p class="admin-stat-label">资产授权</p>
                <p class="admin-stat-note">子账号与资产范围同步</p>
              </div>
              <div class="admin-stat-tile">
                <p class="admin-stat-label">账单回款</p>
                <p class="admin-stat-note">支付提供方、回款与状态管理</p>
              </div>
              <div class="admin-stat-tile">
                <p class="admin-stat-label">AI 策略</p>
                <p class="admin-stat-note">方案目录、Token 价格与端点同步</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
