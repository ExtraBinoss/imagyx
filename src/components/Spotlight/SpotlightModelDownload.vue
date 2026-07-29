<script setup lang="ts">
import { computed } from 'vue'
import { Download, LoaderCircle } from '@lucide/vue'
import type { ModelDownloadProgress } from '../../types'
import { formatBytes } from '../../utils'
import { useTranslate } from '../../i18n'

const props = defineProps<{ progress: ModelDownloadProgress }>()

const { t } = useTranslate()
const downloading = computed(() => props.progress.stage === 'downloading')
const percent = computed(() => {
  if (!props.progress.totalBytes) return null
  return Math.min(100, Math.max(0, (props.progress.currentBytes / props.progress.totalBytes) * 100))
})
const title = computed(() => downloading.value
  ? t('indexing.title.downloading')
  : t('indexing.title.loading'))
const detail = computed(() => {
  if (downloading.value && props.progress.totalBytes > 0) {
    return t('indexing.message.downloading', {
      current: formatBytes(props.progress.currentBytes),
      total: formatBytes(props.progress.totalBytes),
    })
  }
  return props.progress.message || props.progress.fileName || t('indexing.message.downloading_name')
})
</script>

<template>
  <section class="spotlight-model-download" aria-live="polite" aria-busy="true">
    <span class="spotlight-model-download__icon">
      <Download v-if="downloading" :size="22" />
      <LoaderCircle v-else class="spin" :size="22" />
    </span>
    <strong>{{ title }}</strong>
    <p>{{ detail }}</p>
    <div class="spotlight-model-download__track" :class="{ 'spotlight-model-download__track--indeterminate': percent == null }">
      <i :style="percent != null ? { width: `${percent}%` } : undefined" />
    </div>
    <small v-if="progress.totalFiles > 1">{{ progress.currentFile }}/{{ progress.totalFiles }}</small>
  </section>
</template>

<style scoped>
.spotlight-model-download {
  display: grid;
  place-items: center;
  align-content: center;
  min-height: 330px;
  padding: 34px;
  color: var(--text-muted);
  text-align: center;
}
.spotlight-model-download__icon {
  display: grid;
  place-items: center;
  width: 52px;
  height: 52px;
  margin-bottom: 15px;
  border: 1px solid color-mix(in srgb, var(--primary) 30%, var(--border));
  border-radius: 16px;
  background: color-mix(in srgb, var(--primary-soft) 72%, var(--surface));
  color: var(--primary-text);
}
.spotlight-model-download strong { color: var(--text); font-size: 15px; }
.spotlight-model-download p { max-width: 310px; margin: 8px 0 13px; font-size: 11px; line-height: 1.5; }
.spotlight-model-download__track {
  width: min(280px, 100%);
  height: 6px;
  overflow: hidden;
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--border-strong) 55%, transparent);
}
.spotlight-model-download__track i {
  display: block;
  width: 0;
  height: 100%;
  border-radius: inherit;
  background: var(--primary);
  transition: width 180ms ease;
}
.spotlight-model-download__track--indeterminate i { width: 38%; animation: model-download-progress 1.1s ease-in-out infinite; }
.spotlight-model-download small { margin-top: 8px; font-size: 10px; font-variant-numeric: tabular-nums; }
.spin { animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(1turn); } }
@keyframes model-download-progress { 0% { transform: translate3d(-120%, 0, 0); } 100% { transform: translate3d(320%, 0, 0); } }
@media (prefers-reduced-motion: reduce) {
  .spin, .spotlight-model-download__track--indeterminate i { animation-duration: 0.01ms; }
  .spotlight-model-download__track i { transition-duration: 0.01ms; }
}
</style>
