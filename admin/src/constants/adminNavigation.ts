import type { AdminPageMeta, AdminRouteName } from '../types/admin'

export interface AdminNavigationItem extends AdminPageMeta {
  routeName: Exclude<AdminRouteName, 'login'>
  path: string
  icon: string
}

export const adminNavigationItems: AdminNavigationItem[] = [
  {
    routeName: 'overview',
    path: '/overview',
    title: '概览',
    description: '关键指标与运行状态',
    icon: 'grid',
  },
  {
    routeName: 'ai-subscriptions',
    path: '/ai-subscriptions',
    title: 'AI 订阅',
    description: '方案目录与席位绑定',
    icon: 'zap',
  },
  {
    routeName: 'billing',
    path: '/billing',
    title: '账单中心',
    description: '回款、支付与账期',
    icon: 'card',
  },
  {
    routeName: 'ai-usage',
    path: '/ai-usage',
    title: 'AI 用量',
    description: '请求、Token 与成本',
    icon: 'chart',
  },
  {
    routeName: 'personal-accounts',
    path: '/personal-accounts',
    title: '个人账号',
    description: '个人账号与订阅',
    icon: 'user',
  },
  {
    routeName: 'global-strategy',
    path: '/global-strategy',
    title: '全局策略',
    description: 'AI 策略与端点同步',
    icon: 'settings',
  },
]

export const adminPageMetaMap = adminNavigationItems.reduce<Record<string, AdminPageMeta>>((acc, item) => {
  acc[item.routeName] = {
    title: item.title,
    description: item.description,
  }
  return acc
}, {})
