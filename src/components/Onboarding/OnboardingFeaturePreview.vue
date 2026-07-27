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
  Plus,
  Search,
  Sun,
  WandSparkles,
} from "@lucide/vue";
import type { ThemeMode } from "../../stores/theme";
import tribalPortrait from "../../assets/onboarding_assets/99003f6e5f05348d1852c38ed196d988.jpg";
import womanGreen from "../../assets/onboarding_assets/girl_train_segmented.png";
import womanPortrait from "../../assets/onboarding_assets/karina armageddon.jpg";
import greenFashion from "../../assets/onboarding_assets/téléchargement (2).jpg";
import imagyxLogo from "../../../src-tauri/icons/imagyx-bigger.avif";
import ShortcutView from "../shortcuts/ShortcutView.vue";
import SpotlightInput from "../Spotlight/SpotlightInput.vue";
import Button from "../ui/Button/Button.vue";
import ButtonGroup from "../ui/ButtonGroup/ButtonGroup.vue";
import KbdChip from "../ui/KbdChip/KbdChip.vue";
import MovingBorder from "../ui/MovingBorder/MovingBorder.vue";
import { useTranslate } from "../../i18n";

defineProps<{
  kind:
    | "welcome"
    | "sidebar"
    | "search"
    | "preview"
    | "spotlight"
    | "settings"
    | "indexing";
  themeMode: ThemeMode;
  shortcut: string;
  hasFolders: boolean;
}>();

const { t } = useTranslate();

const emit = defineEmits<{
  addFolder: [];
  themeChange: [value: ThemeMode];
}>();

const demoImages = [
  {
    src: womanGreen,
    label: "Woman in green",
    fileName: "girl_train_segmented.png",
    tags: ["woman", "green", "train"],
  },
  {
    src: womanPortrait,
    label: "Woman portrait",
    fileName: "karina armageddon.jpg",
    tags: ["woman", "portrait", "editorial"],
  },
  {
    src: tribalPortrait,
    label: "Tribal style",
    fileName: "99003f6e5f05348d1852c38ed196d988.jpg",
    tags: ["tribal", "woman", "pattern"],
  },
  {
    src: greenFashion,
    label: "Green fashion",
    fileName: "téléchargement (2).jpg",
    tags: ["green", "fashion", "woman"],
  },
];
</script>

