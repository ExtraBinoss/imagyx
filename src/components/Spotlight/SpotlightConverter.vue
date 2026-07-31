<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import {
  ArrowLeft,
  ArrowRight,
  LoaderCircle,
  RefreshCw,
} from '@lucide/vue'
import { imagyxApi } from '../../api/tauri'
import { useTranslate } from '../../i18n'
import type {
  ImageAsset,
  ImageConversionFormat,
  ImageConversionProgress,
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
const FORMAT_SHORTCUTS: Record<ImageConversionFormat, string> = {
  avif: 'A',
  webp: 'W',
  png: 'P',
  jpg: 'J',
  ico: 'I',
}

const { t } = useTranslate()
const status = ref<'choose' | 'converting' | 'error'>('choose')
const selectedFormatIndex = ref(0)
const targetFormat = ref<ImageConversionFormat | null>(null)
const progress = ref(0)
const progressStage = ref<ImageConversionProgress['stage']>('decoding')
const error = ref<string | null>(null)
let unlistenProgress: UnlistenFn | null = null

const sourceFormat = computed(() => normalizeFormat(props.source.extension))
const availableFormats = computed(() => FORMATS.filter((format) => format !== sourceFormat.value))
const selectedFormat = computed(() => (
  availableFormats.value[selectedFormatIndex.value] ?? availableFormats.value[0]
))
const sourceFormatLabel = computed(() => (
  sourceFormat.value?.toLocaleUpperCase() ?? props.source.extension.toLocaleUpperCase()
))
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

function selectFormat(index: number) {
  if (status.value === 'converting') return
  const count = availableFormats.value.length
  if (!count) return
  selectedFormatIndex.value = (index + count) % count
  if (status.value === 'error') {
    status.value = 'choose'
    error.value = null
  }
}

function formatIndex(format: ImageConversionFormat): number {
  return availableFormats.value.findIndex((candidate) => candidate === format)
}

async function startConversion(format = selectedFormat.value) {
  if (!format || status.value === 'converting') return
  const index = formatIndex(format)
  if (index >= 0) selectedFormatIndex.value = index
  status.value = 'converting'
  targetFormat.value = format
  progress.value = 0.04
  progressStage.value = 'decoding'
  error.value = null

  try {
    const converted = await imagyxApi.convertImage(props.source.id, format)
    converted.image.semanticScore = props.source.semanticScore
    converted.image.relevanceScore = props.source.relevanceScore
    progress.value = 1
    progressStage.value = 'complete'
    emit('converted', converted.image)
    emit('back')
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
  error.value = null
}

function handleKeydown(event: KeyboardEvent): boolean {
  if (event.key === 'Escape') {
    if (status.value !== 'converting') emit('back')
    return true
  }

  if (status.value === 'converting') return true

  const key = event.key.toLocaleLowerCase()
  if ((event.ctrlKey || event.metaKey) && event.shiftKey && (key === 'c' || event.code === 'KeyC')) {
    void startConversion()
    return true
  }

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

  if (!event.ctrlKey && !event.metaKey && !event.altKey) {
    const shortcutFormat = availableFormats.value.find((format) => (
      FORMAT_SHORTCUTS[format].toLocaleLowerCase('en') === key
    ))
    if (shortcutFormat) {
      void startConversion(shortcutFormat)
      return true
    }
  }

  return true
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
  <Teleport to=".spotlight-input">
    <div class="spotlight-conversion-input" aria-live="polite">
      <Button
        class="spotlight-conversion-input__back"
        variant="ghost"
        size="icon"
        :disabled="status === 'converting'"
        :aria-label="t('spotlight.convert.back')"
        @click="emit('back')"
      >
        <ArrowLeft :size="18" />
      </Button>

      <span class="spotlight-conversion-input__thumb">
        <ThumbnailImage class="spotlight-conversion-input__image" :image="source" />
      </span>

      <span class="spotlight-conversion-input__copy">
        <strong>{{ source.name }}</strong>
        <small>
          <span>{{ sourceFormatLabel }}</span>
          <ArrowRight :size="13" />
          <b>{{ selectedFormat?.toLocaleUpperCase() }}</b>
        </small>
      </span>

      <KbdChip shortcut="Esc" size="sm" />
    </div>
  </Teleport>

  <section class="spotlight-converter" :aria-label="t('spotlight.convert.title')">
    <button
      class="spotlight-converter__command"
      :class="{ 'spotlight-converter__command--busy': status === 'converting' }"
      type="button"
      :disabled="status === 'converting'"
      @click="startConversion()"
    >
      <span class="spotlight-converter__command-icon">
        <LoaderCircle v-if="status === 'converting'" class="spin" :size="19" />
        <RefreshCw v-else :size="18" />
      </span>

      <span class="spotlight-converter__command-copy">
        <strong>{{ t('spotlight.convert.title') }}</strong>
        <small>{{ status === 'converting' ? progressLabel : t('spotlight.convert.magic') }}</small>
      </span>

      <b>{{ selectedFormat?.toLocaleUpperCase() }}</b>
      <KbdChip shortcut="Enter" size="sm" />

      <span
        v-if="status === 'converting'"
        class="spotlight-converter__progress"
        role="progressbar"
        :aria-label="progressLabel"
        :aria-valuenow="progressPercent"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <span :style="{ width: `${progressPercent}%` }" />
      </span>
    </button>

    <div v-if="error" class="spotlight-converter__error" role="alert">
      <span>
        <strong>{{ t('spotlight.convert.error') }}</strong>
        <small>{{ error }}</small>
      </span>
      <Button variant="secondary" size="sm" @click="startConversion(targetFormat ?? selectedFormat)">
        <template #leading><RefreshCw :size="14" /></template>
        {{ t('spotlight.convert.retry') }}
      </Button>
    </div>

    <header class="spotlight-converter__section-heading">
      <span>
        <strong>{{ t('spotlight.convert.choose_format') }}</strong>
        <small>{{ t('spotlight.convert.format_hint') }}</small>
      </span>
      <KbdChip shortcut="ArrowUp+ArrowDown" size="sm" />
    </header>

    <div class="spotlight-converter__formats" role="listbox">
      <button
        v-for="(format, index) in availableFormats"
        :id="`spotlight-convert-${format}`"
        :key="format"
        class="spotlight-converter__format"
        :class="{ 'spotlight-converter__format--selected': index === selectedFormatIndex }"
        :aria-selected="index === selectedFormatIndex"
        role="option"
        type="button"
        @mouseenter="selectFormat(index)"
        @focus="selectFormat(index)"
        @click="selectFormat(index)"
        @dblclick="startConversion(format)"
      >
        <span class="spotlight-converter__format-badge">{{ format.toLocaleUpperCase() }}</span>
        <span class="spotlight-converter__format-copy">
          <strong>{{ formatDescription(format) }}</strong>
          <small>{{ sourceFormatLabel }} → {{ format.toLocaleUpperCase() }}</small>
        </span>
        <KbdChip :shortcut="FORMAT_SHORTCUTS[format]" size="sm" />
        <ArrowRight :size="15" />
      </button>
    </div>
  </section>
</template>

<style scoped>
.spotlight-conversion-input {
  position: absolute;
  inset: 0;
  z-index: 7;
  display: grid;
  grid-template-columns: 32px 42px minmax(0, 1fr) auto;
  align-items: center;
  gap: 11px;
  min-height: 70px;
  padding: 0 15px 0 13px;
  background: color-mix(in srgb, var(--surface-elevated) 97%, transparent);
  animation: conversion-input-enter 220ms cubic-bezier(0.16, 1, 0.3, 1) both;
}

.spotlight-conversion-input__back { margin-left: -2px; }

.spotlight-conversion-input__thumb {
  width: 42px;
  height: 42px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-hover);
  box-shadow: 0 7px 18px rgb(2 6 23 / 0.14);
}

.spotlight-conversion-input__image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.spotlight-conversion-input__copy { min-width: 0; }

.spotlight-conversion-input__copy strong,
.spotlight-conversion-input__copy small {
  display: flex;
  align-items: center;
  min-width: 0;
}

.spotlight-conversion-input__copy strong {
  overflow: hidden;
  color: var(--text);
  font-size: 15px;
  letter-spacing: -0.2px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.spotlight-conversion-input__copy small {
  gap: 6px;
  margin-top: 5px;
  color: var(--text-muted);
  font-size: 9px;
  font-weight: 720;
  letter-spacing: 0.04em;
}

.spotlight-conversion-input__copy small span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.spotlight-conversion-input__copy small svg {
  flex: 0 0 auto;
  color: var(--text-subtle);
}

.spotlight-conversion-input__copy small b { color: var(--primary-text); }

.spotlight-converter {
  display: grid;
  align-content: start;
  gap: 10px;
  height: 100%;
  padding: 10px 9px 20px;
  overflow-y: auto;
  animation: converter-enter 220ms cubic-bezier(0.16, 1, 0.3, 1) both;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--border-strong) 78%, transparent) transparent;
}

.spotlight-converter__command,
.spotlight-converter__format {
  width: 100%;
  border: 1px solid transparent;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: default;
}

.spotlight-converter__command {
  position: relative;
  display: grid;
  grid-template-columns: 42px minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 11px;
  min-height: 72px;
  padding: 10px 12px;
  overflow: hidden;
  border-color: color-mix(in srgb, var(--primary) 34%, var(--border));
  border-radius: 14px;
  background: color-mix(in srgb, var(--primary-soft) 72%, var(--surface));
  box-shadow: inset 0 1px rgb(255 255 255 / 0.06);
  transition: transform 170ms cubic-bezier(0.16, 1, 0.3, 1), border-color 150ms ease, background-color 150ms ease;
}

.spotlight-converter__command:not(:disabled):hover {
  transform: translate3d(2px, 0, 0);
  border-color: color-mix(in srgb, var(--primary) 52%, var(--border));
}

.spotlight-converter__command--busy { cursor: wait; }

.spotlight-converter__command-icon {
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  border-radius: 11px;
  background: var(--primary);
  color: var(--primary-foreground);
}

.spotlight-converter__command-copy { min-width: 0; }

.spotlight-converter__command-copy strong,
.spotlight-converter__command-copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.spotlight-converter__command-copy strong {
  color: var(--text);
  font-size: 13px;
  letter-spacing: -0.12px;
}

.spotlight-converter__command-copy small {
  margin-top: 5px;
  color: var(--text-muted);
  font-size: 10px;
}

.spotlight-converter__command > b {
  padding: 5px 8px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--surface-elevated) 84%, transparent);
  color: var(--primary-text);
  font-size: 9px;
  letter-spacing: 0.05em;
}

