<script setup lang="ts">
import { ref } from 'vue'
import { LoaderCircle, Search, X } from '@lucide/vue'
import Button from './ui/Button/Button.vue'
import Input from './ui/Input/Input.vue'

const props = defineProps<{
  modelReady: boolean
  modelBackend: string
  modelValue: string
  searching?: boolean
  resultCount?: number
  placeholder?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const searchInput = ref<{ focus: () => void; select: () => void } | null>(null)

function focusSearch() {
  searchInput.value?.focus()
}

function selectSearch() {
  searchInput.value?.select()
}

defineExpose({ focusSearch, selectSearch })
</script>

<template>
  <header class="search-header" data-tauri-drag-region>
    <div class="search-field">
      <Input
        ref="searchInput"
        :model-value="props.modelValue"
        type="search"
        :placeholder="placeholder ?? 'Nom de fichier ou description naturelle…'"
        aria-label="Rechercher des images"
        @update:model-value="emit('update:modelValue', $event)"
      >
        <template #leading><Search :size="18" /></template>
        <template #trailing>
          <div class="search-trailing-actions">
            <span
              v-if="props.modelValue.trim() && !props.searching && props.resultCount != null"
              class="search-count-badge"
            >
              {{ props.resultCount }} résultat{{ props.resultCount > 1 ? 's' : '' }}
            </span>
            <LoaderCircle v-if="props.searching" class="spin" :size="16" />
            <Button
              v-if="props.modelValue"
              variant="ghost"
              size="icon"
              aria-label="Effacer la recherche"
              @click="emit('update:modelValue', '')"
            >
              <X :size="15" />
            </Button>
          </div>
        </template>
      </Input>
    </div>
  </header>
</template>

<style scoped>
.search-trailing-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.search-count-badge {
  padding: 3px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--surface-hover) 85%, transparent);
  color: var(--text-muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  font-weight: 500;
}
.spin {
  animation: spin 0.85s linear infinite;
  color: var(--primary);
}
@keyframes spin {
  to { transform: rotate(1turn); }
}
</style>
