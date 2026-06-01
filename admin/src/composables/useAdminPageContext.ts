import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { adminPageMetaMap } from '../constants/adminNavigation'

export function useAdminPageContext() {
  const route = useRoute()

  return computed(() => {
    const key = String(route.name || 'overview')
    return adminPageMetaMap[key] ?? {
      title: '后台管理',
      description: '统一管理企业账号、订阅策略、账单与 AI 资源分配。',
    }
  })
}
