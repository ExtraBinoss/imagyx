<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  Folder,
  FolderOpen,
  Images,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Settings,
  Trash2,
} from "@lucide/vue";
import type {
  FollowedFolder,
  IndexProgress,
  ModelDownloadProgress,
  RuntimeStats,
} from "@/types";
import { imagyxApi } from "@/api/tauri";
import SpotlightSettings from "@/components/Spotlight/SpotlightSettings.vue";
import LocalAiStatus from "@/components/LocalAiStatus.vue";
import Button from "@/components/ui/Button/Button.vue";
import Popover from "@/components/ui/Popover/Popover.vue";
import { useThemeStore } from "@/stores/theme";
import { useShortcutStore } from "@/stores/shortcut";
import { usePlatformStore } from "@/stores/platform";
import TitleBar from "@/components/TitleBar.vue";
import imagyxLogo from "../../src-tauri/icons/imagyx-smaller.avif";

const props = defineProps<{
  folders: FollowedFolder[];
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

const platform = usePlatformStore();
const theme = useThemeStore();
const shortcut = useShortcutStore();
const allSelected = computed(() => props.selectedFolderId === null);
const contextMenu = ref<{ folderId: string; x: number; y: number } | null>(
  null,
);

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
    <TitleBar />
    <div class="brand">
      <img :src="imagyxLogo" class="brand-mark-img" alt="Imagyx logo" />
      <div><strong>Imagyx</strong><span>Local Intelligence</span></div>
    </div>

    <Button variant="primary" size="lg" block @click="emit('add')">
      <template #leading><Plus :size="17" :stroke-width="2.2" /></template>
      Add Folder
    </Button>

    <nav class="folder-nav" aria-label="Watched Folders">
      <p class="section-label">Library</p>
      <Button
        class="folder-row"
        :class="{ active: allSelected }"
        variant="ghost"
        block
        @click="emit('select', null)"
      >
        <Images :size="17" />
        <span class="folder-name">All Images</span>
        <span class="folder-count">{{ totalImages }}</span>
      </Button>

      <p class="section-label followed-label">Watched Folders</p>
      <div v-if="folders.length === 0" class="sidebar-empty">
        <FolderOpen :size="19" /><span>No watched folders</span>
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
          <Folder :size="17" />
          <span class="folder-name">{{ folder.name }}</span>
          <span class="folder-count">{{ folder.imageCount }}</span>
        </Button>
        <div class="folder-actions" @click.stop @pointerdown.stop>
          <Popover align="end" width="224px">
            <template #trigger>
              <Button variant="ghost" size="icon" aria-label="Folder Actions">
                <MoreHorizontal :size="16" />
              </Button>
            </template>
            <template #content="{ close }">
              <div class="folder-menu">
                <Button
                  variant="ghost"
                  size="sm"
                  block
                  @click="
                    openFolder(folder.id);
                    close();
                  "
                >
                  <template #leading><FolderOpen :size="15" /></template
                  >{{ platform.openFolderLabel }}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  block
                  @click="
                    emit('reindex', folder.id);
                    close();
                  "
                >
                  <template #leading><RefreshCw :size="15" /></template>Reindex
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
                  <template #leading><Trash2 :size="15" /></template>Unfollow
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

      <Popover side="top" align="start" width="380px" class="full-width-popover">
        <template #trigger>
          <Button variant="ghost" size="sm" block class="settings-trigger-btn">
            <template #leading><Settings :size="15" /></template>
            Settings & Theme
          </Button>
        </template>
        <template #content>
          <SpotlightSettings
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
        >{{ platform.openFolderLabel }}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        block
        @click="runContextAction('reindex')"
      >
        <template #leading><RefreshCw :size="15" /></template>Reindex
      </Button>
      <Button
        variant="danger"
        size="sm"
        block
        @click="runContextAction('remove')"
      >
        <template #leading><Trash2 :size="15" /></template>Unfollow
      </Button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  overflow-x: hidden;
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
</style>
