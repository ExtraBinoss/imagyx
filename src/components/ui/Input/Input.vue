<script setup lang="ts">
import { ref } from 'vue'

withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    type?: 'text' | 'search'
    autocomplete?: string
    spellcheck?: boolean
    ariaLabel?: string
  }>(),
  {
    placeholder: '',
    type: 'text',
    autocomplete: 'off',
    spellcheck: false,
    ariaLabel: undefined,
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const input = ref<HTMLInputElement | null>(null)

function focus() {
  input.value?.focus()
}

function select() {
  input.value?.select()
}

defineExpose({ focus, select })
</script>

<template>
  <label class="ui-input">
    <span v-if="$slots.leading" class="ui-input__slot"><slot name="leading" /></span>
    <input
      ref="input"
      :value="modelValue"
      :type="type"
      :placeholder="placeholder"
      :autocomplete="autocomplete"
      :spellcheck="spellcheck"
      :aria-label="ariaLabel"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
    <span v-if="$slots.trailing" class="ui-input__slot"><slot name="trailing" /></span>
  </label>
</template>

<style scoped>
.ui-input {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  width: 100%;
  min-height: 42px;
  padding: 0 var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  color: var(--text-muted);
  transition:
    border-color var(--transition-fast),
    background-color var(--transition-fast);
}

.ui-input:hover { border-color: var(--border-strong); }
.ui-input:focus-within {
  border-color: var(--primary);
  outline: 2px solid var(--focus-ring-soft);
  outline-offset: 1px;
}
.ui-input input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
}
.ui-input input::-webkit-search-cancel-button { display: none; }
.ui-input input::placeholder { color: var(--text-subtle); }
.ui-input__slot { display: inline-flex; flex: 0 0 auto; }
</style>
