<script setup lang="ts">
import {
  Check,
  Copy,
  Ellipsis,
  ExternalLink,
  Folder,
  FolderOpen,
  FolderPlus,
  Images,
  Info,
  Keyboard,
  Moon,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Search,
  SlidersHorizontal,
  Sun,
  WandSparkles,
} from "@lucide/vue";
import { computed, ref } from "vue";
import type {
  IndexProgress,
  ModelDownloadProgress,
  RuntimeStats,
} from "../../types";
import type { ThemeMode } from "../../stores/theme";
import tribalPortrait from "../../assets/onboarding_assets/99003f6e5f05348d1852c38ed196d988.jpg";
import womanGreen from "../../assets/onboarding_assets/girl_train_segmented.png";
import womanPortrait from "../../assets/onboarding_assets/karina armageddon.jpg";
import greenFashion from "../../assets/onboarding_assets/téléchargement (2).jpg";
import imagyxLogo from "../../../src-tauri/icons/imagyx-bigger.avif";
import ShortcutView from "../shortcuts/ShortcutView.vue";
import UnifiedSearchInput from "../UnifiedSearchInput.vue";
import SpotlightInput from "../Spotlight/SpotlightInput.vue";
import SpotlightSettings from "../Spotlight/SpotlightSettings.vue";
import Button from "../ui/Button/Button.vue";
import CopyButton from "../ui/Button/CopyButton.vue";
import ButtonGroup from "../ui/ButtonGroup/ButtonGroup.vue";
import KbdChip from "../ui/KbdChip/KbdChip.vue";
import MovingBorder from "../ui/MovingBorder/MovingBorder.vue";
import LocalAiStatus from "../LocalAiStatus.vue";
import { useTranslate } from "../../i18n";
import {
  isModelDownloading,
  isModelPreparing,
} from "../../utils/model-readiness";

const { t } = useTranslate();

const emit = defineEmits<{
  addFolder: [];
  themeChange: [value: ThemeMode];
  pauseIndexing: [];
  resumeIndexing: [];
  openLibrary: [];
}>();

const props = defineProps<{
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
  totalImages: number;
  progress: IndexProgress | null;
  modelProgress: ModelDownloadProgress | null;
  runtimeStats: RuntimeStats | null;
}>();

const isFolderIndexing = computed(() =>
  Boolean(
    props.progress && !["complete", "error"].includes(props.progress.stage),
  ),
);
const modelNeedsPreparation = computed(
  () =>
    isModelDownloading(props.modelProgress) ||
    isModelPreparing(props.modelProgress, props.runtimeStats),
);

