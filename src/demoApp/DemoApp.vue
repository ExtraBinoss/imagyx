<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Moon, Search, Settings, Sparkles, Sun } from '@lucide/vue'
import type { ImageAsset } from '../types'
import { useThemeStore } from '../stores/theme'
import { capitalize, useTagTypewriter } from '../useTagTypewriter'
import MovingBorder from '../components/ui/MovingBorder/MovingBorder.vue'
import SpotlightInput from '../components/Spotlight/SpotlightInput.vue'
import SpotlightResults from '../components/Spotlight/SpotlightResults.vue'
import SpotlightSettings from '../components/Spotlight/SpotlightSettings.vue'
import type { SpotlightIndexJob, SpotlightView } from '../components/Spotlight/types'

// Import raw onboarding assets
import tribalPortrait from '../assets/onboarding_assets/99003f6e5f05348d1852c38ed196d988.jpg'
import womanGreen from '../assets/onboarding_assets/girl_train_segmented.png'
import womanPortrait from '../assets/onboarding_assets/karina armageddon.jpg'
import greenFashion from '../assets/onboarding_assets/téléchargement (2).jpg'
import angleAngel from '../assets/onboarding_assets/→ ❛ 🕷 · 𝟎 ` 천사.jpg'

const themeStore = useThemeStore()
const activeTab = ref<'spotlight' | 'components'>('spotlight')
const view = ref<SpotlightView>('search')
const searchQuery = ref('')
const settingsQuery = ref('')
const selectedIndex = ref(0)
const searching = ref(false)
const error = ref<string | null>(null)
const resultsOpen = ref(false)
const shellMerged = ref(false)
const copiedImageId = ref<string | null>(null)
const copyingImageId = ref<string | null>(null)
const revealingImageId = ref<string | null>(null)
const openingImageId = ref<string | null>(null)

const { typedTag } = useTagTypewriter()

const allMockAssets: ImageAsset[] = [
  {
    id: '1',
    name: 'girl_train_segmented.png',
    path: '/assets/onboarding_assets/girl_train_segmented.png',
    folderId: 'onboarding',
    extension: 'png',
    width: 1920,
    height: 1080,
    sizeBytes: 2906408,
    modifiedAt: Date.now(),
    thumbnailPath: womanGreen,
    thumbnail_url: womanGreen,
    preview_url: womanGreen,
    semanticScore: 0.94,
  } as ImageAsset & { thumbnail_url: string; preview_url: string },
  {
    id: '2',
    name: 'karina armageddon.jpg',
    path: '/assets/onboarding_assets/karina armageddon.jpg',
    folderId: 'onboarding',
    extension: 'jpg',
    width: 1200,
    height: 1600,
    sizeBytes: 131643,
    modifiedAt: Date.now(),
    thumbnailPath: womanPortrait,
    thumbnail_url: womanPortrait,
    preview_url: womanPortrait,
    semanticScore: 0.89,
  } as ImageAsset & { thumbnail_url: string; preview_url: string },
  {
    id: '3',
    name: '99003f6e5f05348d1852c38ed196d988.jpg',
    path: '/assets/onboarding_assets/99003f6e5f05348d1852c38ed196d988.jpg',
    folderId: 'onboarding',
    extension: 'jpg',
    width: 800,
    height: 1200,
    sizeBytes: 44356,
    modifiedAt: Date.now(),
    thumbnailPath: tribalPortrait,
    thumbnail_url: tribalPortrait,
    preview_url: tribalPortrait,
    semanticScore: 0.84,
  } as ImageAsset & { thumbnail_url: string; preview_url: string },
  {
    id: '4',
    name: 'téléchargement (2).jpg',
    path: '/assets/onboarding_assets/téléchargement (2).jpg',
    folderId: 'onboarding',
    extension: 'jpg',
    width: 1080,
    height: 1350,
    sizeBytes: 160177,
    modifiedAt: Date.now(),
    thumbnailPath: greenFashion,
    thumbnail_url: greenFashion,
    preview_url: greenFashion,
    semanticScore: 0.78,
  } as ImageAsset & { thumbnail_url: string; preview_url: string },
  {
    id: '5',
    name: '→ ❛ 🕷 · 𝟎 ` 천사.jpg',
    path: '/assets/onboarding_assets/→ ❛ 🕷 · 𝟎 ` 천사.jpg',
    folderId: 'onboarding',
    extension: 'jpg',
    width: 736,
    height: 736,
    sizeBytes: 38166,
    modifiedAt: Date.now(),
    thumbnailPath: angleAngel,
    thumbnail_url: angleAngel,
    preview_url: angleAngel,
    semanticScore: 0.72,
  } as ImageAsset & { thumbnail_url: string; preview_url: string },
]