<template>
  <div class="feature-preview" :class="`feature-preview--${kind}`">
    <div v-if="kind === 'welcome'" class="mini-app">
      <aside class="mini-sidebar">
        <div class="mini-brand">
          <img :src="imagyxLogo" class="brand-logo" alt="Imagyx logo" />
          <div>
            <strong>Imagyx</strong><small>{{ t("app.tagline") }}</small>
          </div>
        </div>
        <Button
          class="preview-row-button"
          variant="primary"
          size="sm"
          block
          @click="emit('addFolder')"
        >
          <template #leading><Plus :size="14" :stroke-width="2.2" /></template>
          {{ t("sidebar.add_folder") }}
        </Button>
        <div class="mini-sidebar__rows">
          <span class="mini-section-label">{{ t("sidebar.library") }}</span>
          <Button
            class="preview-row-button"
            variant="ghost"
            size="sm"
            block
            pressed
          >
            <template #leading><Images :size="14" /></template
            >{{ t("sidebar.all_images") }}
          </Button>
          <span class="mini-section-label">{{
            t("sidebar.watched_folders")
          }}</span>
          <Button class="preview-row-button" variant="ghost" size="sm" block>
            <template #leading><Folder :size="14" /></template>Visual references
          </Button>
        </div>
      </aside>
      <section class="mini-workspace">
        <div class="mini-search">
          <Search :size="15" />
          <span>woman green</span>
          <KbdChip shortcut="Ctrl+K" size="sm" />
        </div>
        <div class="mini-grid">
          <div
            v-for="image in demoImages"
            :key="image.fileName"
            class="mini-card"
          >
            <img class="demo-image" :src="image.src" :alt="image.label" />
            <strong>{{ image.fileName }}</strong>
          </div>
        </div>
      </section>
    </div>

    <div v-else-if="kind === 'sidebar'" class="sidebar-demo">
      <div class="sidebar-demo__brand">
        <img :src="imagyxLogo" class="brand-logo" alt="Imagyx logo" />
        <div>
          <strong>Imagyx</strong><small>{{ t("app.tagline") }}</small>
        </div>
      </div>
      <Button
        class="preview-row-button"
        variant="primary"
        size="md"
        block
        @click="emit('addFolder')"
      >
        <template #leading><Plus :size="16" :stroke-width="2.2" /></template
        >{{ t("sidebar.add_folder") }}
      </Button>
      <div class="sidebar-demo__list">
        <span class="sidebar-section-label">{{ t("sidebar.library") }}</span>
        <Button
          class="preview-row-button"
          variant="ghost"
          size="md"
          block
          pressed
        >
          <template #leading><Images :size="16" /></template>
          <span class="row-copy">{{ t("sidebar.all_images") }}</span
          ><span class="row-count">1,248</span>
        </Button>
        <span class="sidebar-section-label">{{
          t("sidebar.watched_folders")
        }}</span>
        <div class="folder-row-demo">
          <Button class="preview-row-button" variant="ghost" size="md" block>
            <template #leading><Folder :size="16" /></template>
            <span class="row-copy">Visual references</span
            ><span class="row-count">438</span>
          </Button>
          <Button
            variant="ghost"
            size="icon"
            :aria-label="t('sidebar.folder_actions')"
            ><MoreHorizontal :size="15"
          /></Button>
        </div>
      </div>
      <div class="ai-demo">
        <WandSparkles :size="16" />
        <div>
          <strong>{{ t("indexing.title.ready") }}</strong
          ><small>MobileCLIP-S0 · WebGPU</small>
        </div>
        <span class="status-dot" />
      </div>
    </div>

    <div v-else-if="kind === 'search'" class="search-demo">
      <div class="search-demo__header">
        <MovingBorder border-radius="16px" :duration="4200">
          <div class="search-demo__input">
            <Search :size="18" />
            <span>woman green</span>
            <strong>{{ t("search.result_count", { count: 4 }) }}</strong>
          </div>
        </MovingBorder>
        <span class="shortcut-hint"
          ><KbdChip shortcut="Space" size="md" />
          {{ t("preview.open_file_title") }}</span
        >
      </div>
      <div class="search-demo__grid">
        <article
          v-for="(image, index) in demoImages"
          :key="image.fileName"
          class="search-card"
          :class="{ 'search-card--selected': index === 0 }"
        >
          <span class="demo-image-shell">
            <img class="demo-image" :src="image.src" :alt="image.label" />
            <span class="tag-strip">
              <i v-for="tag in image.tags" :key="tag">{{ tag }}</i>
            </span>
          </span>
          <strong>{{ image.fileName }}</strong>
          <small>{{ 1600 + index * 320 }} × {{ 1100 + index * 180 }}</small>
        </article>
      </div>
    </div>

    <div v-else-if="kind === 'preview'" class="preview-demo">
      <div class="preview-demo__backdrop" />
      <div class="preview-demo__dialog">
        <img
          class="preview-demo__image"
          :src="demoImages[0].src"
          alt="Woman in green preview"
        />
        <div class="preview-demo__meta">
          <strong>{{ demoImages[0].fileName }}</strong>
          <span>Full-resolution preview without leaving the grid</span>
        </div>
        <Button
          variant="ghost"
          size="icon"
          :aria-label="t('preview.information')"
          ><Info :size="16"
        /></Button>
      </div>
      <div class="preview-demo__hints">
        <span><KbdChip shortcut="Space" size="md" /> Open or close</span>
        <span><KbdChip shortcut="Esc" size="md" /> Close</span>
      </div>
    </div>

    <div v-else-if="kind === 'spotlight'" class="spotlight-demo">
      <div class="spotlight-demo__shortcut">
        <span>Open from anywhere</span
        ><KbdChip :shortcut="shortcut" size="md" />
      </div>
      <MovingBorder border-radius="20px" :duration="3600">
        <div class="spotlight-demo__surface">
          <SpotlightInput
            model-value="tribal"
            view="search"
            :placeholder="t('spotlight.placeholder.search')"
            :searching="false"
            :result-label="t('spotlight.result_count', { count: 3 })"
          />
          <div class="spotlight-demo__results">
            <article
              v-for="(image, index) in demoImages.slice(1, 4)"
              :key="image.fileName"
              :class="{ active: index === 0 }"
            >
              <img class="demo-image" :src="image.src" :alt="image.label" />
              <div>
                <strong>{{ image.fileName }}</strong
                ><small>{{ 92 - index * 6 }}% match</small>
              </div>
              <div class="spotlight-demo__actions">
                <Button variant="secondary" size="sm"
                  >Copy <KbdChip shortcut="Ctrl+C" size="sm"
                /></Button>
                <Button variant="secondary" size="sm"
                  >Explorer <KbdChip shortcut="Ctrl+E" size="sm"
                /></Button>
                <Button variant="primary" size="sm"
                  >Imagyx <KbdChip shortcut="Ctrl+I" size="sm"
                /></Button>
              </div>
            </article>
          </div>
        </div>
      </MovingBorder>
    </div>

    <div v-else-if="kind === 'settings'" class="settings-demo">
      <div class="settings-demo__shortcut-summary">
        <div>
          <Keyboard :size="16" /><span
            ><strong>{{ t("settings.shortcut_title") }}</strong
            ><small>{{ t("settings.shortcut_desc") }}</small></span
          >
        </div>
        <KbdChip :shortcut="shortcut" size="md" />
      </div>
      <ShortcutView
        :model-value="shortcut"
        :label="t('settings.shortcut_label')"
        :description="t('settings.shortcut_desc')"
        disabled
      />
      <div class="settings-demo__section">
        <header>
          <Keyboard :size="16" />
          <div>
            <strong>{{ t("settings.appearance_title") }}</strong
            ><small>{{ t("settings.appearance_desc") }}</small>
          </div>
        </header>
        <ButtonGroup full>
          <Button
            variant="ghost"
            size="sm"
            :pressed="themeMode === 'system'"
            @click="emit('themeChange', 'system')"
          >
            <template #leading
              ><Check v-if="themeMode === 'system'" :size="13" /></template
            >System
          </Button>
          <Button
            variant="ghost"
            size="sm"
            :pressed="themeMode === 'light'"
            @click="emit('themeChange', 'light')"
          >
            <template #leading><Sun :size="13" /></template>Light
          </Button>
          <Button
            variant="ghost"
            size="sm"
            :pressed="themeMode === 'dark'"
            @click="emit('themeChange', 'dark')"
          >
            <template #leading><Moon :size="13" /></template>Dark
          </Button>
        </ButtonGroup>
      </div>
    </div>

    <div v-else class="indexing-demo">
      <div class="indexing-demo__hero">
        <FolderOpen :size="28" />
        <div>
          <strong>{{
            hasFolders
              ? t("indexing.title.completed")
              : t("search.no_folder_title")
          }}</strong>
          <span
            >Indexing continues in the background while search remains
            available.</span
          >
        </div>
      </div>
      <div class="indexing-demo__job">
        <span><WandSparkles :size="17" /></span>
        <div>
          <strong>Visual references</strong
          ><small>{{
            t("indexing.message.embedding", { current: 684, total: 1248 })
          }}</small>
          <div class="progress-track"><i /></div>
        </div>
        <b>55%</b>
      </div>
      <Button variant="primary" size="lg" block @click="emit('addFolder')">
        <template #leading><FolderOpen :size="17" /></template>
        {{ hasFolders ? "Add another folder" : "Choose a folder now" }}
      </Button>
    </div>
  </div>
