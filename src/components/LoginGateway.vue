<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useSettingsStore } from "../stores/settings";
import { useAssetStore } from "../stores/assets";
import { useI18n } from "../composables/useI18n";

const emit = defineEmits<{
  (e: "authenticated"): void;
}>();

const settingsStore = useSettingsStore();
const assetStore = useAssetStore();
const { t } = useI18n();

const isSubmitting = ref(false);
const errorMessage = ref("");

const form = reactive({
  mode: settingsStore.account.mode,
  identifier:
    settingsStore.account.email ||
    settingsStore.account.userId ||
    settingsStore.account.subAccountId ||
    "",
  secret: "",
  endpointUrl: settingsStore.sync.endpointUrl || "http://localhost:5047",
  organizationScope: settingsStore.sync.organizationScope || "",
});

watch(
  () => settingsStore.account.mode,
  (mode) => {
    form.mode = mode;
  },
);

const isLocalMode = computed(() => form.mode === "local");

const modeDescription = computed(() => {
  if (form.mode === "enterpriseSubAccount") {
    return "使用企业子账号登录后，仅同步被授权的资产范围。";
  }
  if (form.mode === "personal") {
    return "使用个人账号登录后，会同步个人资产与个人 AI 订阅设置。";
  }
  return "本地模式不会强制云登录，但仍可进入工作台使用离线资产。";
});

const subscriptionSummary = computed(() => settingsStore.activeSubscriptionSummary());
const currentInvoice = computed(() => settingsStore.ai.subscriptionSnapshot?.currentInvoice ?? null);

async function submit() {
  isSubmitting.value = true;
  errorMessage.value = "";

  try {
    await settingsStore.saveSettings({
      account: {
        ...settingsStore.account,
        mode: form.mode,
        email: form.mode === "personal" ? form.identifier : settingsStore.account.email,
        userId: form.mode === "personal" ? form.identifier : settingsStore.account.userId,
        subAccountId:
          form.mode === "enterpriseSubAccount"
            ? form.identifier
            : settingsStore.account.subAccountId,
      },
      sync: {
        ...settingsStore.sync,
        endpointUrl: form.endpointUrl,
        organizationScope: form.organizationScope,
      },
    });

    if (!isLocalMode.value) {
      await settingsStore.loginToCloud(form.secret);
      await settingsStore.pullCloudState();
      await assetStore.pullAssetsFromCloud(
        settingsStore.sync.endpointUrl || form.endpointUrl,
        settingsStore.account.mode,
        settingsStore.account.userId ||
          settingsStore.account.subAccountId ||
          "local-workspace",
        settingsStore.account.accessToken || "",
      );
    } else {
      await assetStore.loadAssets();
    }

    emit("authenticated");
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  } finally {
    isSubmitting.value = false;
  }
}

async function enterLocalMode() {
  await settingsStore.logoutFromCloud()
  await assetStore.loadAssets()
  emit("authenticated")
}
</script>