const results = ref<ImageAsset[]>([])

const activeQuery = computed({
  get: () => view.value === 'settings' ? settingsQuery.value : searchQuery.value,
  set: (val: string) => {
    if (view.value === 'settings') settingsQuery.value = val
    else searchQuery.value = val
  },
})

const placeholder = computed(() => {
  if (view.value === 'settings') return 'Search settings…'
  return typedTag.value
    ? `All images: ${capitalize(typedTag.value)}…`
    : 'All images: name or description…'
})

const resultLabel = computed(() => `${results.value.length} résultat${results.value.length === 1 ? '' : 's'}`)

let morphTimer: number | undefined

watch(searchQuery, (q) => {
  selectedIndex.value = 0
  const query = q.trim().toLowerCase()
  if (!query) {
    results.value = []
    resultsOpen.value = false
    if (morphTimer) clearTimeout(morphTimer)
    morphTimer = window.setTimeout(() => {
      shellMerged.value = false
    }, 220)
    return
  }

  if (morphTimer) clearTimeout(morphTimer)
  shellMerged.value = true
  resultsOpen.value = true

  results.value = allMockAssets.filter((asset) => {
    return asset.name.toLowerCase().includes(query)
  })
  if (results.value.length === 0 && query.length > 0) {
    // If no exact query match, return all raw mock assets for demo presentation
    results.value = allMockAssets
  }
})

function handleCopy(img: ImageAsset) {
  copiedImageId.value = img.id
  setTimeout(() => { copiedImageId.value = null }, 1800)
}
function handleReveal(img: ImageAsset) {
  revealingImageId.value = img.id
  setTimeout(() => { revealingImageId.value = null }, 1200)
}
function handleOpen(img: ImageAsset) {
  openingImageId.value = img.id
  setTimeout(() => { openingImageId.value = null }, 1200)
}

function openSettings() {
  view.value = 'settings'
  shellMerged.value = true
  resultsOpen.value = true
}

function backToSearch() {
  view.value = 'search'
  if (!searchQuery.value.trim()) {
    resultsOpen.value = false
    shellMerged.value = false
  }
}

function toggleTheme() {
  const current = document.documentElement.dataset.theme
  const nextTheme = current === 'dark' ? 'light' : 'dark'
  document.documentElement.dataset.theme = nextTheme
  document.documentElement.style.colorScheme = nextTheme
  localStorage.setItem('imagyx-theme', nextTheme)
  themeStore.setMode(nextTheme)
}
</script>

