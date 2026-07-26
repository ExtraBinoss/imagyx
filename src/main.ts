import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './style.css'
import './virtual-grid.css'

const savedTheme = localStorage.getItem('imagyx-theme')
const resolvedTheme =
  savedTheme === 'light' || savedTheme === 'dark'
    ? savedTheme
    : window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light'

document.documentElement.dataset.theme = resolvedTheme
document.documentElement.style.colorScheme = resolvedTheme

createApp(App).use(createPinia()).mount('#app')
