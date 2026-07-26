<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { X } from '@lucide/vue'
import AppSidebar from './components/AppSidebar.vue'
import ImageGrid from './components/ImageGrid.vue'
import ImagePreviewDialog from './components/ImagePreviewDialog.vue'
import SearchHeader from './components/SearchHeader.vue'
import StatusBar from './components/StatusBar.vue'
import Button from './components/ui/Button/Button.vue'
import ToastViewport from './components/ui/Toast/ToastViewport.vue'
import type { ImageAsset } from './types'
import { useLibraryStore } from './stores/library'
import { usePlatformStore } from './stores/platform'
import { useThemeStore } from './stores/theme'
import { capitalize, useTagTypewriter } from './useTagTypewriter'
import { debounce } from './utils'

const store = useLibraryStore()
const platform = usePlatformStore()
const theme = useThemeStore()
const { typedTag } = useTagTypewriter()
const localQuery = ref('')
const previewImage = ref<ImageAsset | null>(null)
const searchHeader = ref<{ focusSearch: () => void; selectSearch: () => void } | null>(null)
let unlistenOpenImage: UnlistenFn | null = null

const folderPrefix = computed(() => store.selectedFolder ? `${store.selectedFolder.name}: ` : 'All images: ')
const searchPlaceholder = computed(() => `${folderPrefix.value}${capitalize(typedTag.value)}…`)
const viewKey = computed(() => `${store.selectedFolderId ?? 'all'}:${store.query}`)

const searchLater = debounce(() => {
  store.setQuery(localQuery.value.trim())
  void store.refreshImages()
}, 180)

watch(localQuery, searchLater)

async function addFolder() {
  const selected = await open({ directory: true, multiple: false, title: 'Choisir un dossier à suivre' })
  if (typeof selected === 'string') await store.addFolder(selected)
}

async function removeFolder(folderId: string) {
  await store.removeFolder(folderId)
}

function handleTypeToSearch(event: KeyboardEvent) {
  if (previewImage.value) return
  const target = event.target as HTMLElement | null
  if (target?.matches('input, textarea, select, [contenteditable="true"]')) return

  if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === 'k') {
    event.preventDefault()
    searchHeader.value?.selectSearch()
    return
  }

  if (event.ctrlKey || event.metaKey || event.altKey || event.key.length !== 1 || event.key.trim().length === 0) return
  event.preventDefault()
  localQuery.value += event.key
  void nextTick(() => searchHeader.value?.focusSearch())
}

async function openImageFromSpotlight(imageId: string) {
  localQuery.value = ''
  store.query = ''
  store.selectedFolderId = null
  await store.refreshImages()
  previewImage.value = store.images.find((image) => image.id === imageId) ?? null
}

onMounted(async () => {
  theme.initialize()
  void platform.initialize()
  void store.initialize()
  window.addEventListener('keydown', handleTypeToSearch)
  unlistenOpenImage = await listen<string>('open-image-requested', (event) => {
    void openImageFromSpotlight(event.payload)
  })
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleTypeToSearch)
  unlistenOpenImage?.()
  for (const unlisten of store.listeners) unlisten()
})
</script>

<template>
  <main class="app-shell">
    <AppSidebar
      :folders="store.folders"
      :selected-folder-id="store.selectedFolderId"
      :total-images="store.totalImages"
      :progress="store.progress"
      :model-progress="store.modelProgress"
      :runtime-stats="store.runtimeStats"
      @select="store.selectFolder"
      @add="addFolder"
      @remove="removeFolder"
      @reindex="store.reindexFolder"
      @pause-indexing="store.pauseIndexing"
      @resume-indexing="store.resumeIndexing"
    />

    <section class="workspace">
      <SearchHeader
        ref="searchHeader"
        v-model="localQuery"
        :placeholder="searchPlaceholder"
        :model-ready="store.appInfo?.aiReady ?? false"
        :model-backend="store.appInfo?.aiBackend ?? 'Automatique'"
        :searching="store.semanticSearching"
        :result-count="store.images.length"
      />

      <div v-if="store.error" class="error-banner" role="alert">
        <span>{{ store.error }}</span>
        <Button variant="ghost" size="icon" aria-label="Masquer l’erreur" @click="store.error = null"><X :size="15" /></Button>
      </div>

      <ImageGrid
        :images="store.images"
        :loading="store.loading"
        :has-folders="store.folders.length > 0"
        :view-key="viewKey"
        @explain="store.explainImage"
        @preview="previewImage = $event"
      />
      <StatusBar :progress="null" :database-path="store.appInfo?.databasePath ?? ''" />
    </section>
  </main>
  <ToastViewport />
  <ImagePreviewDialog :image="previewImage" @close="previewImage = null" />
</template>
