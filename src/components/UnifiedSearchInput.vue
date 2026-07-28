<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import {
  ArrowLeft,
  ImageUp,
  LoaderCircle,
  Search,
  Settings2,
  Sparkles,
  X,
} from '@lucide/vue'
import type { FollowedFolder } from '../types'
import { useTranslate } from '../i18n'
import { visualSearch } from '../services/visual-search'
import { visualSearchSession } from '../services/visual-search-session'
import Button from './ui/Button/Button.vue'
import KbdChip from './ui/KbdChip/KbdChip.vue'
import FolderQueryAutocomplete from './FolderQueryAutocomplete.vue'
import ThumbnailImage from './ThumbnailImage.vue'

const props = withDefaults(defineProps<{
  variant: 'app' | 'spotlight'
  modelValue: string
  placeholder: string
  searching?: boolean
  resultLabel?: string
  view?: 'search' | 'settings'
  showSettings?: boolean
  folderSuggestions?: FollowedFolder[]
  folderSuggestionIndex?: number
}>(), {
  searching: false,
  resultLabel: '',
  view: 'search',
  showSettings: false,
  folderSuggestions: () => [],
  folderSuggestionIndex: 0,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  settings: []
  back: []
  folderSelect: [folder: FollowedFolder]
  folderNavigate: [delta: number]
  visualError: [reason: unknown]
}>()

const { t } = useTranslate()
const input = ref<HTMLInputElement | null>(null)
const focused = ref(false)
const dragActive = ref(false)
const previousQuery = ref('')
let visualWasActive = false
let unlistenWindowDrop: (() => void) | null = null

const visualState = visualSearchSession.state
const visualActive = computed(() => visualState.status !== 'idle')
const visualReady = computed(() => visualState.status === 'ready' && Boolean(visualState.token))
const showTextCount = computed(() => (
  props.view === 'search'
  && !visualActive.value
  && Boolean(props.modelValue.trim())
  && Boolean(props.resultLabel)
))
const showVisualCount = computed(() => visualReady.value && Boolean(props.resultLabel))
const ariaLabel = computed(() => props.view === 'settings'
  ? t('spotlight.search_in_settings')
  : t('search.aria'))
const visualTitle = computed(() => {
  if (visualState.status === 'loading') return t('search.visual.preparing')
  if (visualState.status === 'error') return t('search.visual.error')
  return t('search.visual.similar_to', { name: visualState.label })
})
const visualSubtitle = computed(() => {
  if (visualState.status === 'loading') return t('search.visual.mobileclip')
  if (visualState.status === 'error') return visualState.error ?? t('search.visual.error_desc')
  return t('search.visual.pure_similarity')
})

function focus() { input.value?.focus() }
function select() { input.value?.select() }

function handleInput(event: Event) {
  emit('update:modelValue', (event.target as HTMLInputElement).value)
}

function handleKeydown(event: KeyboardEvent) {
  if (visualActive.value && event.key === 'Escape') {
    event.preventDefault()
    leaveVisualSearch()
    return
  }
  if (!props.folderSuggestions.length) return
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    emit('folderNavigate', 1)
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    emit('folderNavigate', -1)
  } else if (event.key === 'Tab' || event.key === 'Enter') {
    const folder = props.folderSuggestions[props.folderSuggestionIndex]
    if (!folder) return
    event.preventDefault()
    emit('folderSelect', folder)
  }
}

function handleVisualSession() {
  const status = visualState.status
  if (status === 'loading' && !visualWasActive) {
    previousQuery.value = visualSearchSession.isVisualQuery(props.modelValue)
      ? ''
      : props.modelValue
    visualWasActive = true
  }
  if (status === 'ready' && visualState.token && props.modelValue !== visualState.token) {
    emit('update:modelValue', visualState.token)
  }
  if (status === 'idle') visualWasActive = false
}

function leaveVisualSearch() {
  const restore = previousQuery.value
  visualSearchSession.clear()
  emit('update:modelValue', restore)
  void nextTick(focus)
}

async function chooseImage() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      title: t('search.visual.choose_file'),
      filters: [{
        name: t('search.visual.image_files'),
        extensions: ['avif', 'webp', 'png', 'jpg', 'jpeg', 'gif', 'bmp', 'tif', 'tiff', 'ico'],
      }],
    })
    if (typeof selected === 'string') await visualSearch.searchPath(selected, 'file')
  } catch (reason) {
    visualSearchSession.fail(reason)
    emit('visualError', reason)
  } finally {
    void nextTick(focus)
  }
}

async function consumeFile(file: File, source: 'clipboard' | 'drop') {
  try {
    await visualSearch.searchFile(file, source)
  } catch (reason) {
    visualSearchSession.fail(reason)
    emit('visualError', reason)
  }
}

function handlePaste(event: ClipboardEvent) {
  if (props.view !== 'search') return
  const item = [...(event.clipboardData?.items ?? [])]
    .find((candidate) => candidate.kind === 'file' && candidate.type.startsWith('image/'))
  const file = item?.getAsFile()
  if (!file) return
  event.preventDefault()
  void consumeFile(file, 'clipboard')
}

