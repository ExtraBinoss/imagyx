import { defineStore } from 'pinia'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { imagyxApi } from '../api/tauri'

const DEFAULT_SHORTCUT = 'Control+Numpad9'
let shortcutListener: Promise<UnlistenFn> | null = null

export const useShortcutStore = defineStore('shortcut', {
  state: () => ({
    spotlight: DEFAULT_SHORTCUT,
    initialized: false,
    updating: false,
    error: null as string | null,
  }),
  actions: {
    async initialize() {
      if (this.initialized) return
      try {
        const [shortcut] = await Promise.all([
          imagyxApi.spotlightShortcut(),
          shortcutListener ??= listen<string>('spotlight-shortcut-updated', (event) => {
            this.spotlight = event.payload
            this.error = null
          }),
        ])
        this.spotlight = shortcut
      } catch (reason) {
        this.error = String(reason)
      } finally {
        this.initialized = true
      }
    },
    async setSpotlight(shortcut: string) {
      if (this.updating || shortcut === this.spotlight) return
      this.updating = true
      this.error = null
      try {
        this.spotlight = await imagyxApi.setSpotlightShortcut(shortcut)
      } catch (reason) {
        this.error = String(reason)
      } finally {
        this.updating = false
      }
    },
  },
})
