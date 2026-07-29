import { defineStore } from 'pinia'

const COMPLETED_KEY = 'imagyx-onboarding-completed.v1'

export const useOnboardingStore = defineStore('onboarding', {
  state: () => ({
    open: false,
    neverAskAgain: false,
    initialized: false,
  }),

  actions: {
    initialize() {
      if (this.initialized) return
      this.open = localStorage.getItem(COMPLETED_KEY) !== 'true'
      this.initialized = true
    },

    ensureFolderSetup(hasFolders: boolean) {
      if (hasFolders) return
      this.neverAskAgain = false
      this.open = true
    },

    show() {
      this.neverAskAgain = false
      this.open = true
    },

    setNeverAskAgain(value: boolean) {
      this.neverAskAgain = value
    },

    close() {
      if (this.neverAskAgain) localStorage.setItem(COMPLETED_KEY, 'true')
      this.open = false
    },

    finish() {
      localStorage.setItem(COMPLETED_KEY, 'true')
      this.neverAskAgain = true
      this.open = false
    },
  },
})
