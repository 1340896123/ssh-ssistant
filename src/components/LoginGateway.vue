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
  <div class="flex min-h-screen items-center justify-center bg-bg-primary px-6 py-10">
    <div class="grid w-full max-w-6xl gap-8 lg:grid-cols-[1.1fr_0.9fr]">
      <section class="rounded-[32px] border border-border-primary bg-bg-secondary p-8 shadow-[0_28px_80px_rgba(0,0,0,0.18)]">
        <p class="text-sm font-semibold uppercase tracking-[0.32em] text-accent">{{ t("app.title") }}</p>
        <h1 class="mt-4 text-4xl font-black tracking-tight text-text-primary sm:text-5xl">
          {{ t("loginGateway.heroTitle") }}
        </h1>
        <p class="mt-5 max-w-2xl text-base leading-8 text-text-secondary">
          {{ t("loginGateway.heroDescription") }}
        </p>

        <div class="mt-8 grid gap-4 md:grid-cols-3">
          <article class="rounded-3xl border border-border-primary bg-bg-tertiary p-5">
            <p class="text-sm font-semibold text-text-secondary">{{ t("settings.accountModes.personal") }}</p>
            <p class="mt-3 text-sm leading-7 text-text-primary">{{ t("loginGateway.modeCards.personal") }}</p>
          </article>
          <article class="rounded-3xl border border-border-primary bg-bg-tertiary p-5">
            <p class="text-sm font-semibold text-text-secondary">{{ t("settings.accountModes.enterpriseSubAccount") }}</p>
            <p class="mt-3 text-sm leading-7 text-text-primary">{{ t("loginGateway.modeCards.enterpriseSubAccount") }}</p>
          </article>
          <article class="rounded-3xl border border-border-primary bg-bg-tertiary p-5">
            <p class="text-sm font-semibold text-text-secondary">{{ t("settings.accountModes.local") }}</p>
            <p class="mt-3 text-sm leading-7 text-text-primary">{{ t("loginGateway.modeCards.local") }}</p>
          </article>
        </div>
      </section>

      <section class="rounded-[32px] border border-border-primary bg-bg-secondary p-8 shadow-[0_28px_80px_rgba(0,0,0,0.18)]">
        <div>
          <h2 class="text-2xl font-black text-text-primary">{{ t("loginGateway.title") }}</h2>
          <p class="mt-2 text-sm text-text-secondary">{{ modeDescription }}</p>
        </div>

        <form class="mt-6 space-y-5" @submit.prevent="submit">
          <div>
            <label class="mb-2 block text-sm font-medium text-text-primary">{{ t("settings.accountMode") }}</label>
            <select
              v-model="form.mode"
              class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
            >
              <option value="personal">{{ t("settings.accountModes.personal") }}</option>
              <option value="enterpriseSubAccount">{{ t("settings.accountModes.enterpriseSubAccount") }}</option>
              <option value="local">{{ t("settings.accountModes.local") }}</option>
            </select>
          </div>

          <div
            v-if="isPersonalMode"
            class="inline-flex rounded-2xl bg-bg-tertiary p-1 text-sm font-semibold text-text-secondary"
          >
            <button
              data-testid="login-gateway-tab-login"
              type="button"
              class="rounded-xl px-4 py-2 transition"
              :class="personalAuthView === 'login' ? 'bg-bg-elevated text-text-primary shadow-sm' : ''"
              @click="personalAuthView = 'login'"
            >
              {{ t("loginGateway.tabs.login") }}
            </button>
            <button
              data-testid="login-gateway-tab-register"
              type="button"
              class="rounded-xl px-4 py-2 transition"
              :class="personalAuthView === 'register' ? 'bg-bg-elevated text-text-primary shadow-sm' : ''"
              @click="personalAuthView = 'register'"
            >
              {{ t("loginGateway.tabs.register") }}
            </button>
          </div>

          <div v-if="!isLocalMode">
            <label class="mb-2 block text-sm font-medium text-text-primary">
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
              class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
              :placeholder="t('loginGateway.placeholders.identifier')"
            />
          </div>

          <div v-if="isRegistering">
            <label class="mb-2 block text-sm font-medium text-text-primary">
              {{ t("loginGateway.fields.displayName") }}
            </label>
            <input
              v-model="form.displayName"
              class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
              :placeholder="t('loginGateway.placeholders.displayName')"
            />
          </div>

          <template v-if="form.mode === 'enterpriseSubAccount'">
            <div>
              <label class="mb-2 block text-sm font-medium text-text-primary">{{ t("settings.enterpriseId") }}</label>
              <input
                v-model="form.enterpriseId"
                class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
                :placeholder="t('loginGateway.placeholders.enterpriseId')"
              />
            </div>

            <div>
              <label class="mb-2 block text-sm font-medium text-text-primary">{{ t("settings.enterpriseName") }}</label>
              <input
                v-model="form.enterpriseName"
                class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
                :placeholder="t('loginGateway.placeholders.enterpriseName')"
              />
            </div>
          </template>

          <div v-if="!isLocalMode">
            <label class="mb-2 block text-sm font-medium text-text-primary">
              {{ isRegistering ? t("loginGateway.fields.password") : t("loginGateway.fields.secret") }}
            </label>
            <input
              v-model="form.secret"
              type="password"
              class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
              :placeholder="
                isRegistering
                  ? t('loginGateway.placeholders.password')
                  : t('loginGateway.placeholders.secret')
              "
            />
          </div>

          <div>
            <label class="mb-2 block text-sm font-medium text-text-primary">{{ t("settings.cloudSyncEndpoint") }}</label>
            <input
              v-model="form.endpointUrl"
              class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
              placeholder="http://localhost:5047"
            />
          </div>

          <div>
            <label class="mb-2 block text-sm font-medium text-text-primary">{{ t("settings.organizationScope") }}</label>
            <input
              v-model="form.organizationScope"
              class="w-full rounded-2xl border border-border-primary bg-bg-tertiary px-4 py-3 text-sm text-text-primary outline-none focus:border-accent"
              :placeholder="t('settings.organizationScopePlaceholder')"
            />
          </div>

          <button
            type="submit"
            class="w-full rounded-2xl bg-accent px-5 py-3 text-sm font-semibold text-white transition hover:bg-accent/80 disabled:opacity-60"
            :disabled="isSubmitting"
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

          <p
            v-if="registrationNotice"
            class="rounded-2xl border border-success/30 bg-success/10 px-4 py-3 text-sm text-success"
          >
            {{ registrationNotice }}
          </p>

          <p v-if="errorMessage" class="rounded-2xl border border-error/30 bg-error/10 px-4 py-3 text-sm text-error">
            {{ errorMessage }}
          </p>
        </form>

        <div v-if="!isLocalMode" class="mt-4 border-t border-border-primary pt-4 text-center">
          <button
            type="button"
            class="text-xs text-text-secondary transition-colors hover:text-accent"
            @click="enterLocalMode"
          >
            {{ t("loginGateway.switchToLocalMode") }}
          </button>
        </div>
      </section>
    </div>
  </div>
</template>
