<script setup lang="ts">
import {
  Check,
  Folder,
  FolderOpen,
  Images,
  Info,
  Keyboard,
  Moon,
  MoreHorizontal,
  Search,
  Sparkles,
  Sun,
  WandSparkles,
} from '@lucide/vue'
import type { ThemeMode } from '../../stores/theme'
import ShortcutView from '../shortcuts/ShortcutView.vue'
import SpotlightInput from '../Spotlight/SpotlightInput.vue'
import Button from '../ui/Button/Button.vue'
import ButtonGroup from '../ui/ButtonGroup/ButtonGroup.vue'
import MovingBorder from '../ui/MovingBorder/MovingBorder.vue'

const props = defineProps<{
  kind: 'welcome' | 'sidebar' | 'search' | 'preview' | 'spotlight' | 'settings' | 'indexing'
  themeMode: ThemeMode
  shortcut: string
  hasFolders: boolean
}>()

const emit = defineEmits<{
  addFolder: []
  themeChange: [value: ThemeMode]
}>()

const demoImages = [
  { label: 'Portrait', className: 'demo-image--portrait' },
  { label: 'Plage', className: 'demo-image--beach' },
  { label: 'Ville de nuit', className: 'demo-image--city' },
  { label: 'Chien', className: 'demo-image--dog' },
]
</script>

