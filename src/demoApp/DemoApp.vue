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
let searchTimer: number | undefined

watch(searchQuery, (q) => {
  selectedIndex.value = 0
  const query = q.trim().toLowerCase()
  if (!query) {
    searching.value = false
    results.value = []
    resultsOpen.value = false
    if (searchTimer) clearTimeout(searchTimer)
    if (morphTimer) clearTimeout(morphTimer)
    morphTimer = window.setTimeout(() => {
      shellMerged.value = false
    }, 220)
    return
  }

  if (morphTimer) clearTimeout(morphTimer)
  if (searchTimer) clearTimeout(searchTimer)

  searching.value = true
  shellMerged.value = true

  // Simulate a realistic AI semantic search latency (~280ms)
  searchTimer = window.setTimeout(() => {
    searching.value = false
    resultsOpen.value = true

    // Map specific demo query behaviors as requested
    if (query.includes('woman') || query.includes('green')) {
      // 1: girl_train_segmented.png (94%)
      // 2: → ❛ 🕷 · 𝟎 ` 천사.jpg (88%)
      // 3: karina armageddon.jpg (82%)
      // 4: greenFashion (75%)
      // 5: tribalPortrait (69%)
      results.value = [
        { ...allMockAssets[0], semanticScore: 0.94 }, // girl_train_segmented
        { ...allMockAssets[4], semanticScore: 0.88 }, // → ❛ 🕷 · 𝟎 ` 천사.jpg
        { ...allMockAssets[1], semanticScore: 0.82 }, // karina armageddon
        { ...allMockAssets[3], semanticScore: 0.75 }, // téléchargement (2).jpg
        { ...allMockAssets[2], semanticScore: 0.69 }, // 99003f6e5f...
      ]
    } else if (query.includes('star')) {
      // 1: 99003f6e5f05348d1852c38ed196d988.jpg (95%)
      // 2: karina armageddon (86%)
      // 3: girl_train_segmented (79%)
      // 4: → ❛ 🕷 · 𝟎 ` 천사.jpg (71%)
      // 5: téléchargement (2).jpg (64%)
      results.value = [
        { ...allMockAssets[2], semanticScore: 0.95 }, // 99003f6e5f05348d1852c38ed196d988.jpg
        { ...allMockAssets[1], semanticScore: 0.86 }, // karina armageddon
        { ...allMockAssets[0], semanticScore: 0.79 },
        { ...allMockAssets[4], semanticScore: 0.71 },
        { ...allMockAssets[3], semanticScore: 0.64 },
      ]
    } else if (query.includes('flower')) {
      // 1: téléchargement (2).jpg (green fashion/floral background) (93%)
      // 2: girl_train_segmented (84%)
      // 3: 99003f6e5f... (77%)
      // 4: karina armageddon (70%)
      // 5: → ❛ 🕷 · 𝟎 ` 천사.jpg (62%)
      results.value = [
        { ...allMockAssets[3], semanticScore: 0.93 }, // téléchargement (2).jpg
        { ...allMockAssets[0], semanticScore: 0.84 },
        { ...allMockAssets[2], semanticScore: 0.77 },
        { ...allMockAssets[1], semanticScore: 0.70 },
        { ...allMockAssets[4], semanticScore: 0.62 },
      ]
    } else {
      const filtered = allMockAssets.filter((asset) => asset.name.toLowerCase().includes(query))
      results.value = filtered.length > 0 ? filtered : allMockAssets
    }
  }, 280)
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
                <div v-if="resultsOpen" class="spotlight-panel">
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
  align-items: center;
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

.theme-toggle-btn {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
  border-radius: 10px;
  background: var(--surface, #1e2330);
  color: var(--text, #ffffff);
  cursor: pointer;
  transition: all 0.15s ease;
}

.theme-toggle-btn:hover {
  background: rgba(255, 255, 255, 0.1);
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
  transition: box-shadow 240ms ease, background-color 180ms ease;
}

.spotlight-panel {
  height: 462px;
  min-height: 0;
  overflow: hidden;
  border-top: 1px solid color-mix(in srgb, var(--border) 76%, transparent);
}

.panel-morph-enter-active,
.panel-morph-leave-active {
  transition: max-height 340ms cubic-bezier(0.16, 1, 0.3, 1), opacity 240ms ease, clip-path 340ms cubic-bezier(0.16, 1, 0.3, 1);
  overflow: hidden;
}

.panel-morph-enter-from,
.panel-morph-leave-to {
  max-height: 0;
  opacity: 0;
  clip-path: inset(0 0 100% 0 round 0 0 20px 20px);
}

.panel-morph-enter-to,
.panel-morph-leave-from {
  max-height: 462px;
  opacity: 1;
  clip-path: inset(0 round 0 0 20px 20px);
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
