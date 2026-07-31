<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import {
  Check,
  Copy,
  Ellipsis,
  ExternalLink,
  FolderOpen,
  FolderPlus,
  LoaderCircle,
  RefreshCw,
  Search,
} from "@lucide/vue";
import type {
  FolderIndexCoverage,
  ImageAsset,
  ModelDownloadProgress,
  RuntimeStats,
} from "../../types";
import { visualSearch } from "../../services/visual-search";
import ThumbnailImage from "../ThumbnailImage.vue";
import Button from "../ui/Button/Button.vue";
import KbdChip from "../ui/KbdChip/KbdChip.vue";
import SpotlightConverter from "./SpotlightConverter.vue";
import SpotlightIndexProgress from "./SpotlightIndexProgress.vue";
import SpotlightIndexCoverageNotice from "./SpotlightIndexCoverageNotice.vue";
import SpotlightModelDownload from "./SpotlightModelDownload.vue";
import { isModelPreparing } from "../../utils/model-readiness";
import type { SpotlightIndexJob } from "./types";
import { useTranslate } from "../../i18n";

const props = defineProps<{
  results: ImageAsset[];
  selectedIndex: number;
  searching: boolean;
  semanticSearching: boolean;
  hasSearchQuery: boolean;
  error: string | null;
  copiedImageId: string | null;
  copyingImageId: string | null;
  revealingImageId: string | null;
  openingImageId: string | null;
  showAddAction: boolean;
  hasFolders: boolean;
  libraryReady: boolean;
  showBackgroundHint: boolean;
  incompleteCoverage: FolderIndexCoverage[];
  jobs: SpotlightIndexJob[];
  fileManagerName: string;
  performanceMode: boolean;
  modelProgress: ModelDownloadProgress | null;
  runtimeStats: RuntimeStats | null;
}>();

const emit = defineEmits<{
  select: [index: number];
  scrollState: [scrolling: boolean];
  open: [image: ImageAsset];
  copy: [image: ImageAsset];
  reveal: [image: ImageAsset];
  addFolder: [];
  resume: [folderIds: string[]];
}>();

const { t } = useTranslate();
const RESULT_ROW_HEIGHT = 72;
const RESULT_OVERSCAN = 4;
const viewport = ref<HTMLElement | null>(null);
const resultList = ref<HTMLElement | null>(null);
const actionHub = ref<HTMLElement | null>(null);
const converterView = ref<InstanceType<typeof SpotlightConverter> | null>(null);
const converterSource = ref<ImageAsset | null>(null);
const actionsOpen = ref(false);
const findingSimilar = ref(false);
const canScrollDown = ref(false);
const scrollTop = ref(0);
const viewportHeight = ref(0);
const resultsOffset = ref(0);
let scrollFrame: number | undefined;
let scrollEndTimer: number | undefined;
let resizeObserver: ResizeObserver | null = null;
const isScrolling = ref(false);

const selectedImage = computed(
  () => props.results[props.selectedIndex] ?? null,
);
const modelPreparing = computed(() =>
  isModelPreparing(props.modelProgress, props.runtimeStats),
);
const visibleResults = computed(() => {
  const firstVisible = Math.floor(
    Math.max(0, scrollTop.value - resultsOffset.value) / RESULT_ROW_HEIGHT,
  );
  const visibleCount = Math.ceil(viewportHeight.value / RESULT_ROW_HEIGHT);
  const start = Math.max(0, firstVisible - RESULT_OVERSCAN);
  const end = Math.min(
    props.results.length,
    firstVisible + visibleCount + RESULT_OVERSCAN,
  );
  return props.results.slice(start, end).map((image, offset) => ({
    image,
    index: start + offset,
  }));
});

watch(
  () => [
    props.results.length,
    props.jobs.length,
    props.showAddAction,
    props.hasFolders,
    props.libraryReady,
    props.showBackgroundHint,
    props.incompleteCoverage.length,
    props.searching,
    props.hasSearchQuery,
  ],
  () => {
    void nextTick(scheduleScrollState);
  },
);

watch(
  () => selectedImage.value?.id,
  () => {
    actionsOpen.value = false;
  },
);

