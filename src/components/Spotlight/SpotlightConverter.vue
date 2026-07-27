<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Copy,
  ExternalLink,
  FolderOpen,
  LoaderCircle,
  RefreshCw,
  Sparkles,
} from '@lucide/vue'
import { imagyxApi } from '../../api/tauri'
import { useTranslate } from '../../i18n'
import type {
  ImageAsset,
  ImageConversionFormat,
  ImageConversionProgress,
  ImageConversionResult,
} from '../../types'
import ThumbnailImage from '../ThumbnailImage.vue'
import Button from '../ui/Button/Button.vue'
import KbdChip from '../ui/KbdChip/KbdChip.vue'

const props = defineProps<{
  source: ImageAsset
  copiedImageId: string | null
  copyingImageId: string | null
  revealingImageId: string | null
  openingImageId: string | null
  fileManagerName: string
}>()

const emit = defineEmits<{
  back: []
  converted: [image: ImageAsset]
  copy: [image: ImageAsset]
  reveal: [image: ImageAsset]
  open: [image: ImageAsset]
}>()

const FORMATS: ImageConversionFormat[] = ['avif', 'webp', 'png', 'jpg', 'ico']
const { t } = useTranslate()
const status = ref<'choose' | 'converting' | 'ready' | 'error'>('choose')
const selectedFormatIndex = ref(0)
const targetFormat = ref<ImageConversionFormat | null>(null)
const progress = ref(0)
const progressStage = ref<ImageConversionProgress['stage']>('decoding')
const result = ref<ImageConversionResult | null>(null)
const error = ref<string | null>(null)
let unlistenProgress: UnlistenFn | null = null

const sourceFormat = computed(() => normalizeFormat(props.source.extension))
const availableFormats = computed(() => FORMATS.filter((format) => format !== sourceFormat.value))
const selectedFormat = computed(() => availableFormats.value[selectedFormatIndex.value] ?? availableFormats.value[0])
const convertedImage = computed(() => result.value?.image ?? null)
const progressPercent = computed(() => Math.round(Math.max(0, Math.min(1, progress.value)) * 100))
const progressLabel = computed(() => t(`spotlight.convert.progress.${progressStage.value}`))

watch(
  () => props.source.id,
  () => reset(),
)

watch(availableFormats, (formats) => {
  if (selectedFormatIndex.value >= formats.length) selectedFormatIndex.value = 0
})

function normalizeFormat(extension: string): ImageConversionFormat | null {
  const format = extension.toLocaleLowerCase('en') === 'jpeg'
    ? 'jpg'
    : extension.toLocaleLowerCase('en')
  return FORMATS.includes(format as ImageConversionFormat)
    ? format as ImageConversionFormat
    : null
}

function formatDescription(format: ImageConversionFormat): string {
  return t(`spotlight.convert.format.${format}`)
}

function formatSize(image: ImageAsset): string {
  return `${image.width} × ${image.height} · ${Math.max(1, Math.round(image.sizeBytes / 1024))} KB`
}

function selectFormat(index: number) {
  const count = availableFormats.value.length
  if (!count) return
  selectedFormatIndex.value = (index + count) % count
}

async function startConversion(format = selectedFormat.value) {
  if (!format || status.value === 'converting') return
  status.value = 'converting'
  targetFormat.value = format
  progress.value = 0.04
  progressStage.value = 'decoding'
  result.value = null
  error.value = null
  try {
    const converted = await imagyxApi.convertImage(props.source.id, format)
    converted.image.semanticScore = props.source.semanticScore
    result.value = converted
    progress.value = 1
    progressStage.value = 'complete'
    status.value = 'ready'
    emit('converted', converted.image)
  } catch (reason) {
    error.value = String(reason)
    status.value = 'error'
  }
}

function reset() {
  status.value = 'choose'
  selectedFormatIndex.value = 0
  targetFormat.value = null
  progress.value = 0
  progressStage.value = 'decoding'
  result.value = null
  error.value = null
}

