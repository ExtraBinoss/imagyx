<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Check, Copy, Cpu, FolderSync, Gauge, LoaderCircle, MemoryStick, Pause, Play } from '@lucide/vue'
import type { IndexProgress, ModelDownloadProgress, RuntimeStats } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Button from './ui/Button/Button.vue'
import Popover from './ui/Popover/Popover.vue'

import { usePlatformStore } from '../stores/platform'
import { useLibraryStore } from '../stores/library'

const props = defineProps<{
  progress: IndexProgress | null
  modelProgress: ModelDownloadProgress | null
  runtimeStats: RuntimeStats | null
}>()

const emit = defineEmits<{ pause: []; resume: [] }>()
const platform = usePlatformStore()
const library = useLibraryStore()
const liveStats = ref<RuntimeStats | null>(props.runtimeStats)
const popoverOpen = ref(false)
const copied = ref(false)
let timer: number | undefined

watch(() => props.runtimeStats, (value) => { if (value) liveStats.value = value }, { immediate: true })

const paused = computed(() => liveStats.value?.stage === 'paused')
const indexing = computed(() =>
  ['decoding', 'inference', 'indexing', 'saving'].includes(liveStats.value?.stage ?? ''),
)
const downloading = computed(() => props.modelProgress?.stage === 'downloading')
const preparingModel = computed(() =>
  props.modelProgress?.stage === 'checking' || props.modelProgress?.stage === 'loading',
)
const active = computed(() => indexing.value || downloading.value || preparingModel.value)
const current = computed(() => (indexing.value || paused.value) ? liveStats.value?.current ?? 0 : props.modelProgress?.currentBytes ?? 0)
const total = computed(() => (indexing.value || paused.value) ? liveStats.value?.total ?? 0 : props.modelProgress?.totalBytes ?? 0)
const percent = computed(() => total.value > 0 ? Math.min(100, Math.max(0, (current.value / total.value) * 100)) : (active.value ? null : 100))
const indeterminate = computed(() => preparingModel.value || (indexing.value && current.value === 0))

const title = computed(() => {
  if (paused.value) return 'Indexing Paused'
  if (downloading.value) return 'Downloading AI Model'
  if (indexing.value) return liveStats.value?.stage === 'saving' ? 'Saving Analysis' : 'WebGPU AI Analysis'
  if (preparingModel.value) return 'Loading Model'
  if (props.modelProgress?.stage === 'error') return 'AI Unavailable'
  return 'Indexing Completed'
})

const detail = computed(() => {
  if (indexing.value || paused.value) {
    const speed = liveStats.value?.imagesPerSecond ?? 0
    return `${current.value} / ${total.value}${speed > 0 ? ` · ${speed.toFixed(1)} img/s` : ''}`
  }
  if (downloading.value && props.modelProgress) {
    if (props.modelProgress.totalBytes > 0) {
      return `${formatBytes(props.modelProgress.currentBytes)} / ${formatBytes(props.modelProgress.totalBytes)}`
    }
    return props.modelProgress.fileName ?? 'Downloading…'
  }
  return liveStats.value?.accelerationLabel ?? 'Local AI Ready'
})

const phaseLabel = computed(() => {
  switch (liveStats.value?.stage) {
    case 'checking': return 'Checking Cache'
    case 'loading': return 'Loading Model'
    case 'loading-text': return 'Loading Text Encoder'
    case 'decoding': return 'Decoding Images'
    case 'inference': return 'WebGPU Inference'
    case 'indexing': return 'Semantic Indexing'
    case 'saving': return 'Writing to SQLite'
    case 'paused': return 'Resume Available'
    case 'error': return 'Error'
    default: return 'Ready'
  }
})

async function refreshStats() {
  try { liveStats.value = await imagyxApi.runtimeStats() } catch { /* ignore */ }
}

watch(popoverOpen, (open) => { if (open) void refreshStats() })
onMounted(() => {
  void refreshStats()
  timer = window.setInterval(() => { if (popoverOpen.value || active.value || paused.value) void refreshStats() }, 1200)
})
onBeforeUnmount(() => { if (timer) window.clearInterval(timer) })

import { copyDebugInfoToClipboard } from '../utils/copy-information'

async function copyAiDetails() {
  await copyDebugInfoToClipboard({
    appVersion: platform.appVersion,
    platform: platform.platform,
    databasePath: library.appInfo?.databasePath,
    runtimeStats: liveStats.value,
    progress: props.progress,
    lastIndexedAt: library.lastIndexedAt,
    foldersCount: library.folders.length,
    totalImagesCount: library.totalImages,
    phaseLabel: phaseLabel.value,
    currentProgress: current.value,
    totalProgress: total.value,
  })
  copied.value = true
  setTimeout(() => {
    copied.value = false
  }, 2000)
}
</script>