function updateScrollState() {
  if (converterSource.value) {
    canScrollDown.value = false;
    return;
  }
  const element = viewport.value;
  if (!element) {
    canScrollDown.value = false;
    return;
  }
  scrollTop.value = element.scrollTop;
  viewportHeight.value = element.clientHeight;
  resultsOffset.value = resultList.value?.offsetTop ?? 0;
  canScrollDown.value =
    element.scrollHeight > element.clientHeight + 2 &&
    element.scrollTop + element.clientHeight < element.scrollHeight - 2;
}

function scheduleScrollState() {
  if (scrollFrame) return;
  scrollFrame = window.requestAnimationFrame(() => {
    scrollFrame = undefined;
    updateScrollState();
  });
}

function handleScroll() {
  if (!isScrolling.value) {
    isScrolling.value = true;
    emit("scrollState", true);
  }
  if (scrollEndTimer) window.clearTimeout(scrollEndTimer);
  scrollEndTimer = window.setTimeout(() => {
    isScrolling.value = false;
    emit("scrollState", false);
  }, 120);
  scheduleScrollState();
}

function handleResultPointerEnter(index: number) {
  // Virtual rows are replaced while scrolling. Selecting each replacement forces
  // a parent render and makes fast wheel scrolling feel sticky.
  if (!isScrolling.value) emit("select", index);
}

function scrollToIndex(index: number) {
  if (converterSource.value) return;
  void nextTick(() => {
    const element = viewport.value;
    const list = resultList.value;
    if (!element || !list) return;
    const rowTop = list.offsetTop + index * RESULT_ROW_HEIGHT;
    const rowBottom = rowTop + RESULT_ROW_HEIGHT;
    if (rowTop < element.scrollTop) element.scrollTop = rowTop;
    else if (rowBottom > element.scrollTop + element.clientHeight) {
      element.scrollTop = rowBottom - element.clientHeight;
    }
    scheduleScrollState();
  });
}

function openConverter(image: ImageAsset) {
  actionsOpen.value = false;
  converterSource.value = image;
  canScrollDown.value = false;
}

function closeConverter() {
  converterSource.value = null;
  void nextTick(scheduleScrollState);
}

function promoteConvertedResult(image: ImageAsset) {
  const existingIndex = props.results.findIndex(
    (result) => result.id === image.id,
  );
  if (existingIndex >= 0) props.results.splice(existingIndex, 1);
  props.results.unshift(image);
  emit("select", 0);
}

function toggleActions() {
  if (!selectedImage.value) return;
  actionsOpen.value = !actionsOpen.value;
}

function copySelected() {
  if (!selectedImage.value) return;
  emit("copy", selectedImage.value);
}

async function findSimilarSelected() {
  const image = selectedImage.value;
  if (!image || findingSimilar.value) return;
  actionsOpen.value = false;
  findingSimilar.value = true;
  try {
    await visualSearch.findSimilar(image);
  } finally {
    findingSimilar.value = false;
  }
}

function convertSelected() {
  if (!selectedImage.value) return;
  openConverter(selectedImage.value);
}

function revealSelected() {
  if (!selectedImage.value) return;
  actionsOpen.value = false;
  emit("reveal", selectedImage.value);
}

function openSelected() {
  if (!selectedImage.value) return;
  actionsOpen.value = false;
  emit("open", selectedImage.value);
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (!actionsOpen.value) return;
  const target = event.target;
  if (target instanceof Node && actionHub.value?.contains(target)) return;
  actionsOpen.value = false;
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (converterSource.value) {
    const handled = converterView.value?.handleKeydown(event) ?? false;
    if (handled) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
    return;
  }

  if (actionsOpen.value && event.key === "Escape") {
    event.preventDefault();
    event.stopImmediatePropagation();
    actionsOpen.value = false;
    return;
  }

  const primaryModifier = event.ctrlKey || event.metaKey;
  const key = event.key.toLocaleLowerCase();
  if (primaryModifier && !event.shiftKey && key === "k") {
    event.preventDefault();
    event.stopImmediatePropagation();
    toggleActions();
    return;
  }

  if (!primaryModifier || !event.shiftKey || event.altKey) return;
  const image = selectedImage.value;
  if (!image) return;

  if (key === "s" || event.code === "KeyS") {
    event.preventDefault();
    event.stopImmediatePropagation();
    void findSimilarSelected();
    return;
  }

  if (key !== "c" && event.code !== "KeyC") return;
  event.preventDefault();
  event.stopImmediatePropagation();
  openConverter(image);
}

