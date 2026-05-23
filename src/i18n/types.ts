import type { Composer } from "vue-i18n";
import zh from "./locales/zh.json";
import zhExtra from "./locales/zh.extra.json";

export type I18nMessages = typeof zh & typeof zhExtra;

export type SupportedLocale = "en" | "zh";

export interface EnhancedI18n extends Composer {
  t: (key: string, ...args: any[]) => string;
  te: (key: string) => boolean;
}

export interface LocaleSettings {
  locale: SupportedLocale;
  fallbackLocale: SupportedLocale;
}
