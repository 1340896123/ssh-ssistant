import { createRouter, createWebHistory } from 'vue-router'
import AdminLayout from '../layouts/AdminLayout.vue'
import LoginPage from '../pages/LoginPage.vue'
import OverviewPage from '../pages/OverviewPage.vue'
import AiSubscriptionsPage from '../pages/AiSubscriptionsPage.vue'
import BillingPage from '../pages/BillingPage.vue'
import AiUsagePage from '../pages/AiUsagePage.vue'
import PersonalAccountsPage from '../pages/PersonalAccountsPage.vue'
import GlobalStrategyPage from '../pages/GlobalStrategyPage.vue'
import { clearAdminSession } from '../composables/useAdminSession'

function getStoredNumber(key: string) {
  return Number(localStorage.getItem(key) || 0)
}

function hasValidSession() {
  const authToken = localStorage.getItem('admin-auth-token') || ''
  const refreshToken = localStorage.getItem('admin-refresh-token') || ''
  const authExpiresAt = getStoredNumber('admin-auth-expires-at')
  const refreshExpiresAt = getStoredNumber('admin-refresh-expires-at')
  const now = Date.now()

  if (authToken && authExpiresAt > now) return true
  if (refreshToken && refreshExpiresAt > now) return true
  if (authToken || refreshToken) {
    clearAdminSession()
  }
  return false
}

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/overview',
    },
    {
      path: '/login',
      name: 'login',
      component: LoginPage,
      meta: { public: true },
    },
    {
      path: '/',
      component: AdminLayout,
      children: [
        { path: 'overview', name: 'overview', component: OverviewPage },
        { path: 'ai-subscriptions', name: 'ai-subscriptions', component: AiSubscriptionsPage },
        { path: 'billing', name: 'billing', component: BillingPage },
        { path: 'ai-usage', name: 'ai-usage', component: AiUsagePage },
        { path: 'personal-accounts', name: 'personal-accounts', component: PersonalAccountsPage },
        { path: 'global-strategy', name: 'global-strategy', component: GlobalStrategyPage },
      ],
    },
  ],
})

router.beforeEach((to) => {
  const authenticated = hasValidSession()

  if (to.meta.public) {
    if (authenticated && to.name === 'login') {
      return { name: 'overview' }
    }
    return true
  }

  if (!authenticated) {
    return {
      name: 'login',
      query: to.fullPath === '/login' ? undefined : { redirect: to.fullPath },
    }
  }

  return true
})

export default router