<template>
  <div class="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top_left,_#d7eadf,_#f7f3eb_40%,_#eadfd0_100%)] px-6 py-10">
    <div class="grid w-full max-w-6xl gap-8 lg:grid-cols-[1.1fr_0.9fr]">
      <section class="rounded-[32px] border border-white/70 bg-white/80 p-8 shadow-[0_28px_80px_rgba(67,52,30,0.14)] backdrop-blur">
        <p class="text-sm font-semibold uppercase tracking-[0.32em] text-emerald-700">SSH Assistant</p>
        <h1 class="mt-4 text-4xl font-black tracking-tight text-slate-900 sm:text-5xl">
          三种账号模式统一登录
        </h1>
        <p class="mt-5 max-w-2xl text-base leading-8 text-slate-600">
          个人账号、企业子账号、本地模式共用一套入口。登录后自动拉取资产中心、同步设置，并继续沿用后台下发的 AI 订阅与自定义端点策略。
        </p>

        <div class="mt-8 grid gap-4 md:grid-cols-3">
          <article class="rounded-3xl border border-slate-200 bg-slate-50/80 p-5">
            <p class="text-sm font-semibold text-slate-500">个人账号</p>
            <p class="mt-3 text-sm leading-7 text-slate-700">同步个人资产、AI 订阅状态与个人自定义端点。</p>
          </article>
          <article class="rounded-3xl border border-slate-200 bg-slate-50/80 p-5">
            <p class="text-sm font-semibold text-slate-500">企业子账号</p>
            <p class="mt-3 text-sm leading-7 text-slate-700">只拉取企业后台分配的资产范围，并继承企业订阅策略。</p>
          </article>
          <article class="rounded-3xl border border-slate-200 bg-slate-50/80 p-5">
            <p class="text-sm font-semibold text-slate-500">本地模式</p>
            <p class="mt-3 text-sm leading-7 text-slate-700">无需云登录即可进入桌面工作台，保留离线资产体验。</p>
          </article>
        </div>

        <div class="mt-8 rounded-3xl border border-slate-200 bg-slate-50/80 p-5">
          <p class="text-sm font-semibold text-slate-500">当前订阅状态</p>
          <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-600">
            <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ subscriptionSummary.label }}</span>
            <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ subscriptionSummary.scope }}</span>
            <span class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">{{ subscriptionSummary.billing }}</span>
            <span v-if="subscriptionSummary.renewal" class="rounded-full bg-white px-3 py-1 ring-1 ring-slate-200">renew {{ subscriptionSummary.renewal }}</span>
          </div>
          <p v-if="currentInvoice" class="mt-3 text-xs text-slate-500">
            当前账期 {{ currentInvoice.billingMonth }} · {{ currentInvoice.status }} · remaining {{ currentInvoice.remainingAmount }} {{ currentInvoice.currency }}
          </p>
        </div>
      </section>

      <section class="rounded-[32px] border border-white/70 bg-white/85 p-8 shadow-[0_28px_80px_rgba(67,52,30,0.14)] backdrop-blur">
        <div>
          <h2 class="text-2xl font-black text-slate-900">登录工作台</h2>
          <p class="mt-2 text-sm text-slate-500">{{ modeDescription }}</p>
        </div>

        <div class="mt-6 space-y-5">
          <div>
            <label class="mb-2 block text-sm font-medium text-slate-700">账号模式</label>
            <select
              v-model="form.mode"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
            >
              <option value="personal">{{ t("settings.accountModes.personal") }}</option>
              <option value="enterpriseSubAccount">{{ t("settings.accountModes.enterpriseSubAccount") }}</option>
              <option value="local">{{ t("settings.accountModes.local") }}</option>
            </select>
          </div>

          <div v-if="!isLocalMode">
            <label class="mb-2 block text-sm font-medium text-slate-700">
              {{ form.mode === "personal" ? "账号标识（邮箱或用户 ID）" : "子账号标识（邮箱或子账号 ID）" }}
            </label>
            <input
              v-model="form.identifier"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
              placeholder="user@example.com"
            />
          </div>

          <div v-if="!isLocalMode">
            <label class="mb-2 block text-sm font-medium text-slate-700">登录密钥</label>
            <input
              v-model="form.secret"
              type="password"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
              placeholder="temporary login secret"
            />
          </div>

          <div>
            <label class="mb-2 block text-sm font-medium text-slate-700">后台地址</label>
            <input
              v-model="form.endpointUrl"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
              placeholder="http://localhost:5047"
            />
          </div>

          <div>
            <label class="mb-2 block text-sm font-medium text-slate-700">组织范围</label>
            <input
              v-model="form.organizationScope"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
              placeholder="default-org"
            />
          </div>

          <button
            class="w-full rounded-2xl bg-slate-900 px-5 py-3 text-sm font-semibold text-white transition hover:bg-slate-700 disabled:opacity-60"
            :disabled="isSubmitting"
            @click="submit"
          >
            {{ isSubmitting ? "正在进入工作台..." : "进入工作台" }}
          </button>

          <button
            v-if="!isLocalMode"
            class="w-full rounded-2xl border border-slate-300 bg-white px-5 py-3 text-sm font-semibold text-slate-700 transition hover:bg-slate-50"
            @click="enterLocalMode"
          >
            切换为本地模式
          </button>

          <p v-if="errorMessage" class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
            {{ errorMessage }}
          </p>
        </div>
      </section>
    </div>
  </div>
</template>
