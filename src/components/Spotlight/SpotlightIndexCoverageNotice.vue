<script setup lang="ts">
import { TriangleAlert } from '@lucide/vue'
import type { FolderIndexCoverage } from '../../types'
import Button from '../ui/Button/Button.vue'
import { useTranslate } from '../../i18n'

defineProps<{
  coverage: FolderIndexCoverage[]
}>()

const emit = defineEmits<{
  resume: [folderIds: string[]]
}>()

const { t } = useTranslate()
</script>

<template>
  <aside class="index-coverage-notice" role="status">
    <span class="index-coverage-notice__icon"><TriangleAlert :size="17" /></span>
    <span class="index-coverage-notice__copy">
      <strong>{{ t('spotlight.index_incomplete_title') }}</strong>
      <small>{{ t('spotlight.index_incomplete_desc', { count: coverage.length }) }}</small>
    </span>
    <Button
      class="index-coverage-notice__action"
      variant="secondary"
      size="sm"
      @click="emit('resume', coverage.map((item) => item.folderId))"
    >
      {{ t('sidebar.resume_indexing') }}
    </Button>
  </aside>
</template>

<style scoped>
.index-coverage-notice {
  display: grid;
  grid-template-columns: 30px minmax(0, 1fr) auto;
  align-items: center;
  gap: 9px;
  margin: 4px 4px 8px;
  padding: 9px 10px;
  border: 1px solid color-mix(in srgb, var(--warning-border) 78%, var(--border));
  border-radius: 12px;
  background: color-mix(in srgb, var(--warning-surface) 72%, var(--surface));
}
.index-coverage-notice__icon {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: color-mix(in srgb, var(--warning-surface) 88%, var(--surface));
  color: var(--warning-text);
}
.index-coverage-notice__copy { min-width: 0; }
.index-coverage-notice__copy strong,
.index-coverage-notice__copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.index-coverage-notice__copy strong { color: var(--text); font-size: 10px; }
.index-coverage-notice__copy small { margin-top: 3px; color: var(--text-muted); font-size: 9px; }
.index-coverage-notice__action { min-height: 29px; font-size: 9px; }
</style>
