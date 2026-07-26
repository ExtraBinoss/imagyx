<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Cpu, Gauge, MemoryStick, Pause, Play, Sparkles } from '@lucide/vue'
import type { IndexProgress, ModelDownloadProgress, RuntimeStats } from '../types'
import { imagyxApi } from '../api/tauri'
import { formatBytes } from '../utils'
import Badge from './ui/Badge/Badge.vue'
import Button from './ui/Button/Button.vue'
import Popover from './ui/Popover/Popover.vue'
import ProgressBar from './ui/ProgressBar/ProgressBar.vue'
import DisclosureButton from './ui/DisclosureButton/DisclosureButton.vue'

const props = defineProps<{
  progress: IndexProgress | null
  modelProgress: ModelDownloadProgress | null
  runtimeStats: RuntimeStats | null
}>()

const emit = defineEmits<{ pause: []; resume: [] }>()
const liveStats = ref<RuntimeStats | null>(props.runtimeStats)
const popoverOpen = ref(false)
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
const percent = computed(() => total.value > 0 ? Math.min(100, (current.value / total.value) * 100) : 0)
const indeterminate = computed(() => preparingModel.value || (indexing.value && current.value === 0))

const title = computed(() => {
  if (paused.value) return 'Indexation en pause'
  if (downloading.value) return 'Téléchargement de l’IA'
  if (indexing.value) return liveStats.value?.stage === 'saving' ? 'Sauvegarde de l’analyse' : 'Analyse IA WebGPU'
  if (preparingModel.value) return 'Chargement du modèle'
  if (props.modelProgress?.stage === 'error') return 'IA indisponible'
  return 'IA locale prête'
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
    return props.modelProgress.fileName ?? 'Téléchargement…'
  }
  return liveStats.value?.accelerationLabel ?? 'MobileCLIP-S0'
})

const phaseLabel = computed(() => {
  switch (liveStats.value?.stage) {
    case 'checking': return 'Vérification du cache navigateur'
    case 'loading': return 'Chargement du modèle'
    case 'loading-text': return 'Chargement de l’encodeur texte'
    case 'decoding': return 'Décodage des images'
    case 'inference': return 'Inférence WebGPU'
    case 'indexing': return 'Indexation sémantique'
    case 'saving': return 'Écriture SQLite'
    case 'paused': return 'Reprise disponible'
    case 'error': return 'Erreur'
    default: return 'Prêt'
  }
})

async function refreshStats() {
  try { liveStats.value = await imagyxApi.runtimeStats() } catch { /* diagnostics facultatifs */ }
}

watch(popoverOpen, (open) => { if (open) void refreshStats() })
onMounted(() => {
  void refreshStats()
  timer = window.setInterval(() => { if (popoverOpen.value || active.value || paused.value) void refreshStats() }, 1200)
})
onBeforeUnmount(() => { if (timer) window.clearInterval(timer) })

function formatMilliseconds(value: number) {
  if (value < 1_000) return `${Math.round(value)} ms`
  return `${(value / 1_000).toFixed(1)} s`
}
</script>

