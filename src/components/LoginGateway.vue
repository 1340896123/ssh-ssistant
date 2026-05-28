<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useSettingsStore } from "../stores/settings";
import type { AccountMode } from "../types";
import { useI18n } from "../composables/useI18n";
import { cloudService } from "../services";

const emit = defineEmits<{
  (e: "authenticated"): void;
}>();

const settingsStore = useSettingsStore();
const { t } = useI18n();

const isSubmitting = ref(false);
const errorMessage = ref("");
const registrationNotice = ref("");
const personalAuthView = ref<"login" | "register">("login");

const form = reactive<{
  mode: AccountMode;
  identifier: string;
  secret: string;
  displayName: string;
  endpointUrl: string;
  enterpriseId: string;
  enterpriseName: string;
  organizationScope: string;
}>({
  mode: settingsStore.account.mode === "local" ? "personal" : settingsStore.account.mode,
  identifier:
    settingsStore.account.email ||
    settingsStore.account.userId ||
    settingsStore.account.subAccountId ||
    "",
  secret: "",
  displayName: settingsStore.account.displayName || "",
  endpointUrl: settingsStore.sync.endpointUrl || "http://localhost:5047",
  enterpriseId: settingsStore.account.enterpriseId || "",
  enterpriseName: settingsStore.account.enterpriseName || "",
  organizationScope: settingsStore.sync.organizationScope || "",
});

watch(
  () => settingsStore.account.mode,
  (mode) => {
    form.mode = mode === "local" ? "personal" : mode;
    if (form.mode !== "personal") {
      personalAuthView.value = "login";
      registrationNotice.value = "";
    }
    if (mode !== "enterpriseSubAccount") {
      form.enterpriseId = "";
      form.enterpriseName = "";
    } else {
      form.enterpriseId = settingsStore.account.enterpriseId || form.enterpriseId;
      form.enterpriseName = settingsStore.account.enterpriseName || form.enterpriseName;
    }
  },
);

const isLocalMode = computed(() => form.mode === "local");
const isPersonalMode = computed(() => form.mode === "personal");
const isRegistering = computed(
  () => isPersonalMode.value && personalAuthView.value === "register",
);

const modeDescription = computed(() => {
  if (form.mode === "enterpriseSubAccount") {
    return t("loginGateway.modeDescriptions.enterpriseSubAccount");
  }
  if (form.mode === "personal") {
    return isRegistering.value
      ? t("loginGateway.modeDescriptions.personalRegister")
      : t("loginGateway.modeDescriptions.personalLogin");
  }
  return t("loginGateway.modeDescriptions.local");
});

const subscriptionSummary = computed(() => {
  if (settingsStore.account.mode === "local" && !settingsStore.account.accessToken) {
    return {
      label: "Free",
      scope: "global",
      billing: "USD 0/seat",
      renewal: null,
    };
  }
  return settingsStore.activeSubscriptionSummary();
});

const currentInvoice = computed(() => {
  if (settingsStore.account.mode === "local" && !settingsStore.account.accessToken) {
    return null;
  }
  return settingsStore.ai.subscriptionSnapshot?.currentInvoice ?? null;
});

function mapGatewayErrorMessage(error: unknown) {
  const raw = error instanceof Error ? error.message : String(error);
  const normalized = raw.toLowerCase();

  if (normalized.includes("already registered")) {
    return t("loginGateway.errors.emailRegistered");
  }
  if (
    normalized.includes("is required") ||
    normalized.includes("format is invalid") ||
    normalized.includes("at least 6 characters")
  ) {
    return t("loginGateway.errors.invalidParameters");
  }
  if (normalized.includes("status 400")) {
    return t("loginGateway.errors.invalidParameters");
  }
  if (normalized.includes("status 409")) {
    return t("loginGateway.errors.emailRegistered");
  }
  if (
    normalized.includes("failed to fetch") ||
    normalized.includes("networkerror") ||
    normalized.includes("status 500") ||
    normalized.includes("status 502") ||
    normalized.includes("status 503") ||
    normalized.includes("status 504")
  ) {
    return t("loginGateway.errors.serviceUnavailable");
  }

  return raw;
}

