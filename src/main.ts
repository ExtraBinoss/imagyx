import { createApp, type Component } from 'vue'
import { createPinia } from 'pinia'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { installPerformanceDiagnostics } from './utils'
import './style.css'

installPerformanceDiagnostics()

const currentWindow = getCurrentWindow()
const savedTheme = localStorage.getItem('imagyx-theme')
const resolvedTheme =
  savedTheme === 'light' || savedTheme === 'dark'
    ? savedTheme
    : window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light'

document.documentElement.dataset.theme = resolvedTheme
document.documentElement.dataset.window = currentWindow.label
document.documentElement.style.colorScheme = resolvedTheme

async function loadRootComponent(): Promise<Component> {
  if (currentWindow.label === 'spotlight') {
    await import('./spotlight-window.css')
    return (await import('./components/Spotlight/SpotlightSearch.vue')).default
  }
  await import('./virtual-grid.css')
  return (await import('./App.vue')).default
}

const RootComponent = await loadRootComponent()
createApp(RootComponent).use(createPinia()).mount('#app')