.spotlight-converter__progress {
  position: absolute;
  right: 12px;
  bottom: 7px;
  left: 65px;
  height: 4px;
  overflow: hidden;
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--surface-hover) 84%, transparent);
}

.spotlight-converter__progress span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--primary);
  transition: width 160ms cubic-bezier(0.16, 1, 0.3, 1);
}

.spotlight-converter__error {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--danger-border);
  border-radius: 12px;
  background: var(--danger-surface);
}

.spotlight-converter__error strong,
.spotlight-converter__error small { display: block; }

.spotlight-converter__error strong {
  color: var(--danger-text);
  font-size: 10px;
}

.spotlight-converter__error small {
  margin-top: 4px;
  color: var(--danger-text);
  font-size: 9px;
}

.spotlight-converter__section-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 5px 5px 2px;
}

.spotlight-converter__section-heading strong,
.spotlight-converter__section-heading small { display: block; }

.spotlight-converter__section-heading strong {
  color: var(--text);
  font-size: 11px;
}

.spotlight-converter__section-heading small {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 9px;
}

.spotlight-converter__formats { display: grid; }

.spotlight-converter__format {
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr) auto 18px;
  align-items: center;
  gap: 12px;
  min-height: 64px;
  padding: 7px 11px;
  border-radius: 14px;
  transition: background-color 150ms ease, border-color 150ms ease, transform 180ms cubic-bezier(0.16, 1, 0.3, 1), box-shadow 180ms ease;
}

