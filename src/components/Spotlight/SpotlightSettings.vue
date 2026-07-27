<script setup lang="ts">
import { computed, ref } from 'vue'
import { AlignLeft, AlignRight, BookOpen, Check, Copy, Info, Keyboard, Layout, Monitor, Moon, Palette, SearchX, Sun } from '@lucide/vue'
import type { ThemeMode } from '../../stores/theme'
import { usePlatformStore } from '../../stores/platform'
import { useLibraryStore } from '../../stores/library'
import { formatBytes } from '../../utils'
import ShortcutView from '../shortcuts/ShortcutView.vue'
import Accordion from '../ui/Accordion/Accordion.vue'
import Button from '../ui/Button/Button.vue'
import ButtonGroup from '../ui/ButtonGroup/ButtonGroup.vue'

import { copyDebugInfoToClipboard } from '../../utils/copy-information'

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
  onboarding: []
}>()

const platform = usePlatformStore()
const library = useLibraryStore()
const copied = ref(false)

const isPopover = computed(() => props.variant === 'popover')
const normalizedQuery = computed(() => (props.query ?? '').trim().toLocaleLowerCase('fr'))
const showShortcut = computed(() => isPopover.value || matches(['raccourci', 'shortcut', 'clavier', 'keybind', 'spotlight', 'ouvrir']))
const showTheme = computed(() => isPopover.value || matches(['thème', 'theme', 'apparence', 'clair', 'sombre', 'système', 'couleur']))
const showControls = computed(() => isPopover.value || matches(['boutons', 'bouton', 'contrôles', 'controles', 'fermeture', 'réduction', 'reduction', 'fenêtre', 'fenetre', 'position', 'gauche', 'droite', 'titlebar', 'barre']))
const showOnboarding = computed(() => isPopover.value || matches(['onboarding', 'guide', 'tutoriel', 'découvrir', 'decouvrir', 'bienvenue', 'aide', 'fonctionnalités', 'fonctionnalites']))
const showInfo = computed(() => isPopover.value || matches(['info', 'information', 'version', 'débug', 'debug', 'système', 'imagyx', 'stats', 'indexation', 'base', 'sqlite']))
const hasResults = computed(() => showShortcut.value || showTheme.value || showControls.value || showOnboarding.value || showInfo.value)

const stats = computed(() => library.runtimeStats)
const dbPath = computed(() => library.appInfo?.databasePath ?? 'Indisponible')

function matches(keywords: string[]) {
  const query = normalizedQuery.value
  return !query || keywords.some((keyword) => keyword.includes(query) || query.includes(keyword))
}

async function copyDebugInfo() {
  await copyDebugInfoToClipboard({
    appVersion: platform.appVersion,
    platform: platform.platform,
    controlsPosition: platform.controlsPosition,
    databasePath: dbPath.value,
    shortcut: props.shortcut,
    themeMode: props.themeMode,
    runtimeStats: stats.value,
    lastIndexedAt: library.lastIndexedAt,
    foldersCount: library.folders.length,
    totalImagesCount: library.totalImages,
  })
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

    <section v-if="showControls" class="settings-section">
      <header>
        <span class="settings-section__icon"><Layout :size="17" /></span>
        <div><strong>Boutons de fenêtre</strong><p>Position des contrôles de fermeture, réduction et agrandissement.</p></div>
      </header>
      <ButtonGroup full>
        <Button variant="ghost" size="md" :pressed="platform.controlsPosition === 'left'" @click="platform.setControlsPosition('left')">
          <template #leading><AlignLeft :size="16" /></template>Gauche
        </Button>
        <Button variant="ghost" size="md" :pressed="platform.controlsPosition === 'right'" @click="platform.setControlsPosition('right')">
          <template #leading><AlignRight :size="16" /></template>Droite
        </Button>
      </ButtonGroup>
    </section>

    <section v-if="showOnboarding" class="settings-section">
      <header>
        <span class="settings-section__icon"><BookOpen :size="17" /></span>
        <div><strong>Guide de découverte</strong><p>Revois les fonctions principales et ajoute un dossier depuis le parcours.</p></div>
      </header>
      <Button variant="secondary" size="md" block @click="emit('onboarding')">
        <template #leading><BookOpen :size="16" /></template>Relancer l’onboarding
      </Button>
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
            {{ copied ? 'Copied!' : 'Copy additional information' }}
          </Button>
        </div>
      </Accordion>
    </section>

    <div v-if="!hasResults" class="settings-empty">
      <SearchX :size="25" />
      <strong>Aucun réglage trouvé</strong>
      <span>Essaie “raccourci”, “thème”, “onboarding” ou “apparence”.</span>
    </div>
  </div>
</template>

<style scoped>
.spotlight-settings {
  width: 100%;
  max-width: 100%;
  max-height: 68vh;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 4px;
  box-sizing: border-box;
  scrollbar-width: thin;
}
.spotlight-settings--popover {
  max-height: 480px;
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
