<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  ChevronsUpDown,
  Folder,
  FolderOpen,
  Images,
  LoaderCircle,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Settings,
  TriangleAlert,
  Trash2,
} from "@lucide/vue";
import type {
  FollowedFolder,
  FolderIndexCoverage,
  IndexProgress,
  ModelDownloadProgress,
  RuntimeStats,
} from "@/types";
import SpotlightSettings from "@/components/Spotlight/SpotlightSettings.vue";
import LocalAiStatus from "@/components/LocalAiStatus.vue";
import Button from "@/components/ui/Button/Button.vue";
import Popover from "@/components/ui/Popover/Popover.vue";
import Tooltip from "@/components/ui/Tooltip/Tooltip.vue";
import { useThemeStore } from "@/stores/theme";
import { useShortcutStore } from "@/stores/shortcut";
import { usePlatformStore } from "@/stores/platform";
import TitleBar from "@/components/TitleBar.vue";
import imagyxLogo from "../../src-tauri/icons/imagyx-bigger.avif";
import { useTranslate } from "@/i18n";

const props = defineProps<{
  folders: FollowedFolder[];
  indexCoverage: FolderIndexCoverage[];
  selectedFolderId: string | null;
  totalImages: number;
  progress: IndexProgress | null;
  modelProgress: ModelDownloadProgress | null;
  runtimeStats: RuntimeStats | null;
}>();

const emit = defineEmits<{
  select: [folderId: string | null];
  add: [];
  remove: [folderId: string];
  reindex: [folderId: string];
  pauseIndexing: [];
  resumeIndexing: [];
}>();

const { t } = useTranslate();
const platform = usePlatformStore();
const theme = useThemeStore();
const shortcut = useShortcutStore();
const allSelected = computed(() => props.selectedFolderId === null);
const contextMenu = ref<{ folderId: string; x: number; y: number } | null>(
  null,
);

function isFolderReindexing(folderId: string): boolean {
  const progress = props.progress;
  return Boolean(
    progress &&
    progress.folderId === folderId &&
    progress.stage !== "complete" &&
    progress.stage !== "error",
  );
}

function isFolderIncomplete(folderId: string): boolean {
  const coverage = coverageFor(folderId);
  return Boolean(coverage && coverage.embeddedCount < coverage.imageCount && !isFolderReindexing(folderId));
}

function coverageFor(folderId: string): FolderIndexCoverage | null {
  return props.indexCoverage.find((item) => item.folderId === folderId) ?? null;
}

function openContextMenu(event: MouseEvent, folderId: string) {
  event.preventDefault();
  event.stopPropagation();
  contextMenu.value = {
    folderId,
    x: Math.min(event.clientX, window.innerWidth - 224),
    y: Math.min(event.clientY, window.innerHeight - 156),
  };
}

function closeContextMenu() {
  contextMenu.value = null;
}

async function openFolder(folderId: string) {
  const folder = props.folders.find((item) => item.id === folderId);
  if (folder) await imagyxApi.openInFileManager(folder.path);
}

async function runContextAction(action: "open" | "reindex" | "remove") {
  const folderId = contextMenu.value?.folderId;
  if (!folderId) return;
  closeContextMenu();
  if (action === "open") {
    await openFolder(folderId);
    return;
  }
  emit(action, folderId);
}

onMounted(() => {
  void platform.initialize();
  void shortcut.initialize();
  window.addEventListener("pointerdown", closeContextMenu);
  window.addEventListener("blur", closeContextMenu);
});

