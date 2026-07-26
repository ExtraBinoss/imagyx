<script setup lang="ts">
import { computed } from 'vue'
import { Clock3, Database, LoaderCircle } from '@lucide/vue'
import type { IndexProgress } from '../types'
import ProgressBar from './ui/ProgressBar/ProgressBar.vue'
import Tooltip from './ui/Tooltip/Tooltip.vue'

const props = defineProps<{
  progress: IndexProgress | null
  databasePath: string
}>()

const active = computed(() => Boolean(props.progress && props.progress.stage !== 'complete'))
const queued = computed(() => props.progress?.stage === 'queued')
const percentage = computed(() => {
  if (!props.progress || props.progress.total === 0) return 0
  return Math.min(100, (props.progress.current / props.progress.total) * 100)
})
const countLabel = computed(() => {
  const progress = props.progress
  if (!progress) return ''
  if (progress.batchCurrent && progress.batchTotal) {
    const images = progress.current > 0 ? ` · ${progress.current} / ${progress.total}` : ''
    return `Lot ${progress.batchCurrent} / ${progress.batchTotal}${images}`
  }
  return progress.total > 0 ? `${progress.current} / ${progress.total}` : ''
})
</script>

<template>
  <footer class="status-bar">
    <div class="status-copy">
      <template v-if="active && progress">
        <Clock3 v-if="queued" :size="14" />
        <LoaderCircle v-else class="spin" :size="14" />
        <span>{{ progress.message }}</span>
        <template v-if="!queued">
          <ProgressBar
            class="status-progress"
            :value="percentage"
            :indeterminate="progress.total === 0"
            size="sm"
            label="Progression de l’indexation"
          />
          <span v-if="countLabel" class="status-count">{{ countLabel }}</span>
        </template>
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
