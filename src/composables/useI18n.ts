import { useI18n as useVueI18n } from 'vue-i18n'
import { computed } from 'vue'
import { SupportedLocale, EnhancedI18n } from '../i18n/types'
import { setI18nLanguage } from '../i18n'

export const useI18n = () => {
  const { t, locale, availableLocales } = useVueI18n() as EnhancedI18n

  // 切换语言
  const changeLanguage = async (newLocale: SupportedLocale) => {
    await setI18nLanguage(newLocale)
  }

  // 获取当前语言
  const currentLanguage = computed(() => locale.value as SupportedLocale)

  // 判断是否为中文
  const isChinese = computed(() => currentLanguage.value === 'zh')

  // 判断是否为英文
  const isEnglish = computed(() => currentLanguage.value === 'en')

  return {
    t,
    locale: currentLanguage,
    availableLocales,
    changeLanguage,
    isChinese,
    isEnglish
  }
}