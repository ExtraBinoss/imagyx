import { defineStore } from 'pinia'
import { imagyxApi } from '../api/tauri'

const DEFAULT_SHORTCUT = 'Control+Numpad9'

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
        this.spotlight = await imagyxApi.spotlightShortcut()
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
