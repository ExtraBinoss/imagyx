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
  runtimeStats: RuntimeStats | null
}>()

const liveStats = ref<RuntimeStats | null>(props.runtimeStats)
const popoverOpen = ref(false)
let timer: number | undefined

watch(
  () => props.runtimeStats,
  (value) => {
    if (value) liveStats.value = value
  },
  { immediate: true },
)

const indexing = computed(() =>
  props.progress?.stage === 'embedding' || props.progress?.stage === 'saving',
)
const downloading = computed(() => props.modelProgress?.stage === 'downloading')
const preparingModel = computed(() =>
  props.modelProgress?.stage === 'checking' || props.modelProgress?.stage === 'loading',
)
const active = computed(() => indexing.value || downloading.value || preparingModel.value)
const current = computed(() => indexing.value ? props.progress?.current ?? 0 : props.modelProgress?.currentBytes ?? 0)
const total = computed(() => indexing.value ? props.progress?.total ?? 0 : props.modelProgress?.totalBytes ?? 0)
const percent = computed(() => total.value > 0 ? Math.min(100, (current.value / total.value) * 100) : 0)
const indeterminate = computed(() =>
  preparingModel.value || (indexing.value && current.value === 0),
)

const title = computed(() => {
  if (downloading.value) return 'Téléchargement de l’IA'
  if (indexing.value) return props.progress?.stage === 'saving' ? 'Sauvegarde de l’analyse' : 'Analyse IA'
  if (preparingModel.value) return 'Chargement du modèle'
  if (props.modelProgress?.stage === 'error') return 'IA indisponible'
  return 'IA locale prête'
})

const detail = computed(() => {
  if (indexing.value && props.progress) {
    if (props.progress.current === 0) {
      return props.progress.batchCurrent
        ? `Lot ${props.progress.batchCurrent} · préparation`
        : 'Préparation du premier lot'
    }
    const speed = liveStats.value?.imagesPerSecond ?? 0
    return `${props.progress.current} / ${props.progress.total}${speed > 0 ? ` · ${speed.toFixed(1)} img/s` : ''}`
  }
  if (downloading.value && props.modelProgress) {
    return `${formatBytes(props.modelProgress.currentBytes)} / ${formatBytes(props.modelProgress.totalBytes)}`
  }
  return liveStats.value?.accelerationActive
    ? liveStats.value.accelerationLabel
    : liveStats.value?.backendEffective ?? 'MobileCLIP2-S0'
})

const phaseLabel = computed(() => {
  switch (liveStats.value?.stage) {
    case 'checking': return 'Vérification du cache'
    case 'loading': return 'Optimisation du modèle'
    case 'decoding': return 'Décodage et resize 256 px'
    case 'indexing': return 'Inférence MobileCLIP'
    case 'saving': return 'Écriture SQLite'
    case 'error': return 'Erreur'
    default: return 'Prêt'
  }
})

async function refreshStats(): Promise<void> {
  try {
    liveStats.value = await imagyxApi.runtimeStats()
  } catch {
    // La carte compacte reste utilisable même si les diagnostics sont indisponibles.
  }
}

watch(popoverOpen, (open) => {
  if (open) void refreshStats()
})

onMounted(() => {
  void refreshStats()
  timer = window.setInterval(() => {
    if (popoverOpen.value || active.value) void refreshStats()
  }, 1500)
})

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer)
})

function formatMilliseconds(value: number): string {
  if (value < 1_000) return `${Math.round(value)} ms`
  return `${(value / 1_000).toFixed(1)} s`
}
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
      :indeterminate="indeterminate"
      size="sm"
      label="Progression de l’IA locale"
    />

    <Popover align="start" width="320px">
      <template #trigger="{ open }">
        <DisclosureButton
          :open="open"
          label="Voir les performances"
          @toggle="popoverOpen = !open"
        />
      </template>
      <template #content>
        <div class="ai-stats">
          <div class="ai-stats__title">
            <div>
              <strong>{{ liveStats?.modelName ?? 'MobileCLIP2-S0' }}</strong>
              <span>{{ phaseLabel }}</span>
            </div>
            <Badge :variant="liveStats?.accelerationActive ? 'success' : 'neutral'">
              {{ liveStats?.accelerationActive ? liveStats.accelerationLabel : 'CPU' }}
            </Badge>
          </div>

          <dl>
            <div><dt>Backend actif</dt><dd>{{ liveStats?.backendEffective ?? 'Chargement…' }}</dd></div>
            <div><dt>Backend demandé</dt><dd>{{ liveStats?.backendRequested ?? 'Automatique' }}</dd></div>
            <div><dt>Lot</dt><dd>{{ liveStats?.batchCurrent ?? 0 }} / {{ liveStats?.batchTotal ?? 0 }} · {{ liveStats?.batchSize ?? 0 }} images</dd></div>
            <div><dt>Progression</dt><dd>{{ liveStats?.current ?? 0 }} / {{ liveStats?.total ?? 0 }}</dd></div>
            <div><dt>Débit</dt><dd>{{ liveStats?.imagesPerSecond?.toFixed(1) ?? '0.0' }} img/s</dd></div>
            <div><dt>Temps moyen</dt><dd>{{ liveStats?.averageMsPerImage ? `${liveStats.averageMsPerImage.toFixed(0)} ms/image` : '—' }}</dd></div>
          </dl>

          <div class="ai-stats__timings">
            <span>Décodage <strong>{{ formatMilliseconds(liveStats?.decodeMs ?? 0) }}</strong></span>
            <span>Inférence <strong>{{ formatMilliseconds(liveStats?.inferenceMs ?? 0) }}</strong></span>
            <span>SQLite <strong>{{ formatMilliseconds(liveStats?.saveMs ?? 0) }}</strong></span>
          </div>

          <div class="ai-stats__resources">
            <div>
              <Cpu :size="15" />
              <span>Imagyx {{ liveStats?.processCpuPercent.toFixed(0) ?? 0 }} % CPU</span>
            </div>
            <div>
              <Gauge :size="15" />
              <span>Système {{ liveStats?.systemCpuPercent.toFixed(0) ?? 0 }} % CPU</span>
            </div>
            <div>
              <MemoryStick :size="15" />
              <span>{{ formatBytes(liveStats?.processMemoryBytes ?? 0) }} RAM · modèle {{ formatBytes(liveStats?.modelCacheBytes ?? 0) }}</span>
            </div>
          </div>

          <p class="ai-stats__cache">
            {{ liveStats?.thumbnailCacheItems ?? 0 }} miniatures en cache sur 256 maximum
          </p>
          <p v-if="liveStats?.fallbackReason" class="ai-stats__warning">
            Accélération indisponible : {{ liveStats.fallbackReason }}
          </p>
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

.ai-stats__timings {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-2);
}

.ai-stats__timings span {
  display: grid;
  gap: 3px;
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-muted);
  font-size: 10px;
}

.ai-stats__timings strong {
  color: var(--text);
  font-size: var(--text-xs);
  font-variant-numeric: tabular-nums;
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

.ai-stats__cache,
.ai-stats__warning {
  margin: 0;
  font-size: var(--text-xs);
  line-height: 1.45;
}

.ai-stats__cache {
  color: var(--text-muted);
}

.ai-stats__warning {
  color: var(--warning-text, var(--text-muted));
}
</style>