<template>
  <div class="demo-page">
    <header class="demo-header">
      <div class="demo-brand">
        <Sparkles :size="20" class="brand-sparkle" />
        <span>Imagyx Hyperframes Studio</span>
      </div>
      <nav class="demo-tabs">
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'spotlight' }"
          @click="activeTab = 'spotlight'"
        >
          <Search :size="15" /> Spotlight Demo
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'components' }"
          @click="activeTab = 'components'"
        >
          <Settings :size="15" /> Components
        </button>
        <button class="theme-toggle-btn" aria-label="Toggle theme" @click="toggleTheme">
          <Sun v-if="themeStore.resolvedMode === 'dark'" :size="16" />
          <Moon v-else :size="16" />
        </button>
      </nav>
    </header>

    <main class="demo-body">
      <!-- Spotlight Demo Tab -->
      <section v-if="activeTab === 'spotlight'" class="spotlight-demo-viewport">
        <div class="spotlight-stage">
          <MovingBorder border-radius="20px" :duration="3600" :active="true">
            <div
              class="spotlight-shell"
              :class="{
                'spotlight-shell--merged': shellMerged,
                'spotlight-shell--expanded': resultsOpen
              }"
            >
              <SpotlightInput
                v-model="activeQuery"
                :view="view"
                :placeholder="placeholder"
                :searching="searching"
                :result-label="resultLabel"
                @settings="openSettings"
                @back="backToSearch"
              />

              <Transition name="panel-morph">
                <div v-if="resultsOpen" class="spotlight-expandable">
                  <SpotlightResults
                    v-if="view === 'search'"
                    :results="results"
                    :selected-index="selectedIndex"
                    :searching="searching"
                    :error="error"
                    :copied-image-id="copiedImageId"
                    :copying-image-id="copyingImageId"
                    :revealing-image-id="revealingImageId"
                    :opening-image-id="openingImageId"
                    :show-add-action="false"
                    :jobs="[]"
                    file-manager-name="Explorer"
                    @select="selectedIndex = $event"
                    @copy="handleCopy"
                    @reveal="handleReveal"
                    @open="handleOpen"
                  />

                  <SpotlightSettings
                    v-else-if="view === 'settings'"
                    :query="settingsQuery"
                  />
                </div>
              </Transition>
            </div>
          </MovingBorder>
        </div>
      </section>

      <!-- Placeholder Tab for Future Demo Components -->
      <section v-else class="components-demo-viewport">
        <div class="demo-card">
          <h2>Components Studio</h2>
          <p>More component demos can be loaded here for Hyperframes recording.</p>
        </div>
      </section>
    </main>
  </div>
</template>

<style scoped>
.demo-page {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  background: var(--background, #0b0c0f);
  color: var(--text, #e2e8f0);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  overflow: hidden;
}

.demo-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  border-bottom: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  background: color-mix(in srgb, var(--surface, #14171f) 80%, transparent);
  backdrop-filter: blur(12px);
}

.demo-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 700;
  font-size: 15px;
}

.brand-sparkle {
  color: var(--primary, #6366f1);
}

.demo-tabs {
  display: flex;
  gap: 8px;
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: var(--text-muted, #94a3b8);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.tab-btn:hover {
  color: var(--text, #ffffff);
  background: rgba(255, 255, 255, 0.05);
}

.tab-btn.active {
  background: var(--surface, #1e2330);
  color: var(--text, #ffffff);
  border-color: var(--border, rgba(255, 255, 255, 0.12));
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.demo-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  position: relative;
}

.spotlight-demo-viewport {
  flex: 1;
  display: grid;
  place-items: center;
  padding: 40px;
  background: radial-gradient(circle at 50% 30%, color-mix(in srgb, var(--primary, #6366f1) 12%, transparent), transparent 60%);
}

.spotlight-stage {
  width: min(680px, 92vw);
}

.spotlight-shell {
  width: 100%;
  border-radius: 20px;
  background: color-mix(in srgb, var(--surface-elevated, #181c27) 96%, transparent);
  box-shadow: inset 0 1px rgba(255, 255, 255, 0.12), 0 24px 60px -20px rgba(0, 0, 0, 0.7);
  overflow: hidden;
  transition: all 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

.spotlight-expandable {
  border-top: 1px solid var(--border, rgba(255, 255, 255, 0.08));
}

.components-demo-viewport {
  flex: 1;
  display: grid;
  place-items: center;
  padding: 40px;
}

.demo-card {
  padding: 32px;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 16px;
  background: var(--surface, #14171f);
  text-align: center;
}
</style>
