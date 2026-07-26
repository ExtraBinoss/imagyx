import { defineStore } from 'pinia'
import { imagyxApi } from '../api/tauri'

export type DesktopPlatform = 'windows' | 'macos' | 'linux' | 'unknown'

function normalizePlatform(value: string): DesktopPlatform {
  if (value === 'windows') return 'windows'
  if (value === 'macos') return 'macos'
  if (value === 'linux') return 'linux'
  return 'unknown'
}

export const usePlatformStore = defineStore('platform', {
  state: () => ({
    platform: 'unknown' as DesktopPlatform,
    initialized: false,
  }),
  getters: {
    isWindows: (state) => state.platform === 'windows',
    isMac: (state) => state.platform === 'macos',
    isLinux: (state) => state.platform === 'linux',
    fileManagerName: (state) => {
      if (state.platform === 'windows') return 'Explorer'
      if (state.platform === 'macos') return 'Finder'
      return 'gestionnaire de fichiers'
    },
    openFolderLabel(): string {
      return `Ouvrir dans ${this.fileManagerName}`
    },
  },
  actions: {
    async initialize() {
      if (this.initialized) return
      try {
        this.platform = normalizePlatform(await imagyxApi.platform())
      } finally {
        this.initialized = true
      }
    },
  },
})
