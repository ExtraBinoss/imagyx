<script setup lang="ts">
import { computed, ref } from 'vue'
import { Check, Copy, Info, Keyboard, Monitor, Moon, Palette, SearchX, Sun } from '@lucide/vue'
import type { ThemeMode } from '../../stores/theme'
import { usePlatformStore } from '../../stores/platform'
import { useLibraryStore } from '../../stores/library'
import { formatBytes } from '../../utils'
import ShortcutView from '../shortcuts/ShortcutView.vue'
import Accordion from '../ui/Accordion/Accordion.vue'
import Button from '../ui/Button/Button.vue'
import ButtonGroup from '../ui/ButtonGroup/ButtonGroup.vue'

const props = withDefaults(
  defineProps<{
    query?: string
    shortcut: string
    shortcutUpdating: boolean
    shortcutError: string | null
    themeMode: ThemeMode
    variant?: 'spotlight' | 'popover'
  }>(),
  {
    query: '',
    variant: 'popover',
  },
)

const emit = defineEmits<{
  shortcutChange: [value: string]
  themeChange: [value: ThemeMode]
}>()

const platform = usePlatformStore()
const library = useLibraryStore()
const copied = ref(false)

const isPopover = computed(() => props.variant === 'popover')
const normalizedQuery = computed(() => (props.query ?? '').trim().toLocaleLowerCase('fr'))
const showShortcut = computed(() => isPopover.value || matches(['raccourci', 'shortcut', 'clavier', 'keybind', 'spotlight', 'ouvrir']))
const showTheme = computed(() => isPopover.value || matches(['thème', 'theme', 'apparence', 'clair', 'sombre', 'système', 'couleur']))
const showInfo = computed(() => isPopover.value || matches(['info', 'information', 'version', 'débug', 'debug', 'système', 'imagyx', 'stats', 'indexation', 'base', 'sqlite']))
const hasResults = computed(() => showShortcut.value || showTheme.value || showInfo.value)

const stats = computed(() => library.runtimeStats)
const dbPath = computed(() => library.appInfo?.databasePath ?? 'Indisponible')

function formatMs(value?: number) {
  if (!value) return '0 ms'
  if (value < 1_000) return `${Math.round(value)} ms`
  return `${(value / 1_000).toFixed(1)} s`
}

function matches(keywords: string[]) {
  const query = normalizedQuery.value
  return !query || keywords.some((keyword) => keyword.includes(query) || query.includes(keyword))
}

function copyDebugInfo() {
  const infoText = `Imagyx Debug Info:
- Version: v${platform.appVersion}
- OS: ${platform.platform}
- Database: ${dbPath.value}
- AI Model: ${stats.value?.modelName ?? 'MobileCLIP-S0'}
- Active Backend: ${stats.value?.backendEffective ?? 'Automatic'} (Requested: ${stats.value?.backendRequested ?? 'WebGPU'})
- Indexing Stage: ${stats.value?.stage ?? 'Ready'}
- Progress: ${stats.value?.current ?? 0} / ${stats.value?.total ?? 0}
- Throughput: ${stats.value?.imagesPerSecond?.toFixed(1) ?? '0.0'} img/s (${stats.value?.averageMsPerImage ? `${Math.round(stats.value.averageMsPerImage)} ms/img` : '—'})
- Timings: Decode: ${formatMs(stats.value?.decodeMs)} | Inference: ${formatMs(stats.value?.inferenceMs)} | SQLite: ${formatMs(stats.value?.saveMs)}
- Resources: App CPU: ${stats.value?.processCpuPercent?.toFixed(0) ?? 0}% | System CPU: ${stats.value?.systemCpuPercent?.toFixed(0) ?? 0}% | RAM: ${formatBytes(stats.value?.processMemoryBytes ?? 0)}
- Library: ${library.folders.length} watched folder(s) (${library.totalImages} images)
- Shortcut: ${props.shortcut}
- Theme: ${props.themeMode}`

  void navigator.clipboard.writeText(infoText)
  copied.value = true
  setTimeout(() => {
    copied.value = false
  }, 2000)
}
</script>

