<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { X } from '@lucide/vue'
import AppSidebar from './components/AppSidebar.vue'
import ImageGrid from './components/ImageGrid.vue'
import SearchHeader from './components/SearchHeader.vue'
import StatusBar from './components/StatusBar.vue'
import Button from './components/ui/Button/Button.vue'
import ToastViewport from './components/ui/Toast/ToastViewport.vue'
import { useLibraryStore } from './stores/library'
import { useThemeStore } from './stores/theme'
import { debounce } from './utils'

const store = useLibraryStore()
const theme = useThemeStore()
const localQuery = ref('')

const selectedTitle = computed(() => store.selectedFolder?.name ?? 'Toutes les images')
const subtitle = computed(() => {
  if (store.semanticSearching) return 'Recherche IA en cours…'
  if (store.query) return `${store.images.length} résultat${store.images.length > 1 ? 's' : ''}`
  return `${store.images.length} image${store.images.length > 1 ? 's' : ''}`
})
const viewKey = computed(() => `${store.selectedFolderId ?? 'all'}:${store.query}:${store.selectedModel}`)

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

onMounted(() => {
  theme.initialize()
  void store.initialize()
})

onBeforeUnmount(() => {
  for (const unlisten of store.listeners) unlisten()
})
</script>

<template>
  <main class="app-shell">
    <AppSidebar
      :folders="store.folders"
      :selected-folder-id="store.selectedFolderId"
      :selected-model="store.selectedModel"
      :total-images="store.totalImages"
      :progress="store.progress"
      :model-progress="store.modelProgress"
      :runtime-stats="store.runtimeStats"
      @select="store.selectFolder"
      @model="store.selectModel"
      @add="addFolder"
      @remove="removeFolder"
      @reindex="store.reindexFolder"
    />

    <section class="workspace">
      <SearchHeader
        v-model="localQuery"
        :model-ready="store.appInfo?.aiReady ?? false"
        :model-backend="store.appInfo?.aiBackend ?? 'Automatique'"
      />

      <div class="content-heading">
        <div><h1>{{ selectedTitle }}</h1><p>{{ subtitle }}</p></div>
      </div>

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
      />
      <StatusBar :progress="null" :database-path="store.appInfo?.databasePath ?? ''" />
    </section>
  </main>
  <ToastViewport />
</template>
