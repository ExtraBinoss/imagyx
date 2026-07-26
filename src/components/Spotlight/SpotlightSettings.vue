<script setup lang="ts">
import { computed } from 'vue'
import { Keyboard, Monitor, Moon, Palette, SearchX, Sun } from '@lucide/vue'
import type { ThemeMode } from '../../stores/theme'
import ShortcutView from '../shortcuts/ShortcutView.vue'
import Button from '../ui/Button/Button.vue'
import ButtonGroup from '../ui/ButtonGroup/ButtonGroup.vue'

const props = defineProps<{
  query: string
  shortcut: string
  shortcutUpdating: boolean
  shortcutError: string | null
  themeMode: ThemeMode
}>()

const emit = defineEmits<{
  shortcutChange: [value: string]
  themeChange: [value: ThemeMode]
}>()

const normalizedQuery = computed(() => props.query.trim().toLocaleLowerCase('fr'))
const showShortcut = computed(() => matches(['raccourci', 'shortcut', 'clavier', 'keybind', 'spotlight', 'ouvrir']))
const showTheme = computed(() => matches(['thème', 'theme', 'apparence', 'clair', 'sombre', 'système', 'couleur']))
const hasResults = computed(() => showShortcut.value || showTheme.value)

function matches(keywords: string[]) {
  const query = normalizedQuery.value
  return !query || keywords.some((keyword) => keyword.includes(query) || query.includes(keyword))
}
</script>

<template>
  <div class="spotlight-settings">
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

    <div v-if="!hasResults" class="settings-empty">
      <SearchX :size="25" />
      <strong>Aucun réglage trouvé</strong>
      <span>Essaie “raccourci”, “thème” ou “apparence”.</span>
    </div>
  </div>
</template>

<style scoped>
.spotlight-settings {
  height: 100%;
  overflow-y: auto;
  padding: 12px;
  scrollbar-width: thin;
}
.settings-section {
  display: grid;
  gap: 13px;
  padding: 15px;
  border: 1px solid var(--border);
  border-radius: 16px;
  background: color-mix(in srgb, var(--surface) 90%, transparent);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.12), 0 12px 28px -24px rgb(15 23 42 / 0.42);
  animation: settings-rise 220ms cubic-bezier(0.16, 1, 0.3, 1) both;
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
@keyframes settings-rise {
  from { opacity: 0; transform: translateY(7px) scale(0.992); }
  to { opacity: 1; transform: none; }
}
</style>