<template>
  <div class="local-ai-status">
    <div class="local-ai-status__heading">
      <span class="local-ai-status__icon"><Sparkles :size="15" /></span>
      <div><strong>{{ title }}</strong><span>{{ detail }}</span></div>
      <Button
        v-if="indexing || paused"
        class="local-ai-status__control"
        variant="ghost"
        size="icon"
        :aria-label="paused ? 'Reprendre l’indexation' : 'Mettre l’indexation en pause'"
        :title="paused ? 'Reprendre' : 'Pause'"
        @click="emit(paused ? 'resume' : 'pause')"
      >
        <Play v-if="paused" :size="15" />
        <Pause v-else :size="15" />
      </Button>
    </div>
    <ProgressBar class="local-ai-status__progress" :value="percent" :indeterminate="indeterminate" size="sm" label="Progression de l’IA locale" />
    <Popover align="start" width="320px">
      <template #trigger="{ open }">
        <DisclosureButton :open="open" label="Voir les performances" @toggle="popoverOpen = !open" />
      </template>
      <template #content>
        <div class="ai-stats">
          <div class="ai-stats__title">
            <div><strong>{{ liveStats?.modelName ?? 'MobileCLIP-S0' }}</strong><span>{{ phaseLabel }}</span></div>
            <Badge :variant="liveStats?.accelerationActive ? 'success' : 'neutral'">
              {{ liveStats?.accelerationLabel ?? 'Chargement' }}
            </Badge>
          </div>
          <dl>
            <div><dt>Backend actif</dt><dd>{{ liveStats?.backendEffective ?? 'Chargement…' }}</dd></div>
            <div><dt>Backend demandé</dt><dd>{{ liveStats?.backendRequested ?? 'WebGPU' }}</dd></div>
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
            <div><Cpu :size="15" /><span>Imagyx {{ liveStats?.processCpuPercent.toFixed(0) ?? 0 }} % CPU</span></div>
            <div><Gauge :size="15" /><span>Système {{ liveStats?.systemCpuPercent.toFixed(0) ?? 0 }} % CPU</span></div>
            <div><MemoryStick :size="15" /><span>{{ formatBytes(liveStats?.processMemoryBytes ?? 0) }} RAM</span></div>
          </div>
          <p v-if="liveStats?.fallbackReason" class="ai-stats__warning">{{ liveStats.fallbackReason }}</p>
        </div>
      </template>
    </Popover>
  </div>
</template>

<style scoped>
.local-ai-status { display:grid; gap:var(--space-3); margin-top:auto; padding:var(--space-3); border:1px solid var(--border); border-radius:var(--radius-lg); background:var(--surface); }
.local-ai-status__heading { display:flex; align-items:center; gap:var(--space-3); min-width:0; }
.local-ai-status__icon { display:grid; place-items:center; width:30px; height:30px; border-radius:var(--radius-md); background:var(--primary-soft); color:var(--primary-text); }
.local-ai-status__heading div { min-width:0; flex:1; }
.local-ai-status__heading strong,.local-ai-status__heading span { display:block; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.local-ai-status__heading strong { font-size:var(--text-xs); }
.local-ai-status__heading span { margin-top:3px; color:var(--text-muted); font-size:10px; }
.local-ai-status__control { flex:0 0 auto; }
.local-ai-status__progress { width:100%; }
.ai-stats { display:grid; gap:var(--space-4); }
.ai-stats__title { display:flex; align-items:center; justify-content:space-between; gap:var(--space-3); }
.ai-stats__title strong,.ai-stats__title span { display:block; }
.ai-stats__title strong { font-size:var(--text-sm); }
.ai-stats__title span { margin-top:3px; color:var(--text-muted); font-size:var(--text-xs); }
.ai-stats dl { display:grid; gap:var(--space-2); margin:0; }
.ai-stats dl div { display:flex; justify-content:space-between; gap:var(--space-4); font-size:var(--text-xs); }
.ai-stats dt { color:var(--text-muted); }
.ai-stats dd { margin:0; color:var(--text); font-variant-numeric:tabular-nums; text-align:right; }
.ai-stats__timings { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:var(--space-2); }
.ai-stats__timings span { display:grid; gap:3px; padding:var(--space-2); border:1px solid var(--border); border-radius:var(--radius-md); color:var(--text-muted); font-size:10px; }
.ai-stats__timings strong { color:var(--text); font-size:var(--text-xs); font-variant-numeric:tabular-nums; }
.ai-stats__resources { display:grid; gap:var(--space-2); padding-top:var(--space-3); border-top:1px solid var(--border); }
.ai-stats__resources div { display:flex; align-items:center; gap:var(--space-2); color:var(--text-muted); font-size:var(--text-xs); }
.ai-stats__warning { margin:0; color:var(--warning-text,var(--text-muted)); font-size:var(--text-xs); line-height:1.45; }
</style>