.spotlight-converter__format:hover,
.spotlight-converter__format--selected {
  border-color: color-mix(in srgb, var(--primary) 28%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 70%, var(--surface));
  box-shadow: inset 0 1px rgb(255 255 255 / 0.05);
  transform: translate3d(2px, 0, 0) scale(0.998);
}

.spotlight-converter__format-badge {
  display: grid;
  place-items: center;
  width: 52px;
  height: 44px;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-hover);
  color: var(--primary-text);
  font-size: 10px;
  font-weight: 780;
  letter-spacing: 0.04em;
  box-shadow: 0 6px 16px rgb(2 6 23 / 0.1);
}

.spotlight-converter__format-copy { min-width: 0; }

.spotlight-converter__format-copy strong,
.spotlight-converter__format-copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.spotlight-converter__format-copy strong {
  color: var(--text);
  font-size: 12px;
}

.spotlight-converter__format-copy small {
  margin-top: 5px;
  color: var(--text-muted);
  font-size: 9px;
  font-weight: 650;
}

.spotlight-converter__format > svg { color: var(--text-subtle); }
.spin { animation: spin 0.8s linear infinite; }

@keyframes spin { to { transform: rotate(1turn); } }

@keyframes converter-enter {
  from { opacity: 0; transform: translate3d(12px, 0, 0); filter: blur(3px); }
  to { opacity: 1; transform: none; filter: none; }
}

@keyframes conversion-input-enter {
  from { opacity: 0; transform: translate3d(10px, 0, 0); filter: blur(3px); }
  to { opacity: 1; transform: none; filter: none; }
}

@media (prefers-reduced-motion: reduce) {
  .spotlight-conversion-input,
  .spotlight-converter,
  .spin { animation-duration: 0.01ms; }
}
</style>