<template>
  <Transition name="fade">
    <div v-if="active || paused || platform.isDev" class="sidebar-status-container">
      <Popover side="top" align="start" width="280px">
        <template #trigger="{ open }">
          <article
            class="sidebar-index-card"
            :class="{
              'sidebar-index-card--active': active,
              'sidebar-index-card--open': open
            }"
            role="button"
            tabindex="0"
            title="Détails de l'indexation IA"
            @click="popoverOpen = !open"
          >
            <span class="card-icon">
              <Pause v-if="paused" :size="16" />
              <LoaderCircle v-else-if="indexing || downloading" class="spin" :size="16" />
              <FolderSync v-else :size="16" />
            </span>

            <div class="card-copy">
              <div class="card-title-row">
                <strong>{{ title }}</strong>
                <span v-if="percent != null && active && !indeterminate" class="card-percent">{{ Math.round(percent) }}%</span>
              </div>
              <span class="card-detail">{{ detail }}</span>

              <!-- Spotlight style progress bar -->
              <div class="card-track" :class="{ 'card-track--indeterminate': indeterminate }">
                <i :style="percent != null ? { width: `${percent}%` } : undefined" />
              </div>
            </div>

            <Button
              v-if="indexing || paused"
              class="card-control"
              variant="ghost"
              size="icon"
              :aria-label="paused ? 'Reprendre' : 'Pause'"
              @click.stop="emit(paused ? 'resume' : 'pause')"
            >
              <Play v-if="paused" :size="14" />
              <Pause v-else :size="14" />
            </Button>
          </article>
        </template>

        <template #content>
          <div class="ai-stats">
            <div class="ai-stats__title">
              <div>
                <strong>{{ liveStats?.modelName ?? 'MobileCLIP-S0' }}</strong>
                <span>{{ phaseLabel }}</span>
              </div>
            </div>

            <dl class="ai-simple-dl">
              <div><dt>Progress</dt><dd>{{ current }} / {{ total }}</dd></div>
              <div v-if="liveStats?.imagesPerSecond"><dt>Speed</dt><dd>{{ liveStats.imagesPerSecond.toFixed(1) }} img/s</dd></div>
              <div><dt>Resources</dt><dd>CPU {{ liveStats?.processCpuPercent?.toFixed(0) ?? 0 }}% · RAM {{ formatBytes(liveStats?.processMemoryBytes ?? 0) }}</dd></div>
              <div><dt>Hardware Accelerated</dt><dd>{{ liveStats?.accelerationActive ? 'OK' : 'KO' }}</dd></div>
            </dl>

            <Button variant="secondary" size="sm" block class="ai-copy-btn" @click="copyAiDetails">
              <template #leading>
                <Check v-if="copied" :size="14" />
                <Copy v-else :size="14" />
              </template>
              {{ copied ? 'Copied!' : 'Copy additional information' }}
            </Button>

            <p v-if="liveStats?.fallbackReason" class="ai-stats__warning">{{ liveStats.fallbackReason }}</p>
          </div>
        </template>
      </Popover>
    </div>
  </Transition>
</template>

<style scoped>
.sidebar-status-container {
  width: 100%;
}

.sidebar-status-container :deep(.ui-popover),
.sidebar-status-container :deep(.ui-popover__trigger) {
  width: 100%;
  display: flex;
}

.sidebar-index-card {
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.sidebar-index-card:hover,
.sidebar-index-card--open {
  border-color: var(--border-strong);
  background: var(--surface-hover);
}

.sidebar-index-card--active {
  border-color: color-mix(in srgb, var(--primary) 35%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 30%, var(--surface));
}

.card-icon {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
  color: var(--primary-text);
  border: 1px solid var(--border);
}

.card-copy {
  min-width: 0;
}

.card-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.card-copy strong {
  color: var(--text);
  font-size: var(--text-xs);
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-percent {
  color: var(--primary-text);
  font-size: 10px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.card-detail {
  display: block;
  margin-top: 2px;
  color: var(--text-muted);
  font-size: 10px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-track {
  position: relative;
  height: 3px;
  margin-top: 6px;
  overflow: hidden;
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--border) 80%, transparent);
}

.card-track i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--primary), var(--primary-hover));
  transition: width 200ms ease;
}

.card-track--indeterminate i {
  width: 35%;
  animation: progress-slide 1.2s ease-in-out infinite;
}

.card-control {
  width: 26px;
  height: 26px;
  padding: 0;
}

.ai-stats {
  display: grid;
  gap: var(--space-3);
}

.ai-stats__title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding-bottom: var(--space-2);
  border-bottom: 1px solid var(--border);
}

.ai-stats__title strong {
  font-size: var(--text-sm);
  color: var(--text);
}

.ai-stats__title span {
  display: block;
  margin-top: 2px;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.ai-stats dl {
  display: grid;
  gap: 6px;
  margin: 0;
}

.ai-stats dl div {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  font-size: var(--text-xs);
}

.ai-stats dt {
  color: var(--text-muted);
}

.ai-simple-dl {
  display: grid;
  gap: 6px;
  margin: 0;
}

.ai-simple-dl div {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  font-size: var(--text-xs);
}

.ai-simple-dl dt {
  color: var(--text-muted);
}

.ai-simple-dl dd {
  margin: 0;
  color: var(--text);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}

.ai-copy-btn {
  margin-top: var(--space-1);
}

.ai-stats__warning {
  margin: 0;
  color: var(--warning-text);
  font-size: var(--text-xs);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.spin {
  animation: spin 0.8s linear infinite;
}

@keyframes progress-slide {
  from { transform: translateX(-120%); }
  to { transform: translateX(310%); }
}

@keyframes spin {
  to { transform: rotate(1turn); }
}
</style>