<template>
  <div class="spotlight-settings" :class="[`spotlight-settings--${variant}`]">
    <section v-if="showShortcut" class="settings-section">
      <header>
        <span class="settings-section__icon"><Keyboard :size="17" /></span>
        <div><strong>Raccourci global</strong><p>Ouvre Imagyx Spotlight depuis n’importe quelle application.</p></div>
      </header>
      <ShortcutView
        :model-value="shortcut"
        label="Ouvrir Spotlight"
        description="Clique dans le champ puis saisis une combinaison avec au moins un modificateur."
        :disabled="shortcutUpdating"
        @change="emit('shortcutChange', $event)"
      />
      <p v-if="shortcutError" class="settings-error">{{ shortcutError }}</p>
    </section>

    <section v-if="showTheme" class="settings-section">
      <header>
        <span class="settings-section__icon"><Palette :size="17" /></span>
        <div><strong>Apparence</strong><p>Le thème est appliqué immédiatement à toutes les fenêtres Imagyx.</p></div>
      </header>
      <ButtonGroup full>
        <Button variant="ghost" size="md" :pressed="themeMode === 'system'" @click="emit('themeChange', 'system')">
          <template #leading><Monitor :size="16" /></template>Système
        </Button>
        <Button variant="ghost" size="md" :pressed="themeMode === 'light'" @click="emit('themeChange', 'light')">
          <template #leading><Sun :size="16" /></template>Clair
        </Button>
        <Button variant="ghost" size="md" :pressed="themeMode === 'dark'" @click="emit('themeChange', 'dark')">
          <template #leading><Moon :size="16" /></template>Sombre
        </Button>
      </ButtonGroup>
    </section>

    <section v-if="showInfo" class="settings-section info-section">
      <Accordion title="Imagyx Information" :default-open="false">
        <template #title>
          <Info :size="15" />
          <span>Imagyx Information</span>
        </template>
        <div class="info-details">
          <div class="info-row">
            <span class="info-label">Version</span>
            <span class="info-value">v{{ platform.appVersion }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">Platform</span>
            <span class="info-value">{{ platform.platform }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">AI Model</span>
            <span class="info-value">MobileCLIP-S0 (WebGPU)</span>
          </div>
          <div class="info-row">
            <span class="info-label">Library</span>
            <span class="info-value">{{ library.folders.length }} folder(s) · {{ library.totalImages }} images</span>
          </div>

          <Button variant="secondary" size="sm" block class="copy-info-btn" @click="copyDebugInfo">
            <template #leading>
              <Check v-if="copied" :size="14" />
              <Copy v-else :size="14" />
            </template>
            {{ copied ? 'Copied!' : 'Copy Debug Info' }}
          </Button>
        </div>
      </Accordion>
    </section>

    <div v-if="!hasResults" class="settings-empty">
      <SearchX :size="25" />
      <strong>Aucun réglage trouvé</strong>
      <span>Essaie “raccourci”, “thème” ou “apparence”.</span>
    </div>
  </div>
</template>

<style scoped>
.spotlight-settings {
  width: 100%;
  max-width: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 4px;
  box-sizing: border-box;
  scrollbar-width: thin;
}
.settings-section {
  display: grid;
  width: 100%;
  box-sizing: border-box;
  padding: 12px 14px;
  gap: 13px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.12), 0 12px 28px -24px rgb(15 23 42 / 0.42);
  animation: settings-rise 220ms cubic-bezier(0.16, 1, 0.3, 1) both;
}
.spotlight-settings--popover .settings-section {
  animation: none;
}
.settings-section + .settings-section { margin-top: 10px; animation-delay: 35ms; }
.settings-section header { display: flex; align-items: center; gap: 11px; }
.settings-section__icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-hover);
  color: var(--primary-text);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.16);
}
.settings-section header strong { display: block; color: var(--text); font-size: 12px; }
.settings-section header p { margin: 4px 0 0; color: var(--text-muted); font-size: 10px; line-height: 1.45; }
.settings-error {
  margin: 0;
  padding: 9px 10px;
  border: 1px solid var(--danger-border);
  border-radius: 10px;
  background: var(--danger-surface);
  color: var(--danger-text);
  font-size: 10px;
}
.settings-empty {
  display: grid;
  place-items: center;
  align-content: center;
  min-height: 310px;
  color: var(--text-muted);
  text-align: center;
}
.settings-empty svg { margin-bottom: 12px; color: var(--primary-text); }
.settings-empty strong { color: var(--text); font-size: 13px; }
.settings-empty span { margin-top: 6px; font-size: 10px; }
.info-section {
  padding: 0;
  overflow: hidden;
}
.info-details {
  display: grid;
  gap: var(--space-2);
}
.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  font-size: var(--text-xs);
}
.info-label {
  color: var(--text-muted);
}
.info-value {
  color: var(--text);
  font-weight: 550;
  font-variant-numeric: tabular-nums;
}
.info-value--path {
  max-width: 190px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.copy-info-btn {
  margin-top: var(--space-2);
}
@keyframes settings-rise {
  from { opacity: 0; transform: translateY(7px) scale(0.992); }
  to { opacity: 1; transform: none; }
}
</style>
