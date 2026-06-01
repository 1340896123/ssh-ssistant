import { computed, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { adminLogin, adminRefresh } from '../services/adminApi'
import { DEFAULT_ADMIN_API_BASE, type LoginFormState } from '../types/admin'
import { parseApiError } from '../utils/adminFormat'

const STORAGE_KEYS = {
  apiBase: 'admin-api-base',
  authToken: 'admin-auth-token',
  refreshToken: 'admin-refresh-token',
  username: 'admin-username',
  authExpiresAt: 'admin-auth-expires-at',
  refreshExpiresAt: 'admin-refresh-expires-at',
}

const apiBase = ref(localStorage.getItem(STORAGE_KEYS.apiBase) || DEFAULT_ADMIN_API_BASE)
const authToken = ref(localStorage.getItem(STORAGE_KEYS.authToken) || '')
const refreshToken = ref(localStorage.getItem(STORAGE_KEYS.refreshToken) || '')
const adminUsername = ref(localStorage.getItem(STORAGE_KEYS.username) || '')
const authExpiresAt = ref(Number(localStorage.getItem(STORAGE_KEYS.authExpiresAt) || 0))
const refreshExpiresAt = ref(Number(localStorage.getItem(STORAGE_KEYS.refreshExpiresAt) || 0))
const loginForm = reactive<LoginFormState>({
  username: adminUsername.value || 'admin',
  password: 'admin123',
})
const authBusy = ref(false)
const authError = ref('')

function persistSession() {
  localStorage.setItem(STORAGE_KEYS.apiBase, apiBase.value)
  localStorage.setItem(STORAGE_KEYS.authToken, authToken.value)
  localStorage.setItem(STORAGE_KEYS.refreshToken, refreshToken.value)
  localStorage.setItem(STORAGE_KEYS.username, adminUsername.value)
  localStorage.setItem(STORAGE_KEYS.authExpiresAt, String(authExpiresAt.value))
  localStorage.setItem(STORAGE_KEYS.refreshExpiresAt, String(refreshExpiresAt.value))
}

function applyAuthPayload(payload: {
  token: string
  refreshToken: string
  username: string
  expiresAt: string
  refreshExpiresAt: string
}) {
  authToken.value = payload.token
  refreshToken.value = payload.refreshToken
  adminUsername.value = payload.username
  authExpiresAt.value = Date.parse(payload.expiresAt)
  refreshExpiresAt.value = Date.parse(payload.refreshExpiresAt)
  loginForm.username = payload.username
  persistSession()
}

function clearSessionStorage() {
  localStorage.removeItem(STORAGE_KEYS.authToken)
  localStorage.removeItem(STORAGE_KEYS.refreshToken)
  localStorage.removeItem(STORAGE_KEYS.username)
  localStorage.removeItem(STORAGE_KEYS.authExpiresAt)
  localStorage.removeItem(STORAGE_KEYS.refreshExpiresAt)
}

export function clearAdminSession() {
  authToken.value = ''
  refreshToken.value = ''
  adminUsername.value = ''
  authExpiresAt.value = 0
  refreshExpiresAt.value = 0
  clearSessionStorage()
}

export function useAdminSession() {
  const router = useRouter()

  async function login() {
    authBusy.value = true
    authError.value = ''
    try {
      const payload = await adminLogin(apiBase.value, loginForm)
      applyAuthPayload(payload)
      return true
    } catch (error) {
      authError.value = parseApiError(error)
      return false
    } finally {
      authBusy.value = false
    }
  }

  async function refreshSession() {
    if (!refreshToken.value) {
      clearAdminSession()
      return false
    }

    try {
      const payload = await adminRefresh(apiBase.value, refreshToken.value)
      applyAuthPayload(payload)
      return true
    } catch {
      clearAdminSession()
      return false
    }
  }

  async function ensureValidSession() {
    if (authToken.value && authExpiresAt.value > Date.now()) return true
    if (refreshToken.value && refreshExpiresAt.value > Date.now()) {
      return refreshSession()
    }

    clearAdminSession()
    return false
  }

  async function logout(redirect = true) {
    clearAdminSession()
    authError.value = ''
    if (redirect) {
      await router.replace({ name: 'login' })
    }
  }

  function updateApiBase(value: string) {
    apiBase.value = value.trim()
    localStorage.setItem(STORAGE_KEYS.apiBase, apiBase.value)
  }

  return {
    apiBase,
    authBusy,
    authError,
    authExpiresAt,
    authToken,
    adminUsername,
    ensureValidSession,
    isAuthenticated: computed(() => Boolean(authToken.value)),
    login,
    loginForm,
    logout,
    refreshExpiresAt,
    refreshSession,
    refreshToken,
    updateApiBase,
  }
}