onMounted(() => {
  resizeObserver = new ResizeObserver(scheduleScrollState);
  if (viewport.value) resizeObserver.observe(viewport.value);
  window.addEventListener("keydown", handleWindowKeydown, { capture: true });
  document.addEventListener("pointerdown", handleDocumentPointerDown, {
    capture: true,
  });
  scheduleScrollState();
});

onBeforeUnmount(() => {
  if (scrollFrame) window.cancelAnimationFrame(scrollFrame);
  if (scrollEndTimer) window.clearTimeout(scrollEndTimer);
  if (isScrolling.value) emit("scrollState", false);
  resizeObserver?.disconnect();
  window.removeEventListener("keydown", handleWindowKeydown, { capture: true });
  document.removeEventListener("pointerdown", handleDocumentPointerDown, {
    capture: true,
  });
});

defineExpose({ scrollToIndex });
</script>

<template>
  <div
    class="spotlight-results-shell"
    :class="{ 'spotlight-results-shell--scrolling': performanceMode }"
  >
    <SpotlightConverter
      v-if="converterSource"
      ref="converterView"
      :source="converterSource"
      :copied-image-id="copiedImageId"
      :copying-image-id="copyingImageId"
      :revealing-image-id="revealingImageId"
      :opening-image-id="openingImageId"
      :file-manager-name="fileManagerName"
      @back="closeConverter"
      @converted="promoteConvertedResult"
      @copy="emit('copy', $event)"
      @reveal="emit('reveal', $event)"
      @open="emit('open', $event)"
    />

    <div
      v-else
      ref="viewport"
      class="spotlight-results"
      :class="{ 'spotlight-results--with-actions': Boolean(selectedImage) }"
      role="listbox"
      @scroll.passive="handleScroll"
    >
      <div
        v-if="!libraryReady"
        class="spotlight-library-loading"
        aria-live="polite"
      >
        <LoaderCircle class="spin" :size="22" />
        <strong>{{ t("spotlight.loading_library") }}</strong>
      </div>

      <section
        v-else-if="!hasFolders"
        class="spotlight-library-empty"
        aria-labelledby="spotlight-library-empty-title"
      >
        <span class="spotlight-library-empty__icon"
          ><FolderPlus :size="24"
        /></span>
        <strong id="spotlight-library-empty-title">{{
          t("spotlight.no_folder_title")
        }}</strong>
        <p>{{ t("spotlight.no_folder_desc") }}</p>
        <Button
          class="spotlight-library-empty__button"
          variant="primary"
          size="lg"
          @click="emit('addFolder')"
        >
          <template #leading><FolderPlus :size="17" /></template>
          {{ t("spotlight.no_folder_action") }}
        </Button>
        <small>{{ t("spotlight.no_folder_privacy") }}</small>
      </section>

      <SpotlightModelDownload
        v-else-if="modelPreparing && modelProgress"
        :progress="modelProgress"
        :runtime-stats="runtimeStats"
      />

      <template v-else>
        <SpotlightIndexCoverageNotice
          v-if="incompleteCoverage.length"
          :coverage="incompleteCoverage"
          @resume="emit('resume', $event)"
        />
        <Button
          v-if="showAddAction"
          class="spotlight-add-folder"
          variant="secondary"
          size="lg"
          block
          @click="emit('addFolder')"
        >
          <template #leading><FolderPlus :size="18" /></template>
          <span class="spotlight-add-folder__copy">
            <strong>{{ t("spotlight.add_folder_title") }}</strong>
            <small>{{ t("spotlight.add_folder_desc") }}</small>
          </span>
        </Button>

        <aside
          v-if="showBackgroundHint"
          class="spotlight-background-hint"
          aria-live="polite"
        >
          <span class="spotlight-background-hint__icon"
            ><LoaderCircle class="spin" :size="16"
          /></span>
          <span>
            <strong>{{ t("spotlight.indexing_results_title") }}</strong>
            <small>{{ t("spotlight.indexing_results_desc") }}</small>
          </span>
        </aside>

        <SpotlightIndexProgress
          v-for="job in jobs"
          :key="job.folderId"
          :job="job"
        />

        <div
          v-if="results.length"
          ref="resultList"
          class="spotlight-result-list"
          :style="{ height: `${results.length * RESULT_ROW_HEIGHT}px` }"
        >
          <div
            class="spotlight-result-list__items"
            :style="{
              transform: `translate3d(0, ${(visibleResults[0]?.index ?? 0) * RESULT_ROW_HEIGHT}px, 0)`,
            }"
          >
            <div
              v-for="{ image, index } in visibleResults"
              :key="image.id"
              class="spotlight-result"
              :class="{ 'spotlight-result--selected': index === selectedIndex }"
              :data-result-index="index"
              :aria-posinset="index + 1"
              :aria-setsize="results.length"
              :aria-selected="index === selectedIndex"
              role="option"
              tabindex="-1"
              @mouseenter="handleResultPointerEnter(index)"
              @focus="emit('select', index)"
              @click="emit('select', index)"
              @dblclick="emit('open', image)"
            >
              <span class="spotlight-thumb">
                <ThumbnailImage
                  class="spotlight-thumbnail-image"
                  :image="image"
                  :priority="isScrolling ? 3 : index === selectedIndex ? 0 : 1"
                />
              </span>
              <span class="spotlight-copy">
                <strong>{{ image.name }}</strong>
                <small
                  >{{ image.width }} × {{ image.height }} ·
                  {{ Math.max(1, Math.round(image.sizeBytes / 1024)) }}
                  KB</small
                >
              </span>
              <span v-if="image.semanticScore != null" class="spotlight-score"
                >{{ Math.round(image.semanticScore * 100) }}%</span
              >
            </div>
          </div>
        </div>



        <div
          v-else-if="
            hasSearchQuery &&
            !searching &&
            !semanticSearching &&
            !results.length &&
            !error &&
            !showAddAction &&
            !showBackgroundHint &&
            jobs.length === 0
          "
          class="spotlight-empty"
        >
          <Search :size="24" />
          <strong>{{ t("spotlight.no_results_title") }}</strong>
          <span>{{ t("spotlight.no_results_desc") }}</span>
        </div>
      </template>

      <div v-if="error" class="spotlight-error">{{ error }}</div>
    </div>

    <div
      v-if="!converterSource && selectedImage"
      class="spotlight-action-dock"
      role="toolbar"
      :aria-label="t('spotlight.actions.toolbar')"
    >
      <div class="spotlight-action-dock__bar">
        <Button
          class="spotlight-dock-button spotlight-dock-button--copy"
          :class="{
            'spotlight-dock-button--success':
              copiedImageId === selectedImage.id,
          }"
          :variant="copiedImageId === selectedImage.id ? 'primary' : 'ghost'"
          size="md"
          :depth="false"
          :loading="copyingImageId === selectedImage.id"
          :aria-label="t('copy_image')"
          @click="copySelected"
        >
          <template #leading>
            <Check v-if="copiedImageId === selectedImage.id" :size="15" />
            <Copy v-else :size="15" />
          </template>
          {{ copiedImageId === selectedImage.id ? t("copied") : t("copy") }}
          <template #trailing
            ><KbdChip shortcut="Ctrl+C" size="sm" variant="primary"
          /></template>
        </Button>

        <div ref="actionHub" class="spotlight-action-hub">
          <Transition name="action-popover">
            <div
              v-if="actionsOpen"
              class="spotlight-action-popover"
              role="menu"
              :aria-label="t('spotlight.actions.more')"
            >
              <Button
                class="spotlight-action-menu-button spotlight-action-menu-button--featured"
                variant="ghost"
                size="md"
                :depth="false"
                :disabled="findingSimilar"
                role="menuitem"
                @click="findSimilarSelected"
              >
                <template #leading>
                  <span class="spotlight-action-popover__preview">
                    <ThumbnailImage
                      class="spotlight-action-popover__preview-image"
                      :image="selectedImage"
                      :priority="0"
                    />
                    <span
                      v-if="findingSimilar"
                      class="spotlight-action-popover__preview-loading"
                    >
                      <LoaderCircle class="spin" :size="15" />
                    </span>
                  </span>
                </template>
                <span class="spotlight-action-popover__copy">
                  <strong><span class="spotlight-action-marquee-inner">{{ t("search.visual.find_similar") }}</span></strong>
                  <small><span class="spotlight-action-marquee-inner">{{ t("search.visual.find_similar_desc") }}</span></small>
                </span>
                <template #trailing
                  ><KbdChip shortcut="Ctrl+Shift+S" size="sm" variant="primary"
                /></template>
              </Button>

              <Button
                class="spotlight-action-menu-button"
                variant="ghost"
                size="md"
                :depth="false"
                role="menuitem"
                @click="convertSelected"
              >
                <template #leading><RefreshCw :size="16" /></template>
                {{ t("spotlight.convert.action") }}
                <template #trailing
                  ><KbdChip shortcut="Ctrl+Shift+C" size="sm" variant="primary"
                /></template>
              </Button>

              <Button
                class="spotlight-action-menu-button"
                variant="ghost"
                size="md"
                :depth="false"
                role="menuitem"
                @click="revealSelected"
              >
                <template #leading><FolderOpen :size="16" /></template>
                {{ t("open_in_file_manager", { name: fileManagerName }) }}
                <template #trailing
                  ><KbdChip shortcut="Ctrl+E" size="sm" variant="primary"
                /></template>
              </Button>

              <Button
                class="spotlight-action-menu-button"
                variant="ghost"
                size="md"
                :depth="false"
                role="menuitem"
                @click="openSelected"
              >
                <template #leading><ExternalLink :size="16" /></template>
                {{ t("open_in_imagyx") }}
                <template #trailing
                  ><KbdChip shortcut="Ctrl+I" size="sm" variant="primary"
                /></template>
              </Button>
            </div>
          </Transition>

          <Button
            class="spotlight-dock-button spotlight-dock-button--more"
            variant="ghost"
            size="md"
            :depth="false"
            :pressed="actionsOpen"
            :aria-label="t('spotlight.actions.more')"
            :aria-expanded="actionsOpen"
            aria-haspopup="menu"
            @click="toggleActions"
          >
            <template #leading><Ellipsis :size="17" /></template>
            {{ t("spotlight.actions.more") }}
            <template #trailing
              ><KbdChip shortcut="Ctrl+K" size="sm" variant="primary"
            /></template>
          </Button>
        </div>
      </div>
    </div>

    <div
      v-if="canScrollDown && !selectedImage"
      class="spotlight-scroll-shadow"
      aria-hidden="true"
    />
  </div>
