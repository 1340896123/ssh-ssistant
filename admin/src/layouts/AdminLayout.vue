<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { RouterLink, RouterView, useRoute, useRouter } from 'vue-router'
import AdminNavIcon from '../components/AdminNavIcon.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'
import { useAdminPageContext } from '../composables/useAdminPageContext'
import { useAdminSession } from '../composables/useAdminSession'
import { adminNavigationItems } from '../constants/adminNavigation'

const router = useRouter()
const route = useRoute()
const pageMeta = useAdminPageContext()
const session = useAdminSession()
const dashboardState = useAdminDashboard()

const refreshExpiryText = computed(() =>
  session.refreshExpiresAt.value ? new Date(session.refreshExpiresAt.value).toLocaleString() : '-',
)

const navItems = adminNavigationItems

async function refreshDashboard() {
  const valid = await session.ensureValidSession()
  if (!valid) {
    await router.replace({ name: 'login' })
    return
  }

  try {
    await dashboardState.loadDashboard(session.apiBase.value, session.authToken.value)
  } catch {
    if (!session.isAuthenticated.value) {
      await router.replace({ name: 'login' })
    }
  }
}

async function handleLogout() {
  await session.logout()
}

watch(
  () => session.isAuthenticated.value,
  async (authenticated) => {
    if (!authenticated && route.name !== 'login') {
      await router.replace({ name: 'login' })
    }
  },
)

onMounted(() => {
  void refreshDashboard()
})
</script>

<template>
  <div class="admin-shell">
    <div class="admin-app">
      <aside class="admin-sidebar">
        <div class="admin-sidebar-brand">
          <div class="admin-brand-mark">SA</div>
          <div>
            <p class="admin-sidebar-title">SSH Assistant</p>
            <p class="admin-sidebar-subtitle">Admin Console</p>
          </div>
        </div>

        <nav class="admin-sidebar-nav">
          <RouterLink
            v-for="item in navItems"
            :key="item.routeName"
            :to="{ name: item.routeName }"
            class="admin-nav-item"
            :class="{ 'is-active': route.name === item.routeName }"
          >
            <div class="admin-nav-row">
              <span class="admin-nav-icon" :data-icon="item.icon">
                <AdminNavIcon :icon="item.icon" />
              </span>
              <span class="admin-nav-label">{{ item.title }}</span>
            </div>
            <span class="admin-nav-desc">{{ item.description }}</span>
          </RouterLink>
        </nav>

        <div class="admin-sidebar-footer">
          <p class="admin-sidebar-caption">当前管理员</p>
          <p class="admin-sidebar-user">{{ session.adminUsername.value || '-' }}</p>
          <p class="admin-sidebar-meta">刷新令牌有效期至 {{ refreshExpiryText }}</p>
        </div>
      </aside>

      <main class="admin-main">
        <header class="admin-topbar">
          <div>
            <div class="admin-eyebrow">SSH Assistant Admin</div>
            <h1 class="admin-page-title">{{ pageMeta.title }}</h1>
            <p class="admin-page-subtitle">{{ pageMeta.description }}</p>
          </div>

          <div class="admin-topbar-actions">
            <label class="admin-inline-field admin-inline-field-wide">
              <span class="admin-inline-label">后台 API</span>
              <input
                :value="session.apiBase.value"
                class="admin-input"
                @input="session.updateApiBase(($event.target as HTMLInputElement).value)"
              />
            </label>
            <button class="admin-button-primary" :disabled="dashboardState.loading.value" @click="refreshDashboard">
              刷新数据
            </button>
            <button class="admin-button-secondary" @click="handleLogout">退出登录</button>
          </div>
        </header>

        <div class="admin-mobile-nav">
          <RouterLink
            v-for="item in navItems"
            :key="item.routeName"
            :to="{ name: item.routeName }"
            class="admin-mobile-chip"
            :class="{ 'is-active': route.name === item.routeName }"
          >
            {{ item.title }}
          </RouterLink>
        </div>

        <div class="admin-feedback-stack">
          <p v-if="dashboardState.notice.value" class="admin-alert admin-alert-success">{{ dashboardState.notice.value }}</p>
          <p v-if="dashboardState.error.value" class="admin-alert admin-alert-danger">{{ dashboardState.error.value }}</p>
          <p v-if="dashboardState.loading.value" class="admin-alert admin-alert-info">正在同步后台数据，请稍候…</p>
        </div>

        <RouterView />
      </main>
    </div>
  </div>
</template>
