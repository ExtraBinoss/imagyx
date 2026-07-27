<script setup lang="ts">
import { computed } from 'vue'
import { Check, FolderSync, LoaderCircle, TriangleAlert } from '@lucide/vue'
import type { SpotlightIndexJob } from './types'

const props = defineProps<{ job: SpotlightIndexJob }>()

const percent = computed(() => {
  const determinate = props.job.stage === 'embedding' || props.job.stage === 'complete' || props.job.current > 0
  if (!determinate || props.job.total <= 0) return null
  return Math.min(100, Math.max(0, props.job.current / props.job.total * 100))
})
const complete = computed(() => props.job.stage === 'complete')
const failed = computed(() => props.job.stage === 'error')
</script>

<template>
  <article class="index-job" :class="{ 'index-job--complete': complete, 'index-job--error': failed }">
    <span class="index-job__icon">
      <Check v-if="complete" :size="17" />
      <TriangleAlert v-else-if="failed" :size="17" />
      <LoaderCircle v-else-if="job.stage === 'embedding'" class="spin" :size="17" />
      <FolderSync v-else :size="17" />
    </span>
    <div class="index-job__copy">
      <strong>{{ job.folderName }}</strong>
      <span>{{ job.message }}</span>
      <div class="index-job__track" :class="{ 'index-job__track--indeterminate': percent == null && !complete && !failed }">
        <i :style="percent == null ? undefined : { width: `${percent}%` }" />
      </div>
    </div>
    <span v-if="percent != null && !complete" class="index-job__percent">{{ Math.round(percent) }}%</span>
  </article>
</template>

<style scoped>
.index-job {
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr) auto;
  align-items: center;
  gap: 11px;
  margin: 4px 4px 8px;
  padding: 11px 12px;
  border: 1px solid color-mix(in srgb, var(--primary) 22%, var(--border));
  border-radius: 13px;
  background: color-mix(in srgb, var(--primary-soft) 48%, var(--surface));
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.12), 0 8px 18px -17px rgb(15 23 42 / 0.35);
  animation: job-enter 240ms cubic-bezier(0.16, 1, 0.3, 1) both;
}
.index-job__icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border: 1px solid color-mix(in srgb, var(--primary) 24%, var(--border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--surface) 88%, transparent);
  color: var(--primary-text);
}
.index-job__copy { min-width: 0; }
.index-job__copy strong,
.index-job__copy span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.index-job__copy strong { color: var(--text); font-size: 11px; }
.index-job__copy span { margin-top: 4px; color: var(--text-muted); font-size: 9px; }
.index-job__track {
  position: relative;
  height: 3px;
  margin-top: 8px;
  overflow: hidden;
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--border) 76%, transparent);
}
.index-job__track i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--primary), #93c5fd);
  transition: width 220ms cubic-bezier(0.16, 1, 0.3, 1);
}
.index-job__track--indeterminate i {
  width: 34%;
  animation: progress-slide 1.1s ease-in-out infinite;
}
.index-job__percent { color: var(--primary-text); font-size: 10px; font-weight: 720; font-variant-numeric: tabular-nums; }
.index-job--complete { border-color: var(--success-border); background: var(--success-surface); }
.index-job--complete .index-job__icon { color: var(--success-text); border-color: var(--success-border); }
.index-job--error { border-color: var(--danger-border); background: var(--danger-surface); }
.index-job--error .index-job__icon { color: var(--danger-text); border-color: var(--danger-border); }
.spin { animation: spin 0.8s linear infinite; }
@keyframes progress-slide { from { transform: translate3d(-120%, 0, 0); } to { transform: translate3d(310%, 0, 0); } }
@keyframes job-enter { from { opacity: 0; transform: translate3d(0, -7px, 0) scale(0.985); } to { opacity: 1; transform: none; } }
@keyframes spin { to { transform: rotate(1turn); } }
</style>