function handleDragOver(event: DragEvent) {
  if (props.view !== 'search') return
  if (![...(event.dataTransfer?.items ?? [])].some((item) => item.kind === 'file')) return
  event.preventDefault()
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy'
  dragActive.value = true
}

function handleDragLeave(event: DragEvent) {
  const current = event.currentTarget
  const related = event.relatedTarget
  if (current instanceof Node && related instanceof Node && current.contains(related)) return
  dragActive.value = false
}

function handleDrop(event: DragEvent) {
  if (props.view !== 'search') return
  const file = event.dataTransfer?.files[0]
  dragActive.value = false
  if (!file) return
  event.preventDefault()
  void consumeFile(file, 'drop')
}

onMounted(async () => {
  window.addEventListener(visualSearchSession.eventName, handleVisualSession)
  handleVisualSession()
  try {
    unlistenWindowDrop = await getCurrentWindow().onDragDropEvent(({ payload }) => {
      if (props.view !== 'search') return
      if (payload.type === 'enter' || payload.type === 'over') {
        dragActive.value = true
      } else if (payload.type === 'leave') {
        dragActive.value = false
      } else if (payload.type === 'drop') {
        dragActive.value = false
        const path = payload.paths[0]
        if (!path) return
        void visualSearch.searchPath(path, 'drop')
      }
    })
  } catch {
    // DOM drag/drop remains available in browser and demo contexts.
  }
})

onBeforeUnmount(() => {
  window.removeEventListener(visualSearchSession.eventName, handleVisualSession)
  unlistenWindowDrop?.()
})

defineExpose({ focus, select, chooseImage, leaveVisualSearch })
</script>

<template>
  <div
    class="unified-search-input"
    :class="[
      `unified-search-input--${variant}`,
      {
        'unified-search-input--focused': focused,
        'unified-search-input--visual': visualActive,
        'unified-search-input--drop': dragActive,
      },
    ]"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
  >
    <Button
      v-if="view === 'settings' || visualActive"
      class="unified-search-input__nav"
      variant="ghost"
      size="icon"
      :aria-label="visualActive ? t('search.visual.back') : t('spotlight.back_to_search')"
      @click="visualActive ? leaveVisualSearch() : emit('back')"
    >
      <ArrowLeft :size="variant === 'spotlight' ? 18 : 16" />
    </Button>

    <template v-if="visualActive">
      <span class="unified-search-input__visual-thumb">
        <ThumbnailImage
          v-if="visualState.sourceImage"
          :image="visualState.sourceImage"
          :priority="0"
        />
        <img
          v-else-if="visualState.previewUrl"
          :src="visualState.previewUrl"
          :alt="visualState.label"
          draggable="false"
        />
        <LoaderCircle v-else class="spin" :size="18" />
      </span>
      <span class="unified-search-input__visual-copy" aria-live="polite">
        <strong>{{ visualTitle }}</strong>
        <small>{{ visualSubtitle }}</small>
      </span>
      <span v-if="showVisualCount" class="unified-search-input__count">{{ resultLabel }}</span>
      <KbdChip v-if="variant === 'spotlight'" shortcut="Esc" size="sm" />
    </template>

    <template v-else>
      <div class="unified-search-input__leading">
        <Transition name="icon-swap" mode="out-in">
          <LoaderCircle v-if="searching" key="loader" class="spin" :size="variant === 'spotlight' ? 22 : 18" :stroke-width="2.2" />
          <Search v-else key="search" :size="variant === 'spotlight' ? 22 : 18" :stroke-width="1.9" />
        </Transition>
      </div>

      <input
        ref="input"
        :value="modelValue"
        type="search"
        autocomplete="off"
        spellcheck="false"
        :placeholder="placeholder"
        :aria-label="ariaLabel"
        @focus="focused = true"
        @blur="focused = false"
        @input="handleInput"
        @keydown="handleKeydown"
        @paste="handlePaste"
      />

      <span v-if="showTextCount" class="unified-search-input__count">{{ resultLabel }}</span>
      <Button
        v-if="modelValue"
        class="unified-search-input__clear"
        variant="ghost"
        size="icon"
        :aria-label="t('search.clear')"
        @click="emit('update:modelValue', '')"
      >
        <X :size="15" />
      </Button>
      <Button
        v-if="view === 'search'"
        class="unified-search-input__upload"
        variant="ghost"
        size="icon"
        :aria-label="t('search.visual.action')"
        :title="t('search.visual.drop_paste_choose')"
        @click="chooseImage"
      >
        <ImageUp :size="variant === 'spotlight' ? 18 : 17" />
      </Button>
      <Button
        v-if="view === 'search' && showSettings"
        class="unified-search-input__settings"
        variant="ghost"
        size="icon"
        :aria-label="t('spotlight.open_settings')"
        @click="emit('settings')"
      >
        <Settings2 :size="17" />
      </Button>
    </template>

    <div v-if="dragActive" class="unified-search-input__drop-message" aria-hidden="true">
      <Sparkles :size="17" />
      <span>{{ t('search.visual.drop_here') }}</span>
    </div>

    <FolderQueryAutocomplete
      v-if="view === 'search' && !visualActive"
      :folders="folderSuggestions"
      :active-index="folderSuggestionIndex"
      placement="below"
      @select="emit('folderSelect', $event)"
    />
  </div>
