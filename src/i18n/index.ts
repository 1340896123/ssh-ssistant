import { createI18n } from "vue-i18n";
import { SupportedLocale, LocaleSettings, I18nMessages } from "./types";

export let i18n: any;

type LocaleMessages = I18nMessages;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function mergeMessages(
  base: LocaleMessages,
  extra: LocaleMessages,
): LocaleMessages {
  const result = { ...base } as Record<string, unknown>;

  for (const [key, value] of Object.entries(extra)) {
    const existing = result[key];
    if (isRecord(existing) && isRecord(value)) {
      result[key] = mergeMessages(
        existing as unknown as LocaleMessages,
        value as unknown as LocaleMessages,
      );
    } else {
      result[key] = value;
    }
  }

  return result as unknown as LocaleMessages;
}

// 动态导入语言资源
const loadLocaleMessages = async (locale: SupportedLocale) => {
  const [messages, extraMessages] = await Promise.all([
    import(`./locales/${locale}.json`),
    import(`./locales/${locale}.extra.json`),
  ]);

  return mergeMessages(
    messages.default as LocaleMessages,
    extraMessages.default as LocaleMessages,
  );
};

// 创建 i18n 实例
export const createAppI18n = async (
  locale: SupportedLocale = "zh"
): Promise<ReturnType<typeof createI18n>> => {
  const localeMessages = await loadLocaleMessages(locale);

  const settings: LocaleSettings = {
    locale,
    fallbackLocale: "en",
  };

  i18n = createI18n({
    legacy: false,
    locale: settings.locale,
    fallbackLocale: settings.fallbackLocale,
    messages: {
      [locale]: localeMessages,
    },
    globalInjection: true,
  });

  return i18n;
};

export const setI18nLanguage = async (locale: SupportedLocale) => {
  if (!i18n) return;

  const global = i18n.global;

  if (!global.availableLocales.includes(locale)) {
    const messages = await loadLocaleMessages(locale);
    global.setLocaleMessage(locale, messages);
  }

  global.locale.value = locale;

  if (typeof window !== "undefined") {
    localStorage.setItem("preferred-locale", locale);
  }

  document.querySelector("html")?.setAttribute("lang", locale);
};

// 默认导出创建函数
export default createAppI18n;
