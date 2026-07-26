import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { getCurrentWindow } from '@tauri-apps/api/window'
import App from './App.vue'
import SpotlightSearch from './SpotlightSearch.vue'
import './style.css'
import './virtual-grid.css'
import './spotlight-window.css'

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

const RootComponent = currentWindow.label === 'spotlight' ? SpotlightSearch : App
createApp(RootComponent).use(createPinia()).mount('#app')
