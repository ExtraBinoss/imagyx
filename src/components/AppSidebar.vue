<script setup lang="ts">
import { computed } from 'vue'
import { Folder, FolderOpen, Images, Plus, RefreshCw, Trash2 } from '@lucide/vue'
import type { FollowedFolder } from '../types'

const props = defineProps<{
  folders: FollowedFolder[]
  selectedFolderId: string | null
  totalImages: number
}>()

const emit = defineEmits<{
  select: [folderId: string | null]
  add: []
  remove: [folderId: string]
  reindex: [folderId: string]
}>()

const allSelected = computed(() => props.selectedFolderId === null)
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">ix</div>
      <div>
        <strong>Imagyx</strong>
        <span>Local image intelligence</span>
      </div>
    </div>

    <button class="add-folder" type="button" @click="emit('add')">
      <Plus :size="17" :stroke-width="2.2" />
      Ajouter un dossier
    </button>

    <nav class="folder-nav" aria-label="Dossiers suivis">
      <p class="section-label">Bibliothèque</p>
      <button
        class="folder-row"
        :class="{ active: allSelected }"
        type="button"
        @click="emit('select', null)"
      >
        <Images :size="17" />
        <span class="folder-name">Toutes les images</span>
        <span class="folder-count">{{ totalImages }}</span>
      </button>

      <p class="section-label followed-label">Dossiers suivis</p>
      <div v-if="folders.length === 0" class="sidebar-empty">
        <FolderOpen :size="19" />
        <span>Aucun dossier suivi</span>
      </div>

      <div v-for="folder in folders" :key="folder.id" class="folder-entry">
        <button
          class="folder-row"
          :class="{ active: selectedFolderId === folder.id }"
          type="button"
          :title="folder.path"
          @click="emit('select', folder.id)"
        >
          <Folder :size="17" />
          <span class="folder-name">{{ folder.name }}</span>
          <span class="folder-count">{{ folder.imageCount }}</span>
        </button>
        <div class="folder-actions">
          <button type="button" title="Réindexer" @click.stop="emit('reindex', folder.id)">
            <RefreshCw :size="14" />
          </button>
          <button type="button" title="Ne plus suivre" @click.stop="emit('remove', folder.id)">
            <Trash2 :size="14" />
          </button>
        </div>
      </div>
    </nav>
  </aside>
</template>