async function submit() {
  isSubmitting.value = true;
  errorMessage.value = "";
  registrationNotice.value = "";

  try {
    const previousMode = settingsStore.account.mode;
    const shouldPreserveLocalSnapshot =
      previousMode === "local" &&
      form.mode !== "local" &&
      !settingsStore.isLoginGatewayRequired();
    if (shouldPreserveLocalSnapshot) {
      await settingsStore.saveCurrentLocalWorkspaceSnapshot().catch(() => undefined);
    }
    if (isLocalMode.value) {
      await settingsStore.logoutFromCloud({
        nextMode: "local",
        preserveIdentity: false,
      });
    } else {
      settingsStore.resetCloudManagedAiState();

      await settingsStore.saveSettings({
        account: {
          mode: form.mode,
          displayName:
            form.mode === "enterpriseSubAccount"
              ? form.enterpriseName.trim() ||
                form.identifier.trim() ||
                settingsStore.account.displayName ||
                "Enterprise Sub-Account"
              : isRegistering.value
                ? form.displayName.trim() ||
                  form.identifier.trim() ||
                  settingsStore.account.displayName ||
                  "Personal Account"
                : form.identifier.trim() ||
                  settingsStore.account.displayName ||
                  "Personal Account",
          email: form.mode === "personal" ? form.identifier.trim() || null : null,
          userId: form.mode === "personal" ? form.identifier.trim() || null : null,
          enterpriseId:
            form.mode === "enterpriseSubAccount"
              ? form.enterpriseId.trim() || settingsStore.account.enterpriseId || null
              : null,
          enterpriseName:
            form.mode === "enterpriseSubAccount"
              ? form.enterpriseName.trim() || settingsStore.account.enterpriseName || null
              : null,
          subAccountId:
            form.mode === "enterpriseSubAccount"
              ? form.identifier.trim() || null
              : null,
          accessToken: null,
          refreshToken: null,
          expiresAt: null,
          refreshExpiresAt: null,
        },
        sync: {
          ...settingsStore.sync,
          endpointUrl: form.endpointUrl,
          organizationScope: form.organizationScope,
        },
      });

      if (isRegistering.value) {
        const response = await cloudService.register(form.endpointUrl, {
          email: form.identifier.trim(),
          displayName: form.displayName.trim(),
          password: form.secret,
        });
        await settingsStore.applyCloudLoginResponse(response);
        registrationNotice.value = t("loginGateway.register.autoLoginSuccess");
      } else {
        await settingsStore.loginToCloud(form.secret);
      }
    }

    settingsStore.clearLoginGatewayRequired();
    emit("authenticated");
  } catch (error) {
    errorMessage.value = mapGatewayErrorMessage(error);
  } finally {
    isSubmitting.value = false;
  }
}

async function enterLocalMode() {
  form.mode = "local";
  await submit();
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
          <h2 class="text-2xl font-black text-slate-900">{{ t("loginGateway.title") }}</h2>
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

          <div
            v-if="isPersonalMode"
            class="inline-flex rounded-2xl bg-slate-100 p-1 text-sm font-semibold text-slate-600"
          >
            <button
              class="rounded-xl px-4 py-2 transition"
              :class="personalAuthView === 'login' ? 'bg-white text-slate-900 shadow-sm' : ''"
              @click="personalAuthView = 'login'"
            >
              {{ t("loginGateway.tabs.login") }}
            </button>
            <button
              class="rounded-xl px-4 py-2 transition"
              :class="personalAuthView === 'register' ? 'bg-white text-slate-900 shadow-sm' : ''"
              @click="personalAuthView = 'register'"
            >
              {{ t("loginGateway.tabs.register") }}
            </button>
          </div>

          <div v-if="!isLocalMode">
            <label class="mb-2 block text-sm font-medium text-slate-700">
              {{
                form.mode === "personal"
                  ? isRegistering
                    ? t("loginGateway.fields.email")
                    : t("loginGateway.fields.personalIdentifier")
                  : t("loginGateway.fields.enterpriseIdentifier")
              }}
            </label>
            <input
              v-model="form.identifier"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
              placeholder="user@example.com"
            />
          </div>

          <div v-if="isRegistering">
            <label class="mb-2 block text-sm font-medium text-slate-700">
              {{ t("loginGateway.fields.displayName") }}
            </label>
            <input
              v-model="form.displayName"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
              :placeholder="t('loginGateway.placeholders.displayName')"
            />
          </div>

          <template v-if="form.mode === 'enterpriseSubAccount'">
            <div>
              <label class="mb-2 block text-sm font-medium text-slate-700">企业 ID</label>
              <input
                v-model="form.enterpriseId"
                class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
                placeholder="ent-001"
              />
            </div>

            <div>
              <label class="mb-2 block text-sm font-medium text-slate-700">企业名称</label>
              <input
                v-model="form.enterpriseName"
                class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
                placeholder="Enterprise Name"
              />
            </div>
          </template>

          <div v-if="!isLocalMode">
            <label class="mb-2 block text-sm font-medium text-slate-700">
              {{ isRegistering ? t("loginGateway.fields.password") : t("loginGateway.fields.secret") }}
            </label>
            <input
              v-model="form.secret"
              type="password"
              class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none"
              :placeholder="
                isRegistering
                  ? t('loginGateway.placeholders.password')
                  : 'temporary login secret'
              "
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
            {{
              isSubmitting
                ? isRegistering
                  ? t("loginGateway.register.submitting")
                  : t("loginGateway.login.submitting")
                : isRegistering
                  ? t("loginGateway.register.submit")
                  : t("loginGateway.login.submit")
            }}
          </button>

          <button
            v-if="!isLocalMode"
            class="w-full rounded-2xl border border-slate-300 bg-white px-5 py-3 text-sm font-semibold text-slate-700 transition hover:bg-slate-50"
            @click="enterLocalMode"
          >
            切换为本地模式
          </button>

          <p
            v-if="registrationNotice"
            class="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700"
          >
            {{ registrationNotice }}
          </p>

          <p v-if="errorMessage" class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
            {{ errorMessage }}
          </p>
        </div>
      </section>
    </div>
  </div>
</template>