</template>

<style scoped>
.unified-search-input {
  position: relative;
  z-index: 3;
  display: flex;
  align-items: center;
  width: 100%;
  min-width: 0;
  color: var(--text-muted);
  border: 1px solid transparent;
  transition: border-color 150ms ease, background-color 150ms ease, box-shadow 180ms ease;
}
.unified-search-input--app {
  min-height: 48px;
  gap: 9px;
  padding: 0 8px 0 13px;
  border-color: var(--border);
  border-radius: 13px;
  background: var(--surface-elevated);
  box-shadow: 0 4px 20px -4px rgb(0 0 0 / 0.1);
}
.unified-search-input--spotlight {
  min-height: 70px;
  gap: 12px;
  padding: 0 15px 0 18px;
}
.unified-search-input--focused.unified-search-input--app,
.unified-search-input--app:hover { border-color: transparent; }
.unified-search-input--visual {
  background: color-mix(in srgb, var(--surface-elevated) 96%, transparent);
}
.unified-search-input--drop {
  border-color: color-mix(in srgb, var(--primary) 74%, var(--border));
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 14%, transparent), 0 18px 46px -30px var(--primary);
}
.unified-search-input__nav { flex: 0 0 auto; margin-left: -5px; }
.unified-search-input__leading {
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  width: 22px;
  height: 22px;
}
.unified-search-input input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  caret-color: var(--primary);
}
.unified-search-input--app input { font-size: 14px; font-weight: 530; letter-spacing: -0.12px; }
.unified-search-input--spotlight input { font-size: 20px; font-weight: 570; letter-spacing: -0.38px; }
.unified-search-input input::-webkit-search-cancel-button { display: none; }
.unified-search-input input::placeholder { color: var(--text-subtle); opacity: 1; }
.unified-search-input__count {
  flex: 0 0 auto;
  min-width: 78px;
  padding: 5px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--surface-hover) 88%, transparent);
  color: var(--text-muted);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
  text-align: center;
}
.unified-search-input--app .unified-search-input__count { min-width: 86px; font-size: 11px; }
.unified-search-input__clear,
.unified-search-input__upload,
.unified-search-input__settings { flex: 0 0 auto; }
.unified-search-input__upload {
  color: color-mix(in srgb, var(--primary-text) 88%, var(--text-muted));
}
.unified-search-input__visual-thumb {
  position: relative;
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--primary) 24%, var(--border));
  background: var(--surface-hover);
  box-shadow: 0 8px 20px -15px rgb(2 6 23 / 0.62);
}
.unified-search-input--app .unified-search-input__visual-thumb { width: 34px; height: 34px; border-radius: 9px; }
.unified-search-input--spotlight .unified-search-input__visual-thumb { width: 42px; height: 42px; border-radius: 11px; }
.unified-search-input__visual-thumb img,
.unified-search-input__visual-thumb :deep(.thumbnail-loader) { width: 100%; height: 100%; object-fit: cover; }
.unified-search-input__visual-copy { flex: 1; min-width: 0; }
.unified-search-input__visual-copy strong,
.unified-search-input__visual-copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.unified-search-input__visual-copy strong { color: var(--text); font-size: 13px; letter-spacing: -0.14px; }
.unified-search-input--spotlight .unified-search-input__visual-copy strong { font-size: 15px; letter-spacing: -0.24px; }
.unified-search-input__visual-copy small { margin-top: 3px; color: var(--text-muted); font-size: 9px; }
.unified-search-input--spotlight .unified-search-input__visual-copy small { margin-top: 5px; font-size: 10px; }
.unified-search-input__drop-message {
  position: absolute;
  inset: 4px;
  z-index: 8;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border: 1px dashed color-mix(in srgb, var(--primary) 58%, var(--border));
  border-radius: inherit;
  background: color-mix(in srgb, var(--surface-elevated) 86%, var(--primary-soft));
  color: var(--primary-text);
  font-size: 11px;
  font-weight: 680;
  pointer-events: none;
  backdrop-filter: blur(18px) saturate(1.12);
  animation: visual-drop-in 150ms cubic-bezier(0.16, 1, 0.3, 1) both;
}
.spin { animation: spin 0.8s linear infinite; }
.icon-swap-enter-active,
.icon-swap-leave-active { transition: opacity 140ms ease, transform 140ms ease; }
.icon-swap-enter-from,
.icon-swap-leave-to { opacity: 0; transform: scale(0.84); }
@keyframes spin { to { transform: rotate(1turn); } }
@keyframes visual-drop-in { from { opacity: 0; transform: scale(0.985); } to { opacity: 1; transform: none; } }
@media (prefers-reduced-motion: reduce) {
  .spin,
  .unified-search-input__drop-message { animation-duration: 0.01ms; }
}
</style>
