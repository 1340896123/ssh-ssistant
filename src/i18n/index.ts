import { createI18n } from "vue-i18n";
import { SupportedLocale, LocaleSettings } from "./types";

export let i18n: any;

// 动态导入语言资源
const loadLocaleMessages = async (locale: SupportedLocale) => {
  const messages = await import(`./locales/${locale}.json`);
  return messages.default;
};

// 创建 i18n 实例
export const createAppI18n = async (
  locale: SupportedLocale = "zh"
): Promise<ReturnType<typeof createI18n>> => {
  const messages = await loadLocaleMessages(locale);

  const settings: LocaleSettings = {
    locale,
    fallbackLocale: "en",
  };

  i18n = createI18n({
    legacy: false,
    locale: settings.locale,
    fallbackLocale: settings.fallbackLocale,
    messages: {
      [locale]: messages,
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
