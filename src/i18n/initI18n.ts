import { createI18n, type I18n } from 'vue-i18n'
import en from './en'
import fr from './fr'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AppI18n = I18n<any, any, any, any, false>
let _i18n: AppI18n | null = null

function detectLocale(): 'en' | 'fr' {
  const stored = localStorage.getItem('imagyx-locale')
  if (stored === 'en' || stored === 'fr') return stored
  const navLang = navigator.language?.slice(0, 2)
  if (navLang === 'fr') return 'fr'
  return 'en'
}

export function initI18n(): AppI18n {
  if (_i18n) return _i18n
  const locale = detectLocale()

  const i18n = createI18n({
    legacy: false,
    locale,
    fallbackLocale: 'en',
    messages: { en, fr },
  })

  _i18n = i18n as unknown as AppI18n
  ;(i18n.global.locale as { value: string }).value = locale
  document.documentElement.lang = locale

  return _i18n
}

export function getI18n(): AppI18n {
  if (!_i18n) throw new Error('i18n not initialized. Call initI18n() before getI18n().')
  return _i18n
}

export function setLocale(locale: 'en' | 'fr') {
  const i18n = getI18n()
  ;(i18n.global.locale as { value: string }).value = locale
  document.documentElement.lang = locale
  try {
    localStorage.setItem('imagyx-locale', locale)
  } catch { /* ignore */ }
}

