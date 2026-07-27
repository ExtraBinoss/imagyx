<script setup lang="ts">
import { Folder } from '@lucide/vue'
import type { FollowedFolder } from '../types'
import Button from './ui/Button/Button.vue'

defineProps<{
  folders: FollowedFolder[]
  activeIndex: number
  placement: 'above' | 'below'
}>()

const emit = defineEmits<{
  select: [folder: FollowedFolder]
}>()
</script>

<template>
  <div
    v-if="folders.length"
    class="folder-query-autocomplete"
    :class="`folder-query-autocomplete--${placement}`"
    role="listbox"
  >
    <Button
      v-for="(folder, index) in folders"
      :key="folder.id"
      class="folder-query-autocomplete__item"
      :class="{ 'folder-query-autocomplete__item--active': index === activeIndex }"
      variant="ghost"
      size="sm"
      block
      role="option"
      :aria-selected="index === activeIndex"
      @mousedown.prevent
      @click="emit('select', folder)"
    >
      <template #leading><Folder :size="14" /></template>
      <span>{{ folder.name }}</span>
    </Button>
  </div>
</template>

<style scoped>
.folder-query-autocomplete {
  position: absolute;
  animation: folder-query-pop 140ms cubic-bezier(0.16, 1, 0.3, 1) both;
  right: 0;
  left: 0;
  z-index: var(--z-popover);
  display: grid;
  gap: 2px;
  padding: 5px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-elevated);
  box-shadow: var(--shadow-popover);
}
.folder-query-autocomplete--below { top: calc(100% + 7px); }
.folder-query-autocomplete--above { bottom: calc(100% + 7px); }
.folder-query-autocomplete__item { justify-content: flex-start; min-height: 31px; padding-inline: 9px; font-size: 11px; }
.folder-query-autocomplete__item--active { background: var(--surface-hover); color: var(--text); }
@keyframes folder-query-pop { from { opacity: 0; transform: translate3d(0, 4px, 0); } to { opacity: 1; transform: none; } }
</style>
