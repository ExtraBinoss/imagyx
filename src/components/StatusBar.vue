<script setup lang="ts">
import { computed } from 'vue'
import { Clock3, LoaderCircle } from '@lucide/vue'
import type { IndexProgress } from '../types'
import ProgressBar from './ui/ProgressBar/ProgressBar.vue'

const props = defineProps<{
  progress: IndexProgress | null
  databasePath?: string
}>()

const active = computed(() => Boolean(props.progress && props.progress.stage !== 'complete'))
const queued = computed(() => props.progress?.stage === 'queued')
const indeterminate = computed(() =>
  Boolean(
    props.progress &&
      (props.progress.total === 0 ||
        (props.progress.stage === 'embedding' && props.progress.current === 0)),
  ),
)
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
  <footer v-if="active && progress" class="status-bar">
    <div class="status-copy">
      <Clock3 v-if="queued" :size="14" />
      <LoaderCircle v-else class="spin" :size="14" />
      <span>{{ progress.message }}</span>
      <template v-if="!queued">
        <ProgressBar
          class="status-progress"
          :value="percentage"
          :indeterminate="indeterminate"
          size="sm"
          label="Progression de l’indexation"
        />
        <span v-if="countLabel" class="status-count">{{ countLabel }}</span>
      </template>
    </div>
  </footer>
</template>