function handleKeydown(event: KeyboardEvent): boolean {
  if (event.key === 'Escape') {
    if (status.value === 'converting') return true
    emit('back')
    return true
  }

  const key = event.key.toLocaleLowerCase()
  if ((event.ctrlKey || event.metaKey) && event.shiftKey && (key === 'c' || event.code === 'KeyC')) {
    if (status.value === 'ready') reset()
    else if (status.value === 'choose') void startConversion()
    return true
  }

  if ((event.ctrlKey || event.metaKey) && status.value === 'ready' && convertedImage.value) {
    if (key === 'c' || event.code === 'KeyC') {
      emit('copy', convertedImage.value)
      return true
    }
    if (key === 'e' || event.code === 'KeyE') {
      emit('reveal', convertedImage.value)
      return true
    }
    if (key === 'i' || event.code === 'KeyI') {
      emit('open', convertedImage.value)
      return true
    }
  }

  if (status.value === 'choose') {
    if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
      selectFormat(selectedFormatIndex.value + 1)
      return true
    }
    if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
      selectFormat(selectedFormatIndex.value - 1)
      return true
    }
    if (event.key === 'Tab') {
      selectFormat(selectedFormatIndex.value + (event.shiftKey ? -1 : 1))
      return true
    }
    if (event.key === 'Enter') {
      void startConversion()
      return true
    }
  }

  if (status.value === 'error' && event.key === 'Enter') {
    void startConversion(targetFormat.value ?? selectedFormat.value)
    return true
  }
  return status.value === 'converting'
}

defineExpose({ handleKeydown })

onMounted(async () => {
  unlistenProgress = await listen<ImageConversionProgress>('image-conversion-progress', ({ payload }) => {
    if (payload.sourceImageId !== props.source.id || payload.targetFormat !== targetFormat.value) return
    progress.value = Math.max(progress.value, payload.progress)
    progressStage.value = payload.stage
  })
})

onBeforeUnmount(() => {
  unlistenProgress?.()
})
</script>

