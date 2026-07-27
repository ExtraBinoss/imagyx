import { defineStore } from 'pinia'
import { imagyxApi } from '../api/tauri'

export type DesktopPlatform = 'windows' | 'macos' | 'linux' | 'unknown'
export type WindowControlsPosition = 'left' | 'right'

function normalizePlatform(value: string): DesktopPlatform {
  if (value === 'windows') return 'windows'
  if (value === 'macos') return 'macos'
  if (value === 'linux') return 'linux'
  return 'unknown'
}

function initialControlsPosition(): WindowControlsPosition {
  const saved = localStorage.getItem('imagyx.window-controls-position')
  return saved === 'right' ? 'right' : 'left'
}

function detectInitialPlatform(): DesktopPlatform {
  if (typeof navigator === 'undefined') return 'unknown'
  const ua = navigator.userAgent.toLowerCase()
  if (ua.includes('mac')) return 'macos'
  if (ua.includes('win')) return 'windows'
  if (ua.includes('linux')) return 'linux'
  return 'unknown'
}

export const usePlatformStore = defineStore('platform', {
  state: () => ({
    platform: detectInitialPlatform(),
    version: '0.1.0',
    isDevMode: Boolean(import.meta.env.DEV),
    controlsPosition: initialControlsPosition(),
    initialized: false,
  }),
  getters: {
    appVersion: (state) => state.version,
    isDev: (state) => state.isDevMode,
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
    setControlsPosition(position: WindowControlsPosition) {
      this.controlsPosition = position
      try {
        localStorage.setItem('imagyx.window-controls-position', position)
      } catch {
        /* ignore */
      }
    },
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
