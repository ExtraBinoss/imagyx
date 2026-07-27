import { getI18n } from './initI18n'

export function storeT(key: string, fallback?: string, params?: Record<string, unknown>): string {
  return String(getI18n().global.t(key, params ?? {}, fallback ?? key))
}