<template>
  <section class="spotlight-converter" :aria-label="t('spotlight.convert.title')">
    <header class="spotlight-converter__header">
      <Button
        variant="ghost"
        size="icon"
        :disabled="status === 'converting'"
        :aria-label="t('spotlight.convert.back')"
        @click="emit('back')"
      >
        <template #leading><ArrowLeft :size="17" /></template>
      </Button>
      <span class="spotlight-converter__heading">
        <strong>{{ t('spotlight.convert.title') }}</strong>
        <small>{{ t('spotlight.convert.description') }}</small>
      </span>
      <KbdChip shortcut="Ctrl+Shift+C" size="sm" />
    </header>

    <div class="spotlight-converter__flow">
      <article class="spotlight-converter__image-card">
        <span class="spotlight-converter__preview">
          <ThumbnailImage class="spotlight-converter__thumbnail" :image="source" />
        </span>
        <span class="spotlight-converter__image-copy">
          <small>{{ t('spotlight.convert.source') }}</small>
          <strong>{{ source.name }}</strong>
          <span>{{ formatSize(source) }}</span>
        </span>
        <b>{{ source.extension.toLocaleUpperCase() }}</b>
      </article>

      <span class="spotlight-converter__arrow" aria-hidden="true">
        <ArrowRight :size="18" />
      </span>

      <article
        class="spotlight-converter__image-card spotlight-converter__image-card--target"
        :class="{ 'spotlight-converter__image-card--ready': convertedImage }"
      >
        <span class="spotlight-converter__preview">
          <ThumbnailImage
            v-if="convertedImage"
            class="spotlight-converter__thumbnail"
            :image="convertedImage"
          />
          <LoaderCircle v-else-if="status === 'converting'" class="spin" :size="22" />
          <Sparkles v-else :size="22" />
        </span>
        <span class="spotlight-converter__image-copy">
          <small>{{ convertedImage ? t('spotlight.convert.complete') : t('spotlight.convert.target') }}</small>
          <strong>{{ convertedImage?.name ?? targetFormat?.toLocaleUpperCase() ?? '—' }}</strong>
          <span>{{ convertedImage ? formatSize(convertedImage) : t('spotlight.convert.magic') }}</span>
        </span>
        <b>{{ convertedImage?.extension.toLocaleUpperCase() ?? targetFormat?.toLocaleUpperCase() ?? '—' }}</b>
      </article>
    </div>

    <div v-if="status === 'choose'" class="spotlight-converter__chooser">
      <div class="spotlight-converter__section-title">
        <span>
          <strong>{{ t('spotlight.convert.choose_format') }}</strong>
          <small>{{ t('spotlight.convert.format_hint') }}</small>
        </span>
        <KbdChip shortcut="↕ Enter" size="sm" />
      </div>
      <div class="spotlight-converter__formats" role="listbox">
        <button
          v-for="(format, index) in availableFormats"
          :key="format"
          class="spotlight-converter__format"
          :class="{ 'spotlight-converter__format--selected': index === selectedFormatIndex }"
          :aria-selected="index === selectedFormatIndex"
          role="option"
          type="button"
          @mouseenter="selectFormat(index)"
          @focus="selectFormat(index)"
          @click="startConversion(format)"
        >
          <b>{{ format.toLocaleUpperCase() }}</b>
          <span>{{ formatDescription(format) }}</span>
          <ArrowRight :size="15" />
        </button>
      </div>
    </div>

    <div v-else-if="status === 'converting'" class="spotlight-converter__progress-card" aria-live="polite">
      <span class="spotlight-converter__progress-heading">
        <span>
          <strong>{{ progressLabel }}</strong>
          <small>{{ t('spotlight.convert.progress_note') }}</small>
        </span>
        <b>{{ progressPercent }}%</b>
      </span>
      <div
        class="spotlight-converter__progress"
        role="progressbar"
        :aria-label="progressLabel"
        :aria-valuenow="progressPercent"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <span :style="{ width: `${progressPercent}%` }" />
      </div>
    </div>

    <div v-else-if="status === 'ready' && convertedImage" class="spotlight-converter__ready" aria-live="polite">
      <span class="spotlight-converter__ready-icon"><Check :size="18" /></span>
      <span class="spotlight-converter__ready-copy">
        <strong>{{ t('spotlight.convert.complete') }}</strong>
        <small>{{ t('spotlight.convert.complete_desc') }}</small>
        <em>
          <Sparkles :size="13" />
          {{ result?.inheritedSemanticIndex ? t('spotlight.convert.inherited_ai') : t('spotlight.convert.no_ai') }}
        </em>
      </span>
      <span class="spotlight-converter__actions">
        <Button
          class="spotlight-converter__action"
          :class="{ 'spotlight-converter__action--success': copiedImageId === convertedImage.id }"
          :variant="copiedImageId === convertedImage.id ? 'primary' : 'secondary'"
          size="sm"
          :loading="copyingImageId === convertedImage.id"
          :aria-label="t('copy_image')"
          @click="emit('copy', convertedImage)"
        >
          <template #leading>
            <Check v-if="copiedImageId === convertedImage.id" :size="15" />
            <Copy v-else :size="15" />
          </template>
          <template #trailing><KbdChip shortcut="Ctrl+C" size="sm" /></template>
        </Button>
        <Button
          class="spotlight-converter__action"
          variant="secondary"
          size="sm"
          :loading="revealingImageId === convertedImage.id"
          :aria-label="t('open_in_file_manager', { name: fileManagerName })"
          @click="emit('reveal', convertedImage)"
        >
          <template #leading><FolderOpen :size="15" /></template>
          <template #trailing><KbdChip shortcut="Ctrl+E" size="sm" /></template>
        </Button>
        <Button
          class="spotlight-converter__action"
          variant="primary"
          size="sm"
          :loading="openingImageId === convertedImage.id"
          :aria-label="t('open_in_imagyx')"
          @click="emit('open', convertedImage)"
        >
          <template #leading><ExternalLink :size="15" /></template>
          <template #trailing><KbdChip shortcut="Ctrl+I" size="sm" /></template>
        </Button>
        <Button variant="ghost" size="icon" :aria-label="t('spotlight.convert.again')" @click="reset">
          <template #leading><RefreshCw :size="15" /></template>
        </Button>
      </span>
    </div>

    <div v-else class="spotlight-converter__error" role="alert">
      <strong>{{ t('spotlight.convert.error') }}</strong>
      <span>{{ error }}</span>
      <Button variant="secondary" size="sm" @click="startConversion(targetFormat ?? selectedFormat)">
        <template #leading><RefreshCw :size="14" /></template>
        {{ t('spotlight.convert.retry') }}
      </Button>
    </div>
  </section>
