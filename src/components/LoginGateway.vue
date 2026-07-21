<script setup lang="ts">
import { computed } from "vue";
import { Loader2, LogIn } from "lucide-vue-next";
import { useI18n } from "../composables/useI18n";

const props = defineProps<{
  isAwaitingBrowserAuth?: boolean;
  browserAuthError?: string;
}>();

const emit = defineEmits<{
  (e: "requestBrowserLogin"): void;
  (e: "authenticated"): void;
}>();

const { t } = useI18n();

const heroTitleKey = computed(() => "loginGateway.heroTitle.personal");
const heroDescriptionKey = computed(() => "loginGateway.heroDescription.personal");
const variantModeCardKey = computed(() => "loginGateway.modeCards.personal");
const variantModeLabelKey = computed(() => "settings.accountModes.personal");
const variantSubtitleKey = computed(() => "loginGateway.modeDescriptions.personalLogin");
const browserButtonLabel = computed(() => t("loginGateway.browserLogin.button.personal"));

function requestBrowserLogin() {
  if (props.isAwaitingBrowserAuth) {
    return;
  }
  emit("requestBrowserLogin");
}

</script>

<template>
  <div class="flex min-h-screen items-center justify-center bg-bg-primary px-6 py-10">
    <div class="grid w-full max-w-6xl gap-8 lg:grid-cols-[1.1fr_0.9fr]">
      <section class="rounded-[32px] border border-border-primary bg-bg-secondary p-8 shadow-[0_28px_80px_rgba(0,0,0,0.18)]">
        <p class="text-sm font-semibold uppercase tracking-[0.32em] text-accent">{{ t("app.title") }}</p>
        <h1 class="mt-4 text-4xl font-black tracking-tight text-text-primary sm:text-5xl">
          {{ t(heroTitleKey) }}
        </h1>
        <p class="mt-5 max-w-2xl text-base leading-8 text-text-secondary">
          {{ t(heroDescriptionKey) }}
        </p>

        <div class="mt-8 grid gap-4">
          <article class="rounded-3xl border border-border-primary bg-bg-tertiary p-5">
            <p class="text-sm font-semibold text-text-secondary">{{ t(variantModeLabelKey) }}</p>
            <p class="mt-3 text-sm leading-7 text-text-primary">{{ t(variantModeCardKey) }}</p>
          </article>
        </div>
      </section>

      <section class="rounded-[32px] border border-border-primary bg-bg-secondary p-8 shadow-[0_28px_80px_rgba(0,0,0,0.18)]">
        <div>
          <h2 class="text-2xl font-black text-text-primary">{{ t("loginGateway.title") }}</h2>
          <p class="mt-2 text-sm text-text-secondary">{{ t(variantSubtitleKey) }}</p>
        </div>

        <div class="mt-6 space-y-5">
          <button
            data-testid="login-gateway-browser-login"
            type="button"
            class="flex w-full items-center justify-center gap-2 rounded-2xl bg-accent px-5 py-3 text-sm font-semibold text-white transition hover:bg-accent/80 disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="isAwaitingBrowserAuth"
            @click="requestBrowserLogin"
          >
            <LogIn v-if="!isAwaitingBrowserAuth" class="h-4 w-4" />
            <Loader2 v-else class="h-4 w-4 animate-spin" />
            {{ browserButtonLabel }}
          </button>

          <p
            v-if="isAwaitingBrowserAuth"
            class="rounded-2xl border border-accent/30 bg-accent/10 px-4 py-3 text-sm text-accent"
          >
            {{ t("loginGateway.browserLogin.awaiting") }}
          </p>

          <p
            v-if="browserAuthError"
            class="rounded-2xl border border-error/30 bg-error/10 px-4 py-3 text-sm text-error"
          >
            {{ browserAuthError }}
          </p>

          <p class="text-center text-xs leading-6 text-text-secondary">
            {{ t("loginGateway.browserLogin.hint") }}
          </p>
        </div>
      </section>
    </div>
  </div>
</template>
