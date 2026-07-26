<script setup lang="ts">
import { Search, Sparkles, X } from '@lucide/vue'

defineProps<{
  modelReady: boolean
  modelBackend: string
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <header class="search-header">
    <div class="search-box">
      <Search :size="19" />
      <input
        :value="modelValue"
        type="search"
        autocomplete="off"
        spellcheck="false"
        placeholder="Nom de fichier ou description naturelle…"
        @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      />
      <button
        v-if="modelValue"
        class="clear-search"
        type="button"
        title="Effacer"
        @click="emit('update:modelValue', '')"
      >
        <X :size="16" />
      </button>
    </div>
    <div class="ai-pill" :class="{ ready: modelReady }" :title="modelBackend">
      <Sparkles :size="15" />
      <span>{{ modelReady ? modelBackend : 'IA en préparation' }}</span>
    </div>
  </header>
</template>
