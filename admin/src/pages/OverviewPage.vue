<script setup lang="ts">
import AdminPageIntro from '../components/AdminPageIntro.vue'
import { useAdminDashboard } from '../composables/useAdminDashboard'

const dashboardState = useAdminDashboard()
</script>

<template>
  <section class="admin-section">
    <AdminPageIntro title="概览" description="查看当前后台的核心指标、登录态与系统提示。" />

    <div class="admin-summary-grid">
      <article v-for="card in dashboardState.summaryCards.value" :key="card.label" class="admin-summary-card">
        <p class="admin-summary-label">{{ card.label }}</p>
        <p class="admin-summary-value">{{ card.value }}</p>
        <p class="admin-summary-note">{{ card.note }}</p>
      </article>
    </div>

    <div v-if="!dashboardState.dashboard.value && !dashboardState.loading.value" class="admin-empty-state mt-4">
      当前还没有可展示的数据，请检查后台 API 地址或重新登录后刷新。
    </div>
  </section>
</template>