</template>

<style scoped>
.spotlight-results-shell {
  position: relative;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}
.spotlight-results-shell--scrolling .spotlight-action-dock,
.spotlight-results-shell--scrolling .spotlight-action-popover {
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
.spotlight-results-shell--scrolling .spotlight-result,
.spotlight-results-shell--scrolling .spotlight-thumb {
  transition: none;
}
.spotlight-results {
  height: 100%;
  overflow-y: auto;
  padding: 8px 9px 20px;
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, var(--border-strong) 78%, transparent)
    transparent;
}
.spotlight-results--with-actions {
  padding-bottom: 58px;
}
.spotlight-library-loading {
  display: grid;
  place-items: center;
  align-content: center;
  gap: 12px;
  min-height: 300px;
  color: var(--text-muted);
  font-size: 12px;
}
.spotlight-library-empty {
  display: grid;
  place-items: center;
  align-content: center;
  min-height: 370px;
  padding: 34px;
  text-align: center;
}
.spotlight-library-empty__icon {
  display: grid;
  place-items: center;
  width: 54px;
  height: 54px;
  margin-bottom: 15px;
  border: 1px solid color-mix(in srgb, var(--primary) 30%, var(--border));
  border-radius: 16px;
  background: color-mix(in srgb, var(--primary-soft) 72%, var(--surface));
  color: var(--primary-text);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.1),
    0 14px 28px -24px rgb(15 23 42 / 0.5);
}
.spotlight-library-empty strong {
  color: var(--text);
  font-size: 16px;
  letter-spacing: -0.2px;
}
.spotlight-library-empty p {
  max-width: 330px;
  margin: 8px 0 18px;
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.55;
}
.spotlight-library-empty small {
  margin-top: 13px;
  color: var(--text-subtle);
  font-size: 9px;
}
.spotlight-library-empty__button {
  min-width: 178px;
}
.spotlight-add-folder {
  justify-content: flex-start;
  margin: 4px 4px 8px;
  min-height: 64px;
  text-align: left;
}
.spotlight-add-folder__copy {
  display: block;
  min-width: 0;
  text-align: left;
}
.spotlight-add-folder__copy strong,
.spotlight-add-folder__copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.spotlight-add-folder__copy strong {
  font-size: 11px;
  color: var(--text);
}
.spotlight-add-folder__copy small {
  margin-top: 3px;
  color: var(--text-secondary);
  font-size: 10px;
  font-weight: 500;
}
.spotlight-background-hint {
  display: grid;
  grid-template-columns: 30px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  margin: 4px 4px 8px;
  padding: 9px 11px;
  border: 1px solid color-mix(in srgb, var(--primary) 18%, var(--border));
  border-radius: 12px;
  background: color-mix(in srgb, var(--primary-soft) 30%, var(--surface));
}
.spotlight-background-hint__icon {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: color-mix(in srgb, var(--primary-soft) 68%, var(--surface));
  color: var(--primary-text);
}
.spotlight-background-hint strong,
.spotlight-background-hint small {
  display: block;
}
.spotlight-background-hint strong {
  color: var(--text);
  font-size: 10px;
}
.spotlight-background-hint small {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: 9px;
}
.spotlight-result-list {
  position: relative;
  animation: result-list-reveal 180ms ease both;
}
.spotlight-result-list__items {
  will-change: transform;
}
.spotlight-result {
  position: relative;
  display: grid;
  grid-template-columns: 54px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 72px;
  padding: 8px 11px;
  overflow: hidden;
  border: 1px solid transparent;
  border-radius: 14px;
  background: transparent;
  color: inherit;
  cursor: default;
  transition:
    background-color 150ms ease,
    border-color 150ms ease,
    transform 180ms cubic-bezier(0.16, 1, 0.3, 1),
    box-shadow 180ms ease;
}
.spotlight-result--selected {
  border-color: color-mix(in srgb, var(--primary) 28%, var(--border));
  background: color-mix(in srgb, var(--primary-soft) 70%, var(--surface));
  box-shadow: inset 0 1px rgb(255 255 255 / 0.05);
  transform: translate3d(2px, 0, 0) scale(0.998);
  will-change: transform;
}
.spotlight-thumb {
  width: 54px;
  height: 54px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-hover);
  box-shadow: 0 6px 16px rgb(2 6 23 / 0.12);
  transition: transform 220ms cubic-bezier(0.16, 1, 0.3, 1);
}
.spotlight-result--selected .spotlight-thumb {
  transform: scale(1.035) rotate(-0.35deg);
}
.spotlight-thumbnail-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.spotlight-copy {
  min-width: 0;
}
.spotlight-copy strong,
.spotlight-copy small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.spotlight-copy strong {
  color: var(--text);
  font-size: 13px;
  letter-spacing: -0.12px;
}
.spotlight-copy small {
  margin-top: 5px;
  color: var(--text-muted);
  font-size: 10px;
}
.spotlight-score {
  padding: 4px 7px;
  border-radius: var(--radius-full);
  background: var(--primary-soft);
  color: var(--primary-text);
  font-size: 10px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}