onBeforeUnmount(() => {
  window.removeEventListener("pointerdown", closeContextMenu);
  window.removeEventListener("blur", closeContextMenu);
});
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header-bar" data-tauri-drag-region>
      <TitleBar v-if="platform.controlsPosition === 'left'" />
    </div>
    <div class="brand">
      <img :src="imagyxLogo" class="brand-mark-img" alt="Imagyx logo" />
      <div>
        <strong>Imagyx</strong><span>{{ t("app.tagline") }}</span>
      </div>
    </div>

    <Button variant="primary" size="lg" block @click="emit('add')">
      <template #leading><Plus :size="17" :stroke-width="2.2" /></template>
      {{ t("sidebar.add_folder") }}
    </Button>

    <nav class="folder-nav" :aria-label="t('sidebar.watched_folders')">
      <p class="section-label">{{ t("sidebar.library") }}</p>
      <Button
        class="folder-row"
        :class="{ active: allSelected }"
        variant="ghost"
        block
        @click="emit('select', null)"
      >
        <Images :size="17" />
        <span class="folder-name">{{ t("sidebar.all_images") }}</span>
        <span class="folder-count">{{ totalImages }}</span>
      </Button>

      <p class="section-label followed-label">
        {{ t("sidebar.watched_folders") }}
      </p>
      <div v-if="folders.length === 0" class="sidebar-empty">
        <FolderOpen :size="19" /><span>{{
          t("sidebar.no_watched_folders")
        }}</span>
      </div>

      <div
        v-for="folder in folders"
        :key="folder.id"
        class="folder-entry"
        @contextmenu="openContextMenu($event, folder.id)"
      >
        <Button
          class="folder-row"
          :class="{ active: selectedFolderId === folder.id }"
          variant="ghost"
          block
          @click="emit('select', folder.id)"
        >
          <LoaderCircle v-if="isFolderReindexing(folder.id)" class="folder-index-spinner" :size="17" />
          <Tooltip
            v-else-if="isFolderIncomplete(folder.id)"
            :text="t('sidebar.index_coverage', { embedded: coverageFor(folder.id)?.embeddedCount, total: coverageFor(folder.id)?.imageCount })"
          >
            <TriangleAlert class="folder-index-warning" :size="17" />
          </Tooltip>
          <Folder v-else :size="17" />
          <span class="folder-name">{{ folder.name }}</span>
          <span class="folder-count">{{ folder.imageCount }}</span>
        </Button>
        <div class="folder-actions" @click.stop @pointerdown.stop>
          <Popover align="end" width="224px">
            <template #trigger>
              <Button
                variant="ghost"
                size="icon"
                :aria-label="t('sidebar.folder_actions')"
              >
                <MoreHorizontal :size="16" />
              </Button>
            </template>
            <template #content="{ close }">
              <div class="folder-menu">
                <Button
                  :variant="isFolderIncomplete(folder.id) ? 'secondary' : 'ghost'"
                  size="sm"
                  block
                  @click="
                    openFolder(folder.id);
                    close();
                  "
                >
                  <template #leading><FolderOpen :size="15" /></template
                  >{{
                    t("open_in_file_manager", {
                      name: platform.fileManagerName,
                    })
                  }}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  block
                  :loading="isFolderReindexing(folder.id)"
                  @click="
                    emit('reindex', folder.id);
                    close();
                  "
                >
                  <template #leading><RefreshCw :size="15" /></template>
                  {{
                    isFolderReindexing(folder.id)
                      ? t("sidebar.reindexing")
                      : isFolderIncomplete(folder.id)
                        ? t("sidebar.reindex_incomplete")
                        : t("sidebar.reindex")
                  }}
                </Button>
                <Button
                  variant="danger"
                  size="sm"
                  block
                  @click="
                    emit('remove', folder.id);
                    close();
                  "
                >
                  <template #leading><Trash2 :size="15" /></template
                  >{{ t("sidebar.unfollow") }}
                </Button>
              </div>
            </template>
          </Popover>
        </div>
      </div>
    </nav>

    <div class="sidebar-bottom-actions">
      <LocalAiStatus
        :progress="progress"
        :model-progress="modelProgress"
        :runtime-stats="runtimeStats"
        @pause="emit('pauseIndexing')"
        @resume="emit('resumeIndexing')"
      />

      <Popover
        side="top"
        align="start"
        width="380px"
        class="full-width-popover"
      >
        <template #trigger>
          <Button variant="ghost" size="sm" block class="settings-trigger-btn">
            <template #leading><Settings :size="15" /></template>
            {{ t("settings") }}
            <template #trailing
              ><ChevronsUpDown :size="14" class="settings-chevron"
            /></template>
          </Button>
        </template>
        <template #content>
          <SpotlightSettings
            variant="popover"
            query=""
            :shortcut="shortcut.displayShortcut || 'Ctrl+Numpad9'"
            :shortcut-updating="shortcut.isRegistering || false"
            :shortcut-error="shortcut.error || null"
            :theme-mode="theme.mode || 'system'"
            @shortcut-change="shortcut.registerShortcut"
            @theme-change="theme.setMode"
          />
        </template>
      </Popover>
    </div>

    <div
      v-if="contextMenu"
      class="folder-context-menu"
      :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
      @pointerdown.stop
      @click.stop
    >
      <Button variant="ghost" size="sm" block @click="runContextAction('open')">
        <template #leading><FolderOpen :size="15" /></template
        >{{ t("open_in_file_manager", { name: platform.fileManagerName }) }}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        block
        :loading="isFolderReindexing(contextMenu.folderId)"
        @click="runContextAction('reindex')"
      >
        <template #leading><RefreshCw :size="15" /></template>
        {{
          isFolderReindexing(contextMenu.folderId)
            ? t("sidebar.reindexing")
            : t("sidebar.reindex")
        }}
      </Button>
      <Button
        variant="danger"
        size="sm"
        block
        @click="runContextAction('remove')"
      >
        <template #leading><Trash2 :size="15" /></template
        >{{ t("sidebar.unfollow") }}
      </Button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  overflow-x: hidden;
}
.sidebar-header-bar {
  min-height: 38px;
  display: flex;
  align-items: center;
  -webkit-app-region: drag;
}
.brand-mark-img {
  width: 34px;
  height: 34px;
  border-radius: var(--radius-md);
  object-fit: contain;
  flex-shrink: 0;
}
.folder-nav {
  overflow-x: hidden;
  scrollbar-gutter: stable;
  display: grid;
  gap: 2px;
}
.folder-entry {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
}
.folder-entry .folder-row {
  width: 100%;
}
.folder-index-spinner {
  animation: folder-index-spin 0.8s linear infinite;
}
.folder-index-warning {
  color: var(--warning-text);
}
.folder-actions {
  position: absolute;
  right: 6px;
  z-index: 2;
  display: flex;
  align-items: center;
  border-radius: var(--radius-full);
  backdrop-filter: blur(8px);
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transition: opacity var(--transition-fast);
}
.folder-entry:hover .folder-actions,
.folder-entry:focus-within .folder-actions {
  opacity: 1;
  visibility: visible;
  pointer-events: auto;
}
.folder-menu {
  display: grid;
  gap: var(--space-1);
}
.folder-menu :deep(.ui-button) {
  justify-content: flex-start;
}
.folder-context-menu {
  position: fixed;
  z-index: var(--z-popover);
  display: grid;
  gap: var(--space-1);
  width: 224px;
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
  box-shadow: var(--shadow-popover);
}

.settings-trigger-btn {
  justify-content: flex-start !important;
  width: 100% !important;
  text-align: left !important;
}
.settings-trigger-btn :deep(.ui-button__content) {
  flex: 1;
  text-align: left;
}
.settings-chevron {
  margin-left: auto;
  opacity: 0.5;
  transition: opacity var(--transition-fast);
}
.settings-trigger-btn:hover .settings-chevron {
  opacity: 1;
}
@keyframes folder-index-spin {
  to {
    transform: rotate(1turn);
  }
}
</style>
