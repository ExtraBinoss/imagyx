import { defineStore } from 'pinia'
import { emit, listen } from '@tauri-apps/api/event'

export type ThemeMode = 'system' | 'light' | 'dark'

const STORAGE_KEY = 'imagyx-theme'

function isThemeMode(value: string | null): value is ThemeMode {
  return value === 'system' || value === 'light' || value === 'dark'
}

function resolveTheme(mode: ThemeMode): 'light' | 'dark' {
  if (mode !== 'system') return mode
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export const useThemeStore = defineStore('theme', {
  state: () => ({
    mode: 'system' as ThemeMode,
    initialized: false,
  }),

  actions: {
    initialize() {
      if (this.initialized) return
      const saved = localStorage.getItem(STORAGE_KEY)
      this.mode = isThemeMode(saved) ? saved : 'system'
      this.apply()
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
        if (this.mode === 'system') this.apply()
      })
      void listen<ThemeMode>('theme-changed', (event) => {
        if (!isThemeMode(event.payload)) return
        this.mode = event.payload
        localStorage.setItem(STORAGE_KEY, event.payload)
        this.apply()
      })
      this.initialized = true
    },

    setMode(mode: ThemeMode) {
      this.mode = mode
      localStorage.setItem(STORAGE_KEY, mode)
      this.apply()
      void emit('theme-changed', mode)
    },

    apply() {
      const resolved = resolveTheme(this.mode)
      document.documentElement.dataset.theme = resolved
      document.documentElement.style.colorScheme = resolved
    },
  },
})