const spotlightActionsOpen = ref(false);

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
        <MovingBorder border-radius="13px" size="sm" :duration="4200">
          <UnifiedSearchInput
            variant="app"
            model-value="woman"
            view="search"
            :placeholder="t('search.placeholder')"
            :searching="false"
            :result-label="t('search.result_count', { count: 4 })"
          />
        </MovingBorder>
        <Button class="search-demo__similar" variant="secondary" size="sm">
          <template #leading><WandSparkles :size="14" /></template>
          {{ t("search.visual.find_similar") }}
        </Button>
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
          <span class="demo-card-copy">
            <strong>{{ image.label }}</strong>
            <small>{{ image.fileName }}</small>
          </span>
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
        <span>{{ t("settings.shortcut_label") }}</span>
        <KbdChip :shortcut="shortcut" size="lg" variant="primary" />
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
              <span class="spotlight-demo__thumb">
                <img class="demo-image" :src="image.src" :alt="image.label" />
              </span>
              <div class="spotlight-demo__copy">
                <strong>{{ image.fileName }}</strong>
                <small>1600 × 1100 · 420 KB</small>
              </div>
              <span class="spotlight-demo__score">{{ 92 - index * 6 }}%</span>
            </article>
          </div>
          <div class="spotlight-demo__action-dock" role="toolbar">
            <div class="spotlight-demo__action-bar">
              <CopyButton variant="ghost" size="md">
                <template #trailing
                  ><KbdChip shortcut="Ctrl+C" size="sm"
                /></template>
              </CopyButton>

              <div class="spotlight-demo__action-hub">
                <Transition name="action-popover">
                  <div
                    v-if="spotlightActionsOpen"
                    class="spotlight-demo__action-popover"
                    role="menu"
                  >
                    <Button
                      class="spotlight-demo__action-menu-button spotlight-demo__action-menu-button--featured"
                      variant="ghost"
                      size="md"
                      :depth="false"
                      role="menuitem"
                      @click="spotlightActionsOpen = false"
                    >
                      <template #leading>
                        <span class="spotlight-demo__action-popover__preview">
                          <img
                            class="spotlight-demo__action-popover__preview-image"
                            :src="demoImages[1].src"
                            :alt="demoImages[1].label"
                          />
                        </span>
                      </template>
                      <span class="spotlight-demo__action-popover__copy">
                        <strong>{{ t("search.visual.find_similar") }}</strong>
                        <small>{{
                          t("search.visual.find_similar_desc")
                        }}</small>
                      </span>
                      <template #trailing
                        ><KbdChip shortcut="Ctrl+Shift+S" size="sm"
                      /></template>
                    </Button>

                    <Button
                      class="spotlight-demo__action-menu-button"
                      variant="ghost"
                      size="md"
                      :depth="false"
                      role="menuitem"
                      @click="spotlightActionsOpen = false"
                    >
                      <template #leading><RefreshCw :size="16" /></template>
                      {{ t("spotlight.convert.action") }}
                      <template #trailing
                        ><KbdChip shortcut="Ctrl+Shift+C" size="sm"
                      /></template>
                    </Button>

                    <Button
                      class="spotlight-demo__action-menu-button"
                      variant="ghost"
                      size="md"
                      :depth="false"
                      role="menuitem"
                      @click="spotlightActionsOpen = false"
                    >
                      <template #leading><FolderOpen :size="16" /></template>
                      {{ t("open_in_file_manager", { name: "Explorer" }) }}
                      <template #trailing
                        ><KbdChip shortcut="Ctrl+E" size="sm"
                      /></template>
                    </Button>

                    <Button
                      class="spotlight-demo__action-menu-button"
                      variant="ghost"
                      size="md"
                      :depth="false"
                      role="menuitem"
                      @click="spotlightActionsOpen = false"
                    >
                      <template #leading><ExternalLink :size="16" /></template>
                      {{ t("open_in_imagyx") }}
                      <template #trailing
                        ><KbdChip shortcut="Ctrl+I" size="sm"
                      /></template>
                    </Button>
                  </div>
                </Transition>

                <Button
                  variant="ghost"
                  size="md"
                  :depth="false"
                  :pressed="spotlightActionsOpen"
                  @click="spotlightActionsOpen = !spotlightActionsOpen"
                >
                  <template #leading><Ellipsis :size="17" /></template>
                  {{ t("spotlight.actions.more") }}
                  <template #trailing
                    ><KbdChip shortcut="Ctrl+K" size="sm"
                  /></template>
                </Button>
              </div>
            </div>
          </div>
        </div>
      </MovingBorder>
    </div>

    <div v-else-if="kind === 'settings'" class="settings-demo">
      <SpotlightSettings
        variant="embedded"
        :shortcut="shortcut"
        :theme-mode="themeMode"
        @shortcut-change="emit('shortcutChange', $event)"
        @theme-change="emit('themeChange', $event)"
      />
      <div class="settings-demo__app-notice">
        <SlidersHorizontal :size="15" />
        <span>{{ t("settings.app_settings_notice") }}</span>
      </div>
    </div>

    <div v-else class="indexing-demo">
      <template v-if="!hasFolders">
        <div class="indexing-demo__empty-wrapper">
          <LocalAiStatus
            v-if="modelNeedsPreparation"
            class="indexing-demo__model-status"
            :progress="progress"
            :model-progress="modelProgress"
            :runtime-stats="runtimeStats"
            @pause="emit('pauseIndexing')"
            @resume="emit('resumeIndexing')"
          />

          <div class="indexing-demo__setup-card">
            <span class="indexing-demo__icon">
              <FolderPlus :size="28" />
            </span>
            <div class="indexing-demo__setup-copy">
              <strong>{{ t("onboarding.first_folder_title") }}</strong>
              <p>{{ t("onboarding.step_indexing_desc") }}</p>
            </div>
            <Button
              class="indexing-demo__add-folder"
              variant="primary"
              size="lg"
              @click="emit('addFolder')"
            >
              <template #leading><FolderPlus :size="18" /></template>
              {{ t("onboarding.add_folder") }}
            </Button>
            <small class="indexing-demo__privacy">{{
              t("spotlight.no_folder_privacy")
            }}</small>
          </div>
        </div>
      </template>
      <template v-else>
        <div v-if="isFolderIndexing" class="indexing-demo__folder">
          <div class="folder-row-demo">
            <Button class="preview-row-button" variant="ghost" size="md" block>
              <template #leading><Folder :size="16" /></template>
              <span class="row-copy">Visual references</span>
            </Button>
          </div>
          <LocalAiStatus
            :progress="progress"
            :model-progress="modelProgress"
            :runtime-stats="runtimeStats"
            @pause="emit('pauseIndexing')"
            @resume="emit('resumeIndexing')"
          />
        </div>
        <div v-else class="indexing-demo__waiting">
          <FolderOpen :size="28" />
          <strong>{{
            t("onboarding.indexing_complete_title", { count: totalImages })
          }}</strong>
          <span>{{ t("onboarding.indexing_complete_desc") }}</span>
          <div class="indexing-demo__complete-actions">
            <Button variant="secondary" size="md" @click="emit('openLibrary')">
              {{ t("onboarding.open_library") }}
            </Button>
            <Button variant="ghost" size="md" @click="emit('addFolder')">
              <template #leading><FolderPlus :size="16" /></template>
              {{ t("onboarding.add_another_folder") }}
            </Button>
          </div>
        </div>
      </template>
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
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 88px;
  margin-bottom: 14px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 600;
}
.spotlight-demo__surface {
  position: relative;
  overflow: hidden;
  border-radius: 19px;
  background: color-mix(in srgb, var(--surface-elevated) 96%, transparent);
}
.spotlight-demo__results {
  padding: 5px 8px 52px;
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
  border-color: color-mix(in srgb, var(--primary) 28%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 70%, var(--surface));
  box-shadow: inset 0 1px rgb(255 255 255 / 0.05);
  transform: translate3d(2px, 0, 0) scale(0.998);
}
.spotlight-demo__thumb {
  width: 44px;
  height: 44px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface-hover);
  box-shadow: 0 4px 12px rgb(2 6 23 / 0.12);
}
.spotlight-demo__results article.active .spotlight-demo__thumb {
  transform: scale(1.035) rotate(-0.35deg);
}
.spotlight-demo__thumb .demo-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.spotlight-demo__copy {
  min-width: 0;
}
.spotlight-demo__copy strong,
.spotlight-demo__copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.spotlight-demo__copy strong {
  font-size: 11px;
  letter-spacing: -0.12px;
}
.spotlight-demo__copy small {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 9px;
}
.spotlight-demo__score {
  padding: 3px 6px;
  border-radius: var(--radius-full);
  background: var(--primary-soft);
  color: var(--primary-text);
  font-size: 9px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.spotlight-demo__action-dock {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 2;
  padding: 6px 10px 7px;
  border-top: 1px solid
    color-mix(in srgb, var(--border-strong) 34%, transparent);
  background: color-mix(in srgb, var(--surface-elevated) 82%, transparent);
  box-shadow:
    0 -8px 22px -17px rgb(2 6 23 / 0.42),
    inset 0 1px rgb(255 255 255 / 0.055);
  backdrop-filter: blur(18px) saturate(1.1);
}
.spotlight-demo__action-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  width: 100%;
}
.spotlight-demo__action-hub {
  position: relative;
  display: flex;
}
.spotlight-demo__action-popover {
  position: absolute;
  right: 0;
  bottom: calc(100% + 10px);
  width: 290px;
  padding: 6px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--border-strong) 82%, transparent);
  border-radius: 15px;
  background: color-mix(in srgb, var(--surface-elevated) 94%, transparent);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.08),
    0 22px 48px -24px rgb(2 6 23 / 0.72);
  backdrop-filter: blur(28px) saturate(1.18);
  transform-origin: calc(100% - 34px) 100%;
}
.spotlight-demo__action-menu-button {
  width: 100%;
  min-height: 40px;
  justify-content: flex-start;
  padding-inline: 8px;
  text-align: left;
}
.spotlight-demo__action-menu-button + .spotlight-demo__action-menu-button {
  margin-top: 2px;
}
.spotlight-demo__action-menu-button--featured {
  min-height: 46px;
  margin-bottom: 4px;
}
.spotlight-demo__action-menu-button :deep(.ui-button__content) {
  flex: 1;
  justify-content: flex-start;
  min-width: 0;
  text-align: left;
}
.spotlight-demo__action-menu-button :deep(.ui-button__icon:last-child) {
  margin-left: auto;
}
.spotlight-demo__action-popover__copy {
  display: block;
  min-width: 0;
}
.spotlight-demo__action-popover__copy strong,
.spotlight-demo__action-popover__copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.spotlight-demo__action-popover__copy strong {
  font-size: 11px;
  font-weight: 650;
}
.spotlight-demo__action-popover__copy small {
  margin-top: 2px;
  color: var(--text-muted);
  font-size: 9px;
}
.spotlight-demo__action-popover__preview {
  position: relative;
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-hover);
}
.spotlight-demo__action-popover__preview-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.action-popover-enter-active,
.action-popover-leave-active {
  transition:
    opacity 120ms ease,
    transform 170ms cubic-bezier(0.16, 1, 0.3, 1),
    filter 120ms ease;
}
.action-popover-enter-from,
.action-popover-leave-to {
  opacity: 0;
  transform: scale(0.95) translateY(6px);
  filter: blur(4px);
}
.settings-demo {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 16px;
  width: min(520px, 100%);
  height: 100%;
  margin: 0 auto;
  padding: 24px;
  box-sizing: border-box;
}
.settings-demo :deep(.spotlight-settings) {
  padding: 0;
  overflow: visible;
}
.settings-demo__app-notice {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border: 1px dashed color-mix(in srgb, var(--border-strong) 60%, transparent);
  border-radius: 14px;
  background: color-mix(in srgb, var(--surface) 60%, transparent);
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.45;
}
.settings-demo__app-notice svg {
  flex-shrink: 0;
  color: var(--primary-text);
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
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 20px;
  box-sizing: border-box;
}
.indexing-demo__empty-wrapper {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  width: 100%;
  max-width: 440px;
}
.indexing-demo__setup-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 100%;
  padding: 28px 24px;
  border: 1px solid color-mix(in srgb, var(--border) 82%, transparent);
  border-radius: 20px;
  background: color-mix(in srgb, var(--surface) 92%, transparent);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.08),
    0 20px 40px -28px rgb(15 23 42 / 0.4);
  text-align: center;
}
.indexing-demo__icon {
  display: grid;
  place-items: center;
  width: 52px;
  height: 52px;
  margin-bottom: 14px;
  border: 1px solid color-mix(in srgb, var(--primary) 30%, var(--border));
  border-radius: 16px;
  background: color-mix(in srgb, var(--primary-soft) 72%, var(--surface));
  color: var(--primary-text);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.1),
    0 12px 24px -20px rgb(15 23 42 / 0.45);
}
.indexing-demo__setup-copy strong {
  color: var(--text);
  font-size: 16px;
  letter-spacing: -0.2px;
}
.indexing-demo__setup-copy p {
  max-width: 320px;
  margin: 8px 0 18px;
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.5;
}
.indexing-demo__privacy {
  margin-top: 13px;
  color: var(--text-subtle);
  font-size: 9px;
}
.indexing-demo__folder {
  display: grid;
  align-content: center;
  gap: 14px;
  width: min(380px, 100%);
  margin: auto;
}
.indexing-demo__waiting {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--text-muted);
  text-align: center;
}
.indexing-demo__waiting strong {
  color: var(--text);
  font-size: 15px;
}
.indexing-demo__waiting span {
  max-width: 320px;
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.5;
  text-align: center;
}
.indexing-demo__model-status {
  width: 100%;
}
.indexing-demo__complete-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  margin-top: 6px;
}
.indexing-demo__add-folder {
  min-width: min(240px, 100%);
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
