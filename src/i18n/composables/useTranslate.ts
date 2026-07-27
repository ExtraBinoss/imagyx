import { useI18n } from 'vue-i18n'

export function useTranslate() {
  const { t: vueT, locale, availableLocales } = useI18n()

  function t(key: string, fallbackOrParams?: string | Record<string, unknown>, params?: Record<string, unknown>): string {
    const fallback = typeof fallbackOrParams === 'string' ? fallbackOrParams : key
    const resolvedParams = (typeof fallbackOrParams === 'object' ? fallbackOrParams : params) ?? {}
    return vueT(key, resolvedParams, fallback)
  }

  return { t, locale, availableLocales }
}
