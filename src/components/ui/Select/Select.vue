<script setup lang="ts">
import { computed } from 'vue'
import { Check, ChevronDown } from '@lucide/vue'
import Button from '../Button/Button.vue'
import Popover from '../Popover/Popover.vue'

interface SelectOption {
  label: string
  value: string
  description?: string
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    modelValue?: string
    options: SelectOption[]
    placeholder?: string
    ariaLabel?: string
  }>(),
  {
    placeholder: 'Sélectionner',
    ariaLabel: 'Sélectionner une option',
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const selected = computed(() => props.options.find((option) => option.value === props.modelValue))

function selectOption(option: SelectOption, close: () => void) {
  if (option.disabled) return
  emit('update:modelValue', option.value)
  close()
}
</script>

<template>
  <Popover align="end" width="220px">
    <template #trigger>
      <Button class="ui-select__trigger" variant="secondary" size="sm" :aria-label="ariaLabel">
        <span class="ui-select__value">{{ selected?.label ?? placeholder }}</span>
        <template #trailing><ChevronDown :size="14" /></template>
      </Button>
    </template>
    <template #content="{ close }">
      <div class="ui-select__options" role="listbox" :aria-label="ariaLabel">
        <button
          v-for="option in options"
          :key="option.value"
          class="ui-select__option"
          :class="{ 'ui-select__option--selected': option.value === modelValue }"
          type="button"
          role="option"
          :aria-selected="option.value === modelValue"
          :disabled="option.disabled"
          @click="selectOption(option, close)"
        >
          <span class="ui-select__copy">
            <strong>{{ option.label }}</strong>
            <small v-if="option.description">{{ option.description }}</small>
          </span>
          <Check v-if="option.value === modelValue" :size="15" />
        </button>
      </div>
    </template>
  </Popover>
</template>

<style scoped>
.ui-select__trigger {
  justify-content: space-between;
  min-width: 118px;
}

.ui-select__value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ui-select__options {
  display: grid;
  gap: var(--space-1);
}

.ui-select__option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  width: 100%;
  padding: var(--space-3);
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-secondary);
  text-align: left;
  cursor: pointer;
}

.ui-select__option:hover:not(:disabled),
.ui-select__option--selected {
  background: var(--surface-hover);
  color: var(--text);
}

.ui-select__option:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: -2px;
}

.ui-select__option:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.ui-select__copy {
  display: grid;
  min-width: 0;
  gap: var(--space-1);
}

.ui-select__copy strong {
  font-size: var(--text-sm);
  font-weight: 600;
}

.ui-select__copy small {
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.35;
}
</style>