.spotlight-action-dock {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 6;
  padding: 7px 10px 8px;
  pointer-events: none;
  border-top: 1px solid
    color-mix(in srgb, var(--border-strong) 34%, transparent);
  background: color-mix(in srgb, var(--surface-elevated) 82%, transparent);
  box-shadow:
    0 -8px 22px -17px rgb(2 6 23 / 0.42),
    inset 0 1px rgb(255 255 255 / 0.055);
  backdrop-filter: blur(18px) saturate(1.1);
  -webkit-backdrop-filter: blur(18px) saturate(1.1);
}
.spotlight-action-dock__bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  width: 100%;
  min-height: 36px;
  pointer-events: auto;
}
.spotlight-dock-button {
  flex: 0 0 auto;
  font-size: 10px;
}
.spotlight-dock-button--success {
  animation: action-success 280ms cubic-bezier(0.16, 1, 0.3, 1) both;
}
.spotlight-action-hub {
  position: relative;
  display: flex;
}
.spotlight-action-popover {
  position: absolute;
  right: 0;
  bottom: calc(100% + 10px);
  width: 316px;
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
.spotlight-action-menu-button {
  width: 100%;
  min-height: 43px;
  justify-content: flex-start;
  padding-inline: 8px;
  text-align: left;
}
.spotlight-action-menu-button + .spotlight-action-menu-button {
  margin-top: 2px;
}
.spotlight-action-menu-button--featured {
  min-height: 48px;
  margin-bottom: 4px;
}
.spotlight-action-menu-button :deep(.ui-button__content) {
  flex: 1;
  justify-content: flex-start;
  min-width: 0;
  text-align: left;
}
.spotlight-action-menu-button :deep(.ui-button__icon:last-child) {
  margin-left: auto;
}
.spotlight-action-popover__copy {
  position: relative;
  display: block;
  min-width: 0;
  flex: 1;
  overflow: hidden;
  mask-image: linear-gradient(to right, #000 0px, #000 calc(100% - 14px), transparent 100%);
  -webkit-mask-image: linear-gradient(to right, #000 0px, #000 calc(100% - 14px), transparent 100%);
}
.spotlight-action-menu-button:hover .spotlight-action-popover__copy {
  animation: folder-mask-fade 5.5s linear infinite alternate;
}
.spotlight-action-popover__copy strong,
.spotlight-action-popover__copy small {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.spotlight-action-menu-button:hover .spotlight-action-popover__copy strong,
.spotlight-action-menu-button:hover .spotlight-action-popover__copy small {
  text-overflow: clip;
}
.spotlight-action-popover__copy strong {
  font-size: 11px;
  font-weight: 650;
}
.spotlight-action-popover__copy small {
  margin-top: 2px;
  color: var(--text-muted);
  font-size: 9px;
}
.spotlight-action-marquee-inner {
  display: inline-block;
  white-space: nowrap;
  will-change: transform;
}
.spotlight-action-menu-button:hover .spotlight-action-marquee-inner {
  animation: folder-name-bounce 5.5s cubic-bezier(0.45, 0.05, 0.55, 0.95) infinite alternate;
}
.spotlight-action-popover__preview {
  position: relative;
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface-hover);
  color: var(--primary-text);
}
.spotlight-action-popover__preview-image,
.spotlight-action-popover__preview :deep(.thumbnail-loader) {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.spotlight-action-popover__preview-loading {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--surface-elevated) 68%, transparent);
  color: var(--primary-text);
  backdrop-filter: blur(5px);
}
.action-popover-enter-active,
.action-popover-leave-active {
  transition:
    opacity 130ms ease,
    transform 170ms cubic-bezier(0.16, 1, 0.3, 1),
    filter 130ms ease;
}
.action-popover-enter-from,
.action-popover-leave-to {
  opacity: 0;
  transform: translate3d(0, 7px, 0) scale(0.965);
  filter: blur(3px);
}
.spotlight-loading-list {
  display: grid;
  gap: 8px;
  padding: 4px;
}
.spotlight-loading-list span {
  height: 70px;
  border-radius: 14px;
  background: linear-gradient(
    100deg,
    var(--surface-hover) 25%,
    color-mix(in srgb, var(--primary-soft) 44%, var(--surface)) 44%,
    var(--surface-hover) 63%
  );
  background-size: 260% 100%;
  animation: skeleton-shimmer 1.35s linear infinite;
}
.spotlight-semantic-searching {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 6px 10px;
  color: var(--text-muted);
  font-size: 10px;
}
.spotlight-semantic-searching .spin { color: var(--primary-text); }
.spotlight-semantic-empty { display: grid; place-items: center; align-content: center; min-height: 260px; padding: 32px; color: var(--text-muted); text-align: center; }
.spotlight-semantic-empty svg { margin-bottom: 13px; color: var(--primary-text); }
.spotlight-semantic-empty strong { color: var(--text); font-size: 14px; }
.spotlight-semantic-empty span { max-width: 310px; margin-top: 7px; font-size: 11px; line-height: 1.5; }
.spotlight-empty {
  display: grid;
  place-items: center;
  align-content: center;
  min-height: 300px;
  padding: 32px;
  color: var(--text-muted);
  text-align: center;
}
.spotlight-empty svg {
  margin-bottom: 13px;
  color: var(--primary-text);
}
.spotlight-empty strong {
  color: var(--text);
  font-size: 14px;
}
.spotlight-empty span {
  margin-top: 7px;
  font-size: 11px;
}
.spotlight-error {
  margin: 10px;
  padding: 11px 12px;
  border: 1px solid var(--danger-border);
  border-radius: 11px;
  background: var(--danger-surface);
  color: var(--danger-text);
  font-size: 11px;
}
.spotlight-scroll-shadow {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 4;
  height: 54px;
  pointer-events: none;
  background: linear-gradient(
    180deg,
    transparent,
    color-mix(in srgb, var(--surface-elevated) 96%, transparent) 88%
  );
  box-shadow: inset 0 -13px 17px -17px rgb(2 6 23 / 0.32);
}
.spin {
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(1turn);
  }
}
@keyframes result-list-reveal {
  from {
    opacity: 0;
    transform: translate3d(0, 5px, 0);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
@keyframes action-success {
  0% {
    transform: scale(0.94);
  }
  55% {
    transform: scale(1.04);
  }
  100% {
    transform: none;
  }
}
@keyframes skeleton-shimmer {
  to {
    background-position: -160% 0;
  }
}
@keyframes folder-name-bounce {
  0%, 15% {
    transform: translateX(0%);
  }
  85%, 100% {
    transform: translateX(calc(-100% + 75px));
  }
}
@keyframes folder-mask-fade {
  0%, 12% {
    mask-image: linear-gradient(to right, #000 0px, #000 calc(100% - 14px), transparent 100%);
    -webkit-mask-image: linear-gradient(to right, #000 0px, #000 calc(100% - 14px), transparent 100%);
  }
  24% {
    mask-image: linear-gradient(to right, transparent 0px, #000 6px, #000 calc(100% - 14px), transparent 100%);
    -webkit-mask-image: linear-gradient(to right, transparent 0px, #000 6px, #000 calc(100% - 14px), transparent 100%);
  }
  35%, 65% {
    mask-image: linear-gradient(to right, transparent 0px, #000 14px, #000 calc(100% - 14px), transparent 100%);
    -webkit-mask-image: linear-gradient(to right, transparent 0px, #000 14px, #000 calc(100% - 14px), transparent 100%);
  }
  76% {
    mask-image: linear-gradient(to right, transparent 0px, #000 14px, #000 calc(100% - 6px), transparent 100%);
    -webkit-mask-image: linear-gradient(to right, transparent 0px, #000 14px, #000 calc(100% - 6px), transparent 100%);
  }
  88%, 100% {
    mask-image: linear-gradient(to right, transparent 0px, #000 14px, #000 100%);
    -webkit-mask-image: linear-gradient(to right, transparent 0px, #000 14px, #000 100%);
  }
}
@media (prefers-reduced-motion: reduce) {
  .spotlight-dock-button--success,
  .action-popover-enter-active,
  .action-popover-leave-active,
  .spin {
    animation-duration: 0.01ms;
    transition-duration: 0.01ms;
  }
  .spotlight-action-menu-button:hover .spotlight-action-marquee,
  .spotlight-action-menu-button:hover .spotlight-action-marquee-inner {
    animation: none;
  }
}
</style>
