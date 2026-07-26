<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Cpu, Gauge, MemoryStick, Sparkles } from '@lucide/vue'
import type { IndexProgress, ModelDownloadProgress, RuntimeStats } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Popover from './ui/Popover/Popover.vue'
import ProgressBar from './ui/ProgressBar/ProgressBar.vue'
import DisclosureButton from './ui/DisclosureButton/DisclosureButton.vue'

const props = defineProps<{
  progress: IndexProgress | null
  modelProgress: ModelDownloadProgress | null
}>()

const stats = ref<RuntimeStats | null>(null)
const popoverOpen = ref(false)
let timer: number | undefined

const indexing = computed(() => props.progress?.stage === 'embedding')
const downloading = computed(() => props.modelProgress?.stage === 'downloading')
const active = computed(() => indexing.value || downloading.value || props.modelProgress?.stage === 'loading')
const current = computed(() => indexing.value ? props.progress?.current ?? 0 : props.modelProgress?.currentBytes ?? 0)
const total = computed(() => indexing.value ? props.progress?.total ?? 0 : props.modelProgress?.totalBytes ?? 0)
const percent = computed(() => total.value > 0 ? Math.min(100, (current.value / total.value) * 100) : 0)
const title = computed(() => {
  if (downloading.value) return 'Téléchargement de l’IA'
  if (indexing.value) return 'Analyse IA'
  if (props.modelProgress?.stage === 'loading') return 'Chargement du modèle'
  if (props.modelProgress?.stage === 'error') return 'IA indisponible'
  return 'IA locale prête'
})
const detail = computed(() => {
  if (indexing.value && props.progress) return `${props.progress.current} / ${props.progress.total}`
  if (downloading.value && props.modelProgress) {
    return `${formatBytes(props.modelProgress.currentBytes)} / ${formatBytes(props.modelProgress.totalBytes)}`
  }
  return stats.value?.gpuActive ? `GPU · ${stats.value.backend}` : stats.value?.backend ?? 'MobileCLIP2-S0'
})

async function refreshStats(): Promise<void> {
  try {
    stats.value = await imagyxApi.runtimeStats()
  } catch {
    // The compact status remains usable even if diagnostics are unavailable.
  }
}

watch(popoverOpen, (open) => {
  if (open) void refreshStats()
})

onMounted(() => {
  void refreshStats()
  timer = window.setInterval(() => {
    if (popoverOpen.value || active.value) void refreshStats()
  }, 1800)
})

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer)
})
</script>

<template>
  <div class="local-ai-status">
    <div class="local-ai-status__heading">
      <span class="local-ai-status__icon"><Sparkles :size="15" /></span>
      <div>
        <strong>{{ title }}</strong>
        <span>{{ detail }}</span>
      </div>
    </div>

    <ProgressBar
      class="local-ai-status__progress"
      :value="percent"
      :indeterminate="active && total === 0"
      size="sm"
      label="Progression de l’IA locale"
    />

    <Popover align="start" width="300px">
      <template #trigger="{ open }">
        <DisclosureButton
          :open="open"
          label="Voir les performances"
          @toggle="popoverOpen = !popoverOpen"
        />
      </template>
      <template #content>
        <div class="ai-stats">
          <div class="ai-stats__title">
            <div>
              <strong>{{ stats?.modelName ?? 'MobileCLIP2-S0' }}</strong>
              <span>Moteur local</span>
            </div>
            <Badge :variant="stats?.gpuActive ? 'success' : 'neutral'">
              {{ stats?.gpuActive ? 'GPU actif' : stats?.acceleration ?? 'CPU' }}
            </Badge>
          </div>

          <dl>
            <div><dt>Backend</dt><dd>{{ stats?.backend ?? 'Chargement…' }}</dd></div>
            <div><dt>Lot</dt><dd>{{ stats?.batchSize ?? 0 }} images</dd></div>
            <div><dt>Progression</dt><dd>{{ stats?.current ?? 0 }} / {{ stats?.total ?? 0 }}</dd></div>
            <div><dt>Débit</dt><dd>{{ stats?.imagesPerSecond?.toFixed(1) ?? '—' }} img/s</dd></div>
            <div><dt>Temps moyen</dt><dd>{{ stats?.averageMsPerImage?.toFixed(0) ?? '—' }} ms/image</dd></div>
          </dl>

          <div class="ai-stats__resources">
            <div>
              <Cpu :size="15" />
              <span>Imagyx {{ stats?.processCpuPercent.toFixed(0) ?? 0 }} % CPU</span>
            </div>
            <div>
              <Gauge :size="15" />
              <span>Système {{ stats?.systemCpuPercent.toFixed(0) ?? 0 }} % CPU</span>
            </div>
            <div>
              <MemoryStick :size="15" />
              <span>{{ formatBytes(stats?.processMemoryBytes ?? 0) }} RAM</span>
            </div>
          </div>

          <p v-if="stats?.lastError" class="ai-stats__error">{{ stats.lastError }}</p>
        </div>
      </template>
    </Popover>
  </div>
</template>

<style scoped>
.local-ai-status {
  display: grid;
  gap: var(--space-3);
  margin-top: auto;
  padding: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
}

.local-ai-status__heading {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.local-ai-status__icon {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: var(--radius-md);
  background: var(--primary-soft);
  color: var(--primary-text);
}

.local-ai-status__heading div {
  min-width: 0;
}

.local-ai-status__heading strong,
.local-ai-status__heading span {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.local-ai-status__heading strong {
  font-size: var(--text-xs);
}

.local-ai-status__heading span {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 10px;
}

.local-ai-status__progress {
  width: 100%;
}

.ai-stats {
  display: grid;
  gap: var(--space-4);
}

.ai-stats__title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}

.ai-stats__title strong,
.ai-stats__title span {
  display: block;
}

.ai-stats__title strong {
  font-size: var(--text-sm);
}

.ai-stats__title span {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.ai-stats dl {
  display: grid;
  gap: var(--space-2);
  margin: 0;
}

.ai-stats dl div {
  display: flex;
  justify-content: space-between;
  gap: var(--space-4);
  font-size: var(--text-xs);
}

.ai-stats dt {
  color: var(--text-muted);
}

.ai-stats dd {
  margin: 0;
  color: var(--text);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.ai-stats__resources {
  display: grid;
  gap: var(--space-2);
  padding-top: var(--space-3);
  border-top: 1px solid var(--border);
}

.ai-stats__resources div {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.ai-stats__error {
  margin: 0;
  color: var(--danger-text);
  font-size: var(--text-xs);
  line-height: 1.45;
}
</style>
