<script setup lang="ts">
import { computed } from 'vue'
import { Database, LoaderCircle } from '@lucide/vue'
import type { IndexProgress } from '../types'
import ProgressBar from './ui/ProgressBar/ProgressBar.vue'
import Tooltip from './ui/Tooltip/Tooltip.vue'

const props = defineProps<{
  progress: IndexProgress | null
  databasePath: string
}>()

const active = computed(() => Boolean(props.progress && props.progress.stage !== 'complete'))
const percentage = computed(() => {
  if (!props.progress || props.progress.total === 0) return 0
  return Math.min(100, (props.progress.current / props.progress.total) * 100)
})
</script>

<template>
  <footer class="status-bar">
    <div class="status-copy">
      <template v-if="active && progress">
        <LoaderCircle class="spin" :size="14" />
        <span>{{ progress.message }}</span>
        <ProgressBar
          class="status-progress"
          :value="percentage"
          :indeterminate="progress.total === 0"
          size="sm"
          label="Progression de l’indexation"
        />
        <span v-if="progress.total > 0" class="status-count">
          {{ progress.current }} / {{ progress.total }}
        </span>
      </template>
      <template v-else>
        <span class="status-dot" />
        <span>Bibliothèque locale prête</span>
      </template>
    </div>

    <Tooltip v-if="databasePath" :text="databasePath" side="top">
      <div class="database-location">
        <Database :size="14" />
        <span>{{ databasePath }}</span>
      </div>
    </Tooltip>
  </footer>
</template>
