<script setup lang="ts">
import { Database, LoaderCircle } from '@lucide/vue'
import type { IndexProgress } from '../types'

defineProps<{
  progress: IndexProgress | null
  databasePath: string
}>()
</script>

<template>
  <footer class="status-bar">
    <div class="status-copy">
      <template v-if="progress && progress.stage !== 'complete'">
        <LoaderCircle class="spin" :size="14" />
        <span>{{ progress.message }}</span>
      </template>
      <template v-else>
        <span class="status-dot" />
        <span>Bibliothèque locale prête</span>
      </template>
    </div>
    <div class="database-location" :title="databasePath">
      <Database :size="14" />
      <span>{{ databasePath }}</span>
    </div>
  </footer>
</template>
