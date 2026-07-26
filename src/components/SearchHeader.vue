<script setup lang="ts">
import { ref } from 'vue'
import { LoaderCircle, Search, X } from '@lucide/vue'
import Button from './ui/Button/Button.vue'
import Input from './ui/Input/Input.vue'
import MovingBorder from './ui/MovingBorder/MovingBorder.vue'

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
      <MovingBorder border-radius="14px" :duration="searching ? 2600 : 4200" :active="true">
        <Input
          ref="searchInput"
          :model-value="props.modelValue"
          type="search"
          :placeholder="placeholder ?? 'Nom de fichier ou description naturelle…'"
          aria-label="Rechercher des images"
          class="floating-search-input"
          @update:model-value="emit('update:modelValue', $event)"
        >
          <template #leading>
            <div class="header-icon-wrapper">
              <Transition name="icon-swap" mode="out-in">
                <LoaderCircle v-if="props.searching" key="loader" class="spin" :size="18" :stroke-width="2.2" />
                <Search v-else key="search" :size="18" :stroke-width="1.9" />
              </Transition>
            </div>
          </template>
          <template #trailing>
            <div class="search-trailing-actions">
              <span
                v-if="props.modelValue.trim() && props.resultCount != null"
                class="search-count-badge"
              >
                {{ props.searching ? 'Recherche…' : `${props.resultCount} résultat${props.resultCount > 1 ? 's' : ''}` }}
              </span>
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
      </MovingBorder>
    </div>
  </header>
</template>

<style scoped>
.search-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 20;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  min-height: 76px;
  padding: 14px 24px;
  background: linear-gradient(180deg, color-mix(in srgb, var(--surface) 82%, transparent) 0%, color-mix(in srgb, var(--surface) 40%, transparent) 75%, transparent 100%);
  backdrop-filter: blur(24px) saturate(1.25);
  -webkit-backdrop-filter: blur(24px) saturate(1.25);
  pointer-events: none;
}
.search-field {
  width: 100%;
  max-width: 640px;
  pointer-events: auto;
}

.floating-search-input {
  min-height: 48px;
  border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  border-radius: 13px;
  background: color-mix(in srgb, var(--surface-elevated) 88%, transparent);
  box-shadow: 0 8px 24px -6px rgba(0, 0, 0, 0.12), inset 0 1px 0 rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(16px);
}

.header-icon-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
}

.search-trailing-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.search-count-badge {
  padding: 3px 9px;
  border: 1px solid var(--border);
  border-radius: var(--radius-full);
  background: color-mix(in srgb, var(--surface-hover) 88%, transparent);
  color: var(--text-muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  font-weight: 500;
}

.spin {
  animation: spin 0.85s linear infinite;
  color: var(--primary);
}

.icon-swap-enter-active,
.icon-swap-leave-active {
  transition: opacity 160ms ease, transform 180ms cubic-bezier(0.16, 1, 0.3, 1);
}
.icon-swap-enter-from { opacity: 0; transform: scale(0.7) rotate(-45deg); }
.icon-swap-leave-to { opacity: 0; transform: scale(0.7) rotate(45deg); }

@keyframes spin {
  to { transform: rotate(1turn); }
}
</style>
