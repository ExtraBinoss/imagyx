<script setup lang="ts">
import { computed } from 'vue'
import { Folder, FolderOpen, Images, MoreHorizontal, Plus, RefreshCw, Trash2 } from '@lucide/vue'
import type { FollowedFolder, IndexProgress, ModelDownloadProgress, RuntimeStats } from '../types'
import { SEMANTIC_MODELS, type SemanticModelKey } from '../services/semantic'
import Button from './ui/Button/Button.vue'
import Popover from './ui/Popover/Popover.vue'
import Select from './ui/Select/Select.vue'
import Tooltip from './ui/Tooltip/Tooltip.vue'
import LocalAiStatus from './LocalAiStatus.vue'

const props = defineProps<{
  folders: FollowedFolder[]
  selectedFolderId: string | null
  selectedModel: SemanticModelKey
  totalImages: number
  progress: IndexProgress | null
  modelProgress: ModelDownloadProgress | null
  runtimeStats: RuntimeStats | null
}>()

const emit = defineEmits<{
  select: [folderId: string | null]
  add: []
  remove: [folderId: string]
  reindex: [folderId: string]
  model: [modelKey: SemanticModelKey]
}>()

const allSelected = computed(() => props.selectedFolderId === null)
const modelOptions = SEMANTIC_MODELS.map((model) => ({
  label: model.name,
  value: model.key,
  description: model.description,
}))

function reindexFromContext(event: MouseEvent, folderId: string) {
  event.preventDefault()
  emit('reindex', folderId)
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">ix</div>
      <div><strong>Imagyx</strong><span>Intelligence locale</span></div>
    </div>

    <Button variant="primary" size="lg" block @click="emit('add')">
      <template #leading><Plus :size="17" :stroke-width="2.2" /></template>
      Ajouter un dossier
    </Button>

    <nav class="folder-nav" aria-label="Dossiers suivis">
      <p class="section-label">Bibliothèque</p>
      <Button class="folder-row" :class="{ active: allSelected }" variant="ghost" block @click="emit('select', null)">
        <Images :size="17" />
        <span class="folder-name">Toutes les images</span>
        <span class="folder-count">{{ totalImages }}</span>
      </Button>

      <p class="section-label followed-label">Dossiers suivis</p>
      <div v-if="folders.length === 0" class="sidebar-empty">
        <FolderOpen :size="19" /><span>Aucun dossier suivi</span>
      </div>

      <div v-for="folder in folders" :key="folder.id" class="folder-entry" @contextmenu="reindexFromContext($event, folder.id)">
        <Button class="folder-row" :class="{ active: selectedFolderId === folder.id }" variant="ghost" block :title="`${folder.path} · clic droit pour réindexer`" @click="emit('select', folder.id)">
          <Folder :size="17" />
          <span class="folder-name">{{ folder.name }}</span>
          <span class="folder-count">{{ folder.imageCount }}</span>
        </Button>
        <div class="folder-actions">
          <Tooltip text="Actions du dossier" side="right">
            <Popover align="end" width="210px">
              <template #trigger><Button variant="ghost" size="icon" aria-label="Actions du dossier"><MoreHorizontal :size="16" /></Button></template>
              <template #content="{ close }">
                <div class="folder-menu">
                  <Button variant="ghost" size="sm" block @click="emit('reindex', folder.id); close()">
                    <template #leading><RefreshCw :size="15" /></template>Réindexer
                  </Button>
                  <Button variant="danger" size="sm" block @click="emit('remove', folder.id); close()">
                    <template #leading><Trash2 :size="15" /></template>Ne plus suivre
                  </Button>
                </div>
              </template>
            </Popover>
          </Tooltip>
        </div>
      </div>
    </nav>

    <div class="sidebar-model-picker">
      <span class="section-label">Modèle sémantique</span>
      <Select
        :model-value="selectedModel"
        :options="modelOptions"
        aria-label="Choisir le modèle sémantique"
        @update:model-value="emit('model', $event as SemanticModelKey)"
      />
    </div>

    <LocalAiStatus :progress="progress" :model-progress="modelProgress" :runtime-stats="runtimeStats" />
  </aside>
</template>