</template>

<style scoped>
.spotlight-converter { display: grid; gap: 14px; height: 100%; padding: 10px 12px 16px; overflow-y: auto; }
.spotlight-converter__header { display: grid; grid-template-columns: 32px minmax(0, 1fr) auto; align-items: center; gap: 10px; }
.spotlight-converter__heading { min-width: 0; }
.spotlight-converter__heading strong,
.spotlight-converter__heading small { display: block; }
.spotlight-converter__heading strong { color: var(--text); font-size: 13px; letter-spacing: -0.12px; }
.spotlight-converter__heading small { margin-top: 4px; color: var(--text-muted); font-size: 10px; }
.spotlight-converter__flow { display: grid; grid-template-columns: minmax(0, 1fr) 28px minmax(0, 1fr); align-items: center; gap: 7px; }
.spotlight-converter__arrow { display: grid; place-items: center; color: var(--text-subtle); }
.spotlight-converter__image-card { display: grid; grid-template-columns: 52px minmax(0, 1fr) auto; align-items: center; gap: 10px; min-width: 0; padding: 10px; border: 1px solid var(--border); border-radius: 14px; background: color-mix(in srgb, var(--surface) 88%, transparent); }
.spotlight-converter__image-card--target { border-style: dashed; background: color-mix(in srgb, var(--primary-soft) 32%, var(--surface)); }
.spotlight-converter__image-card--ready { border-style: solid; border-color: color-mix(in srgb, var(--primary) 32%, var(--border)); }
.spotlight-converter__preview { display: grid; place-items: center; width: 52px; height: 52px; overflow: hidden; border: 1px solid var(--border); border-radius: 11px; background: var(--surface-hover); color: var(--primary-text); }
.spotlight-converter__thumbnail { width: 100%; height: 100%; object-fit: cover; }
.spotlight-converter__image-copy { min-width: 0; }
.spotlight-converter__image-copy small,
.spotlight-converter__image-copy strong,
.spotlight-converter__image-copy span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.spotlight-converter__image-copy small { color: var(--text-subtle); font-size: 8px; font-weight: 760; letter-spacing: 0.08em; text-transform: uppercase; }
.spotlight-converter__image-copy strong { margin-top: 4px; color: var(--text); font-size: 11px; }
.spotlight-converter__image-copy span { margin-top: 4px; color: var(--text-muted); font-size: 9px; }
.spotlight-converter__image-card > b { align-self: start; padding: 4px 6px; border-radius: 7px; background: var(--primary-soft); color: var(--primary-text); font-size: 8px; letter-spacing: 0.04em; }
.spotlight-converter__chooser { display: grid; gap: 10px; min-height: 0; }
.spotlight-converter__section-title { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 0 2px; }
.spotlight-converter__section-title strong,
.spotlight-converter__section-title small { display: block; }
.spotlight-converter__section-title strong { color: var(--text); font-size: 11px; }
.spotlight-converter__section-title small { margin-top: 3px; color: var(--text-muted); font-size: 9px; }
.spotlight-converter__formats { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
.spotlight-converter__format { display: grid; grid-template-columns: 48px minmax(0, 1fr) 18px; align-items: center; gap: 10px; min-height: 58px; padding: 8px 11px; border: 1px solid var(--border); border-radius: 12px; background: var(--surface); color: inherit; text-align: left; cursor: pointer; transition: transform 150ms ease, border-color 150ms ease, background-color 150ms ease, box-shadow 150ms ease; }
.spotlight-converter__format:hover,
.spotlight-converter__format--selected { border-color: color-mix(in srgb, var(--primary) 36%, var(--border)); background: color-mix(in srgb, var(--primary-soft) 62%, var(--surface)); box-shadow: inset 0 1px rgb(255 255 255 / 0.05); transform: translate3d(2px, 0, 0); }
.spotlight-converter__format b { color: var(--primary-text); font-size: 11px; letter-spacing: 0.04em; }
.spotlight-converter__format span { overflow: hidden; color: var(--text-secondary); font-size: 9px; text-overflow: ellipsis; white-space: nowrap; }
.spotlight-converter__format svg { color: var(--text-subtle); }
.spotlight-converter__progress-card,
.spotlight-converter__ready,
.spotlight-converter__error { margin-top: 4px; padding: 16px; border: 1px solid color-mix(in srgb, var(--primary) 22%, var(--border)); border-radius: 14px; background: color-mix(in srgb, var(--primary-soft) 34%, var(--surface)); }
.spotlight-converter__progress-heading { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
.spotlight-converter__progress-heading strong,
.spotlight-converter__progress-heading small { display: block; }
.spotlight-converter__progress-heading strong { color: var(--text); font-size: 11px; }
.spotlight-converter__progress-heading small { margin-top: 4px; color: var(--text-muted); font-size: 9px; }
.spotlight-converter__progress-heading b { color: var(--primary-text); font-size: 12px; font-variant-numeric: tabular-nums; }
.spotlight-converter__progress { height: 7px; margin-top: 13px; overflow: hidden; border-radius: var(--radius-full); background: color-mix(in srgb, var(--surface-hover) 82%, transparent); }
.spotlight-converter__progress span { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--primary), color-mix(in srgb, var(--primary) 62%, white)); transition: width 180ms cubic-bezier(0.16, 1, 0.3, 1); }
.spotlight-converter__ready { display: grid; grid-template-columns: 38px minmax(0, 1fr) auto; align-items: center; gap: 12px; }
.spotlight-converter__ready-icon { display: grid; place-items: center; width: 38px; height: 38px; border-radius: 12px; background: var(--primary); color: var(--primary-foreground); }
.spotlight-converter__ready-copy { min-width: 0; }
.spotlight-converter__ready-copy strong,
.spotlight-converter__ready-copy small { display: block; }
.spotlight-converter__ready-copy strong { color: var(--text); font-size: 11px; }
.spotlight-converter__ready-copy small { margin-top: 4px; color: var(--text-muted); font-size: 9px; }
.spotlight-converter__ready-copy em { display: inline-flex; align-items: center; gap: 5px; margin-top: 7px; color: var(--primary-text); font-size: 9px; font-style: normal; font-weight: 680; }
.spotlight-converter__actions { display: flex; align-items: center; gap: 5px; }
.spotlight-converter__action { min-width: 62px; padding-inline: 8px; }
.spotlight-converter__action--success { animation: action-success 280ms cubic-bezier(0.16, 1, 0.3, 1) both; }
.spotlight-converter__error { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 7px 12px; border-color: var(--danger-border); background: var(--danger-surface); }
.spotlight-converter__error strong { color: var(--danger-text); font-size: 11px; }
.spotlight-converter__error span { grid-column: 1; color: var(--danger-text); font-size: 9px; }
.spotlight-converter__error button { grid-column: 2; grid-row: 1 / span 2; }
.spin { animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(1turn); } }
@keyframes action-success { 0% { transform: scale(0.94); } 55% { transform: scale(1.04); } 100% { transform: none; } }
@media (prefers-reduced-motion: reduce) {
  .spin,
  .spotlight-converter__action--success { animation-duration: 0.01ms; }
}
</style>
