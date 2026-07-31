import { defineStore } from 'pinia'

const COMPLETED_KEY = 'imagyx-onboarding-completed.v1'

export const useOnboardingStore = defineStore('onboarding', {
  state: () => ({
    open: false,
    initialized: false,
  }),

  actions: {
    initialize() {
      if (this.initialized) return
      this.open = localStorage.getItem(COMPLETED_KEY) !== 'true'
      this.initialized = true
    },

    ensureFolderSetup(hasFolders: boolean) {
      if (hasFolders || localStorage.getItem(COMPLETED_KEY) === 'true') return
      this.open = true
    },

    show() {
      this.open = true
    },

    close() {
      localStorage.setItem(COMPLETED_KEY, 'true')
      this.open = false
    },

    finish() {
      localStorage.setItem(COMPLETED_KEY, 'true')
      this.open = false
    },
  },
})