</template>

<style scoped>
.feature-preview {
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
  border-radius: 20px;
  background:
    radial-gradient(
      circle at 14% 8%,
      color-mix(in srgb, var(--primary) 8%, transparent),
      transparent 34%
    ),
    color-mix(in srgb, var(--background) 72%, var(--surface));
  box-shadow:
    inset 0 1px 0 rgb(255 255 255 / 0.1),
    0 24px 50px -42px rgb(15 23 42 / 0.55);
}
.preview-row-button {
  justify-content: flex-start;
}
.mini-app {
  display: grid;
  grid-template-columns: 150px minmax(0, 1fr);
  height: 100%;
  min-height: 0;
}
.mini-sidebar {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px 14px;
  border-right: 1px solid var(--border);
  background: color-mix(in srgb, var(--sidebar) 94%, transparent);
}
.mini-brand,
.sidebar-demo__brand {
  display: flex;
  align-items: center;
  gap: 9px;
}
.brand-logo {
  width: 28px;
  height: 28px;
  object-fit: contain;
  flex-shrink: 0;
}
.mini-brand strong,
.sidebar-demo__brand strong {
  font-size: 13px;
  line-height: 1.1;
}
.mini-brand small,
.sidebar-demo__brand small {
  font-size: 9px;
  color: var(--text-muted);
  display: block;
}
.mini-sidebar__rows {
  display: grid;
  gap: 4px;
}
.mini-section-label {
  font-size: 8px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-top: 6px;
  margin-bottom: 2px;
}
.sidebar-section-label {
  font-size: 9px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-top: 8px;
  margin-bottom: 2px;
}
.mini-sidebar .preview-row-button {
  font-size: 9px;
}
.mini-sidebar__rows {
  display: grid;
  gap: 4px;
}
.mini-sidebar .preview-row-button {
  font-size: 9px;
}
.mini-workspace {
  min-height: 0;
  overflow: hidden;
  padding: 20px;
}
.mini-search {
  display: flex;
  align-items: center;
  gap: 9px;
  min-height: 42px;
  padding: 0 13px;
  border: 1px solid var(--border);
  border-radius: 13px;
  background: var(--surface);
  color: var(--text-muted);
  box-shadow: var(--btn-depth-shadow);
  font-size: 10px;
}
.mini-search span {
  flex: 1;
  color: var(--text-secondary);
}
.mini-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin-top: 18px;
}
.mini-card {
  padding: 7px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.12),
    0 10px 22px -20px rgb(15 23 42 / 0.5);
}
.mini-card strong {
  display: block;
  margin-top: 7px;
  font-size: 9px;
}
.demo-image {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  background: var(--skeleton);
}
.sidebar-demo {
  display: flex;
  flex-direction: column;
  width: min(330px, calc(100% - 42px));
  height: 100%;
  margin: 0 auto;
  padding: 22px 18px;
  border-inline: 1px solid var(--border);
  background: color-mix(in srgb, var(--sidebar) 96%, transparent);
}
.sidebar-demo__brand {
  margin-bottom: 17px;
}
.sidebar-demo__brand div {
  display: grid;
}
.sidebar-demo__brand strong {
  font-size: 13px;
}
.sidebar-demo__brand small {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 9px;
}
.sidebar-demo__list {
  display: grid;
  gap: 5px;
  margin-top: 16px;
}
.row-copy {
  flex: 1;
  text-align: left;
}
.row-count {
  color: var(--text-muted);
  font-size: 9px;
}
.folder-row-demo {
  display: flex;
  gap: 4px;
}
.folder-row-demo > :first-child {
  flex: 1;
}
.ai-demo {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: auto;
  padding: 11px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--btn-depth-shadow);
}
.ai-demo div {
  flex: 1;
  display: grid;
}
.ai-demo strong {
  font-size: 10px;
}
.ai-demo small {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 8px;
}
.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #22c55e;
  box-shadow: 0 0 0 4px rgb(34 197 94 / 0.12);
}
.search-demo {
  height: 100%;
  padding: 22px;
}
.search-demo__header {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 14px;
}
.search-demo__input {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 0 15px;
  border-radius: 15px;
  background: color-mix(in srgb, var(--surface-elevated) 96%, transparent);
  color: var(--text-muted);
}
.search-demo__input span {
  flex: 1;
  color: var(--text);
  font-size: 12px;
}
.search-demo__input strong {
  color: var(--text-muted);
  font-size: 9px;
}
.shortcut-hint {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--text-muted);
  font-size: 9px;
  white-space: nowrap;
}
.search-demo__grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
  margin-top: 18px;
}
.search-card {
  min-width: 0;
  padding: 6px;
  border: 1px solid transparent;
  border-radius: 12px;
  background: var(--surface);
  box-shadow: 0 12px 22px -22px rgb(15 23 42 / 0.5);
}
.search-card--selected {
  border-color: var(--primary);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--primary) 14%, transparent);
}
.search-card strong,
.search-card small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.search-card strong {
  margin-top: 7px;
  font-size: 9px;
}
.search-card small {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 8px;
}
.demo-image-shell {
  position: relative;
  display: block;
  width: 100%;
  aspect-ratio: 1.45;
  overflow: hidden;
  border-radius: 9px;
}
.tag-strip {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  display: flex;
  gap: 5px;
  padding: 17px 6px 6px;
  overflow: hidden;
  background: linear-gradient(transparent, rgb(2 6 23 / 0.78));
}
.tag-strip i {
  padding: 3px 5px;
  border-radius: 99px;
  background: rgb(255 255 255 / 0.15);
  color: white;
  font-size: 7px;
  font-style: normal;
  white-space: nowrap;
}
.preview-demo {
  position: relative;
  height: 100%;
  display: grid;
  place-items: center;
  padding: 26px;
}
.preview-demo__backdrop {
  position: absolute;
  inset: 0;
  background: radial-gradient(
    circle at 50% 40%,
    color-mix(in srgb, var(--primary) 13%, transparent),
    transparent 48%
  );
}
.preview-demo__dialog {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  width: min(520px, 92%);
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 18px;
  background: color-mix(in srgb, var(--surface-elevated) 96%, transparent);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.12),
    0 30px 70px -34px rgb(2 6 23 / 0.7);
}
.preview-demo__image {
  grid-column: 1 / -1;
  width: 100%;
  height: 205px;
  object-fit: contain;
  border-radius: 13px;
  background: var(--background);
}
.preview-demo__meta {
  align-self: center;
  padding: 11px 5px 1px;
}
.preview-demo__meta strong,
.preview-demo__meta span {
  display: block;
}
.preview-demo__meta strong {
  font-size: 11px;
}
.preview-demo__meta span {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 9px;
}
.preview-demo__hints {
  position: absolute;
  z-index: 2;
  bottom: 12px;
  display: flex;
  align-items: center;
  gap: 14px;
  color: var(--text-muted);
  font-size: 9px;
}
.preview-demo__hints span {
  display: flex;
  align-items: center;
  gap: 7px;
}
.spotlight-demo {
  height: 100%;
  padding: 18px 22px 22px;
}
.spotlight-demo__shortcut {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  min-height: 28px;
  color: var(--text-muted);
  font-size: 9px;
}
.spotlight-demo__surface {
  overflow: hidden;
  border-radius: 19px;
  background: color-mix(in srgb, var(--surface-elevated) 96%, transparent);
}
.spotlight-demo__results {
  padding: 5px 8px 9px;
  border-top: 1px solid var(--border);
}
.spotlight-demo__results article {
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  min-height: 58px;
  padding: 7px 9px;
  border: 1px solid transparent;
  border-radius: 12px;
}
.spotlight-demo__results article.active {
  border-color: color-mix(in srgb, var(--primary) 30%, var(--border));
  background: var(--primary-soft);
}
.spotlight-demo__results .demo-image {
  width: 44px;
  height: 44px;
  border-radius: 9px;
}
.spotlight-demo__results strong,
.spotlight-demo__results small {
  display: block;
}
.spotlight-demo__results strong {
  font-size: 10px;
}
.spotlight-demo__results small {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 8px;
}
.spotlight-demo__actions {
  display: flex;
  gap: 5px;
  opacity: 0;
  transform: translateX(8px);
  transition:
    opacity 160ms ease,
    transform 180ms ease;
}
.spotlight-demo__results article:hover .spotlight-demo__actions,
.spotlight-demo__results article.active .spotlight-demo__actions {
  opacity: 1;
  transform: none;
}
.settings-demo {
  display: grid;
  align-content: center;
  gap: 13px;
  height: 100%;
  padding: 22px;
}
.settings-demo__shortcut-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--surface);
}
.settings-demo__shortcut-summary > div {
  display: flex;
  align-items: center;
  gap: 10px;
}
.settings-demo__shortcut-summary span {
  display: grid;
}
.settings-demo__shortcut-summary strong {
  font-size: 11px;
}
.settings-demo__shortcut-summary small {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 9px;
}
.settings-demo__section {
  display: grid;
  gap: 13px;
  padding: 14px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: var(--btn-depth-shadow);
}
.settings-demo__section header {
  display: flex;
  align-items: center;
  gap: 10px;
}
.settings-demo__section header div {
  display: grid;
}
.settings-demo__section strong {
  font-size: 11px;
}
.settings-demo__section small {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 9px;
}
.indexing-demo {
  display: grid;
  align-content: center;
  gap: 16px;
  width: min(510px, calc(100% - 52px));
  height: 100%;
  margin: 0 auto;
}
.indexing-demo__hero {
  display: flex;
  align-items: center;
  gap: 14px;
}
.indexing-demo__hero > svg {
  color: var(--primary-text);
}
.indexing-demo__hero div {
  display: grid;
}
.indexing-demo__hero strong {
  font-size: 16px;
}
.indexing-demo__hero span {
  margin-top: 6px;
  color: var(--text-muted);
  font-size: 10px;
  line-height: 1.5;
}
.indexing-demo__job {
  display: grid;
  grid-template-columns: 38px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  padding: 13px;
  border: 1px solid color-mix(in srgb, var(--primary) 24%, var(--border));
  border-radius: 14px;
  background: color-mix(in srgb, var(--primary-soft) 48%, var(--surface));
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.14),
    0 15px 28px -26px rgb(15 23 42 / 0.5);
}
.indexing-demo__job > span {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 11px;
  background: var(--surface);
  color: var(--primary-text);
}
.indexing-demo__job div {
  min-width: 0;
}
.indexing-demo__job strong,
.indexing-demo__job small {
  display: block;
}
.indexing-demo__job strong {
  font-size: 11px;
}
.indexing-demo__job small {
  margin-top: 4px;
  color: var(--text-muted);
  font-size: 9px;
}
.indexing-demo__job b {
  color: var(--primary-text);
  font-size: 10px;
}
.progress-track {
  height: 4px;
  margin-top: 9px;
  overflow: hidden;
  border-radius: 99px;
  background: var(--border);
}
.progress-track i {
  display: block;
  width: 55%;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--primary), #93c5fd);
  animation: progress-pulse 1.7s ease-in-out infinite;
}
@keyframes progress-pulse {
  50% {
    filter: brightness(1.25);
  }
}
@media (max-width: 760px) {
  .search-demo__grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .mini-app {
    grid-template-columns: 120px minmax(0, 1fr);
  }
  .spotlight-demo__actions {
    display: none;
  }
}
@media (prefers-reduced-motion: reduce) {
  .progress-track i {
    animation-duration: 0.01ms;
  }
}
</style>