<template>
  <div class="feature-preview" :class="`feature-preview--${kind}`">
    <template v-if="kind === 'welcome'">
      <div class="mini-app">
        <aside class="mini-sidebar">
          <div class="mini-brand"><span><Sparkles :size="15" /></span><strong>Imagyx</strong></div>
          <Button variant="primary" size="sm" block @click="emit('addFolder')">
            <template #leading><FolderOpen :size="14" /></template>
            {{ hasFolders ? 'Ajouter un autre dossier' : 'Ajouter un dossier' }}
          </Button>
          <div class="mini-sidebar__rows">
            <Button variant="ghost" size="sm" block pressed>
              <template #leading><Images :size="14" /></template>Toutes les images
            </Button>
            <Button variant="ghost" size="sm" block>
              <template #leading><Folder :size="14" /></template>Photos
            </Button>
          </div>
        </aside>
        <section class="mini-workspace">
          <div class="mini-search"><Search :size="15" /><span>sunset beach…</span><kbd>12</kbd></div>
          <div class="mini-grid">
            <div v-for="image in demoImages" :key="image.label" class="mini-card">
              <span class="demo-image" :class="image.className" />
              <strong>{{ image.label }}</strong>
            </div>
          </div>
        </section>
      </div>
    </template>

    <template v-else-if="kind === 'sidebar'">
      <div class="sidebar-demo">
        <div class="sidebar-demo__brand"><span><Sparkles :size="16" /></span><div><strong>Imagyx</strong><small>Intelligence locale</small></div></div>
        <Button variant="primary" size="md" block @click="emit('addFolder')">
          <template #leading><FolderOpen :size="15" /></template>Ajouter un dossier
        </Button>
        <div class="sidebar-demo__list">
          <Button variant="ghost" size="md" block pressed>
            <template #leading><Images :size="16" /></template>
            <span class="row-copy">Toutes les images</span><span class="row-count">1 248</span>
          </Button>
          <div class="folder-row-demo">
            <Button variant="ghost" size="md" block>
              <template #leading><Folder :size="16" /></template>
              <span class="row-copy">Vacances</span><span class="row-count">438</span>
            </Button>
            <Button variant="ghost" size="icon" aria-label="Actions"><MoreHorizontal :size="15" /></Button>
          </div>
        </div>
        <div class="ai-demo"><WandSparkles :size="16" /><div><strong>IA locale prête</strong><small>MobileCLIP-S0 · WebGPU</small></div><span class="status-dot" /></div>
      </div>
    </template>

    <template v-else-if="kind === 'search'">
      <div class="search-demo">
        <MovingBorder border-radius="16px" :duration="4200">
          <div class="search-demo__input"><Search :size="18" /><span>femme en robe rouge dans une ville</span><kbd>36</kbd></div>
        </MovingBorder>
        <div class="search-demo__grid">
          <article v-for="(image, index) in demoImages" :key="image.label" class="search-card" :class="{ 'search-card--selected': index === 0 }">
            <span class="demo-image" :class="image.className">
              <span class="tag-strip"><i>{{ image.label }}</i><i>{{ index % 2 ? 'extérieur' : 'portrait' }}</i><i>{{ index % 2 ? 'lumineux' : 'rouge' }}</i></span>
            </span>
            <strong>{{ image.label }}.jpg</strong><small>{{ 1600 + index * 320 }} × {{ 1100 + index * 180 }}</small>
          </article>
        </div>
      </div>
    </template>

    <template v-else-if="kind === 'preview'">
      <div class="preview-demo">
        <div class="preview-demo__backdrop" />
        <div class="preview-demo__dialog">
          <span class="preview-demo__image demo-image--city" />
          <div class="preview-demo__meta"><strong>city-night.jpg</strong><span>Appuie sur Espace depuis la grille</span></div>
          <Button variant="ghost" size="icon" aria-label="Informations"><Info :size="16" /></Button>
        </div>
        <div class="preview-demo__hint"><kbd>Espace</kbd><span>Preview native dans l’app</span></div>
      </div>
    </template>

    <template v-else-if="kind === 'spotlight'">
      <div class="spotlight-demo">
        <MovingBorder border-radius="20px" :duration="3600">
          <div class="spotlight-demo__surface">
            <SpotlightInput
              model-value="blue car at night"
              view="search"
              placeholder="All images: name or description…"
              :searching="false"
              result-label="8 résultats"
            />
            <div class="spotlight-demo__results">
              <article v-for="(image, index) in demoImages.slice(0, 3)" :key="image.label" :class="{ active: index === 0 }">
                <span class="demo-image" :class="image.className" />
                <div><strong>{{ image.label }}.jpg</strong><small>{{ 92 - index * 6 }} % de correspondance</small></div>
                <div class="spotlight-demo__actions">
                  <Button variant="secondary" size="sm">Copier</Button>
                  <Button variant="primary" size="sm">Imagyx</Button>
                </div>
              </article>
            </div>
          </div>
        </MovingBorder>
      </div>
    </template>

    <template v-else-if="kind === 'settings'">
      <div class="settings-demo">
        <ShortcutView
          :model-value="shortcut"
          label="Ouvrir Spotlight"
          description="Le raccourci global est modifiable à la volée."
          disabled
        />
        <div class="settings-demo__section">
          <header><Keyboard :size="16" /><div><strong>Apparence</strong><small>Appliquée immédiatement à toutes les fenêtres.</small></div></header>
          <ButtonGroup full>
            <Button variant="ghost" size="sm" :pressed="themeMode === 'system'" @click="emit('themeChange', 'system')">
              <template #leading><Check v-if="themeMode === 'system'" :size="13" /></template>Système
            </Button>
            <Button variant="ghost" size="sm" :pressed="themeMode === 'light'" @click="emit('themeChange', 'light')">
              <template #leading><Sun :size="13" /></template>Clair
            </Button>
            <Button variant="ghost" size="sm" :pressed="themeMode === 'dark'" @click="emit('themeChange', 'dark')">
              <template #leading><Moon :size="13" /></template>Sombre
            </Button>
          </ButtonGroup>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="indexing-demo">
        <div class="indexing-demo__hero"><FolderOpen :size="28" /><div><strong>{{ hasFolders ? 'Ta bibliothèque est prête à grandir' : 'Ajoute ton premier dossier' }}</strong><span>L’indexation continue en arrière-plan pendant que tu recherches.</span></div></div>
        <div class="indexing-demo__job">
          <span><WandSparkles :size="17" /></span>
          <div><strong>Photos</strong><small>Analyse IA · 684 sur 1 248</small><div class="progress-track"><i /></div></div>
          <b>55 %</b>
        </div>
        <Button variant="primary" size="lg" block @click="emit('addFolder')">
          <template #leading><FolderOpen :size="17" /></template>
          {{ hasFolders ? 'Ajouter un autre dossier' : 'Choisir un dossier maintenant' }}
        </Button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.feature-preview {
  width: 100%;
  min-height: 330px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
  border-radius: 20px;
  background:
    radial-gradient(circle at 14% 8%, color-mix(in srgb, var(--primary) 8%, transparent), transparent 34%),
    color-mix(in srgb, var(--background) 72%, var(--surface));
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.1), 0 24px 50px -42px rgb(15 23 42 / 0.55);
}
.mini-app { display: grid; grid-template-columns: 150px minmax(0, 1fr); min-height: 330px; }
.mini-sidebar { display: flex; flex-direction: column; gap: 12px; padding: 18px 14px; border-right: 1px solid var(--border); background: color-mix(in srgb, var(--sidebar) 94%, transparent); }
.mini-brand,
.sidebar-demo__brand { display: flex; align-items: center; gap: 9px; }
.mini-brand > span,
.sidebar-demo__brand > span { display: grid; place-items: center; width: 29px; height: 29px; border-radius: 9px; background: var(--primary); color: white; box-shadow: inset 0 1px rgb(255 255 255 / 0.24); }
.mini-brand strong { font-size: 12px; }
.mini-sidebar__rows { display: grid; gap: 4px; }
.mini-sidebar :deep(.ui-button) { justify-content: flex-start; font-size: 9px; }
.mini-workspace { padding: 20px; }
.mini-search { display: flex; align-items: center; gap: 9px; min-height: 42px; padding: 0 13px; border: 1px solid var(--border); border-radius: 13px; background: var(--surface); color: var(--text-muted); box-shadow: var(--btn-depth-shadow); font-size: 10px; }
.mini-search span { flex: 1; color: var(--text-secondary); }
kbd { padding: 4px 7px; border: 1px solid var(--border); border-bottom-color: var(--border-strong); border-radius: 7px; background: var(--surface-elevated); color: var(--text-muted); font: inherit; font-size: 9px; box-shadow: 0 2px 0 var(--border); }
.mini-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; margin-top: 18px; }
.mini-card { padding: 7px; border: 1px solid var(--border); border-radius: 12px; background: var(--surface); box-shadow: inset 0 1px rgb(255 255 255 / 0.12), 0 10px 22px -20px rgb(15 23 42 / 0.5); }
.mini-card strong { display: block; margin-top: 7px; font-size: 9px; }
.demo-image { position: relative; display: block; width: 100%; aspect-ratio: 1.45; overflow: hidden; border-radius: 9px; background: var(--skeleton); }
.demo-image--portrait { background: radial-gradient(circle at 50% 30%, #f6d0bb 0 15%, transparent 16%), linear-gradient(135deg, #8b5cf6, #312e81 54%, #111827); }
.demo-image--beach { background: linear-gradient(#7dd3fc 0 45%, #fde68a 46% 68%, #0ea5e9 69%); }
.demo-image--city { background: radial-gradient(circle at 72% 18%, #dbeafe 0 3%, transparent 4%), linear-gradient(125deg, #020617, #1e3a8a 55%, #0f172a); }
.demo-image--dog { background: radial-gradient(circle at 48% 48%, #d6b38a 0 24%, transparent 25%), linear-gradient(145deg, #bbf7d0, #166534); }
.sidebar-demo { display: flex; flex-direction: column; width: min(330px, calc(100% - 42px)); min-height: 330px; margin: 0 auto; padding: 22px 18px; border-inline: 1px solid var(--border); background: color-mix(in srgb, var(--sidebar) 96%, transparent); }
.sidebar-demo__brand { margin-bottom: 17px; }
.sidebar-demo__brand div { display: grid; }
.sidebar-demo__brand strong { font-size: 13px; }
.sidebar-demo__brand small { margin-top: 3px; color: var(--text-muted); font-size: 9px; }
.sidebar-demo__list { display: grid; gap: 5px; margin-top: 16px; }
.sidebar-demo__list :deep(.ui-button) { justify-content: flex-start; }
.row-copy { flex: 1; text-align: left; }
.row-count { color: var(--text-muted); font-size: 9px; }
.folder-row-demo { position: relative; display: flex; gap: 4px; }
.folder-row-demo > :first-child { flex: 1; }
.ai-demo { display: flex; align-items: center; gap: 10px; margin-top: auto; padding: 11px; border: 1px solid var(--border); border-radius: 12px; background: var(--surface); box-shadow: var(--btn-depth-shadow); }
.ai-demo div { flex: 1; display: grid; }
.ai-demo strong { font-size: 10px; }
.ai-demo small { margin-top: 4px; color: var(--text-muted); font-size: 8px; }
.status-dot { width: 7px; height: 7px; border-radius: 50%; background: #22c55e; box-shadow: 0 0 0 4px rgb(34 197 94 / 0.12); }
.search-demo { padding: 22px; }
.search-demo__input { display: flex; align-items: center; gap: 10px; min-height: 52px; padding: 0 15px; border-radius: 15px; background: color-mix(in srgb, var(--surface-elevated) 96%, transparent); color: var(--text-muted); }
.search-demo__input span { flex: 1; color: var(--text); font-size: 12px; }
.search-demo__grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; margin-top: 18px; }
.search-card { padding: 6px; border: 1px solid transparent; border-radius: 12px; background: var(--surface); box-shadow: 0 12px 22px -22px rgb(15 23 42 / 0.5); }
.search-card--selected { border-color: var(--primary); box-shadow: 0 0 0 2px color-mix(in srgb, var(--primary) 14%, transparent); }
.search-card strong,
.search-card small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.search-card strong { margin-top: 7px; font-size: 9px; }
.search-card small { margin-top: 3px; color: var(--text-muted); font-size: 8px; }
.tag-strip { position: absolute; right: 0; bottom: 0; left: 0; display: flex; gap: 5px; padding: 17px 6px 6px; background: linear-gradient(transparent, rgb(2 6 23 / 0.78)); }
.tag-strip i { padding: 3px 5px; border-radius: 99px; background: rgb(255 255 255 / 0.15); color: white; font-size: 7px; font-style: normal; white-space: nowrap; }
.preview-demo { position: relative; min-height: 330px; display: grid; place-items: center; padding: 26px; }
.preview-demo__backdrop { position: absolute; inset: 0; background: radial-gradient(circle at 50% 40%, color-mix(in srgb, var(--primary) 13%, transparent), transparent 48%); }
.preview-demo__dialog { position: relative; z-index: 1; display: grid; grid-template-columns: minmax(0, 1fr) auto; width: min(520px, 92%); padding: 12px; border: 1px solid var(--border); border-radius: 18px; background: color-mix(in srgb, var(--surface-elevated) 96%, transparent); box-shadow: inset 0 1px rgb(255 255 255 / 0.12), 0 30px 70px -34px rgb(2 6 23 / 0.7); }
.preview-demo__image { grid-column: 1 / -1; min-height: 205px; border-radius: 13px; }
.preview-demo__meta { align-self: center; padding: 11px 5px 1px; }
.preview-demo__meta strong,
.preview-demo__meta span { display: block; }
.preview-demo__meta strong { font-size: 11px; }
.preview-demo__meta span { margin-top: 4px; color: var(--text-muted); font-size: 9px; }
.preview-demo__hint { position: absolute; z-index: 2; bottom: 14px; display: flex; align-items: center; gap: 8px; color: var(--text-muted); font-size: 9px; }
.spotlight-demo { padding: 22px; }
.spotlight-demo__surface { overflow: hidden; border-radius: 19px; background: color-mix(in srgb, var(--surface-elevated) 96%, transparent); }
.spotlight-demo__results { padding: 5px 8px 9px; border-top: 1px solid var(--border); }
.spotlight-demo__results article { position: relative; display: grid; grid-template-columns: 44px minmax(0, 1fr) auto; align-items: center; gap: 10px; min-height: 58px; padding: 7px 9px; border: 1px solid transparent; border-radius: 12px; }
.spotlight-demo__results article.active { border-color: color-mix(in srgb, var(--primary) 30%, var(--border)); background: var(--primary-soft); }
.spotlight-demo__results .demo-image { width: 44px; aspect-ratio: 1; }
.spotlight-demo__results strong,
.spotlight-demo__results small { display: block; }
.spotlight-demo__results strong { font-size: 10px; }
.spotlight-demo__results small { margin-top: 4px; color: var(--text-muted); font-size: 8px; }
.spotlight-demo__actions { display: flex; gap: 5px; opacity: 0; transform: translateX(8px); transition: opacity 160ms ease, transform 180ms ease; }
.spotlight-demo__results article:hover .spotlight-demo__actions,
.spotlight-demo__results article.active .spotlight-demo__actions { opacity: 1; transform: none; }
.settings-demo { display: grid; gap: 13px; padding: 22px; }
.settings-demo__section { display: grid; gap: 13px; padding: 14px; border: 1px solid var(--border); border-radius: 14px; background: var(--surface); box-shadow: var(--btn-depth-shadow); }
.settings-demo__section header { display: flex; align-items: center; gap: 10px; }
.settings-demo__section header div { display: grid; }
.settings-demo__section strong { font-size: 11px; }
.settings-demo__section small { margin-top: 4px; color: var(--text-muted); font-size: 9px; }
.indexing-demo { display: grid; gap: 16px; max-width: 510px; margin: 0 auto; padding: 32px 26px; }
.indexing-demo__hero { display: flex; align-items: center; gap: 14px; }
.indexing-demo__hero > svg { color: var(--primary-text); }
.indexing-demo__hero div { display: grid; }
.indexing-demo__hero strong { font-size: 16px; }
.indexing-demo__hero span { margin-top: 6px; color: var(--text-muted); font-size: 10px; line-height: 1.5; }
.indexing-demo__job { display: grid; grid-template-columns: 38px minmax(0, 1fr) auto; align-items: center; gap: 12px; padding: 13px; border: 1px solid color-mix(in srgb, var(--primary) 24%, var(--border)); border-radius: 14px; background: color-mix(in srgb, var(--primary-soft) 48%, var(--surface)); box-shadow: inset 0 1px rgb(255 255 255 / 0.14), 0 15px 28px -26px rgb(15 23 42 / 0.5); }
.indexing-demo__job > span { display: grid; place-items: center; width: 38px; height: 38px; border-radius: 11px; background: var(--surface); color: var(--primary-text); }
.indexing-demo__job div { min-width: 0; }
.indexing-demo__job strong,
.indexing-demo__job small { display: block; }
.indexing-demo__job strong { font-size: 11px; }
.indexing-demo__job small { margin-top: 4px; color: var(--text-muted); font-size: 9px; }
.indexing-demo__job b { color: var(--primary-text); font-size: 10px; }
.progress-track { height: 4px; margin-top: 9px; overflow: hidden; border-radius: 99px; background: var(--border); }
.progress-track i { display: block; width: 55%; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--primary), #93c5fd); animation: progress-pulse 1.7s ease-in-out infinite; }
@keyframes progress-pulse { 50% { filter: brightness(1.25); } }
@media (max-width: 760px) {
  .search-demo__grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .mini-app { grid-template-columns: 120px minmax(0, 1fr); }
}
</style>
