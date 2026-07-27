<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import { Keyboard } from '@lucide/vue'
import { useTranslate } from '../../i18n'

const { t } = useTranslate()

const props = withDefaults(defineProps<{
  modelValue: string
  label?: string
  description?: string
  disabled?: boolean
}>(), {
  label: '',
  description: '',
  disabled: false,
})

const emit = defineEmits<{ change: [value: string] }>()
const field = ref<HTMLElement | null>(null)
const recording = ref(false)
const message = ref('')

const tokens = computed(() => displayTokens(props.modelValue))

function startRecording() {
  if (props.disabled) return
  recording.value = true
  message.value = t('shortcut.press_combo')
  void nextTick(() => field.value?.focus())
}

function stopRecording() {
  recording.value = false
  message.value = ''
}

function capture(event: KeyboardEvent) {
  if (!recording.value || props.disabled) return
  event.preventDefault()
  event.stopPropagation()

  if (event.key === 'Escape') {
    stopRecording()
    return
  }

  const key = normalizeKey(event)
  if (!key) {
    message.value = t('shortcut.hold_modifier')
    return
  }

  const modifiers = [
    event.ctrlKey ? 'Control' : '',
    event.altKey ? 'Alt' : '',
    event.shiftKey ? 'Shift' : '',
    event.metaKey ? 'Super' : '',
  ].filter(Boolean)

  if (modifiers.length === 0) {
    message.value = t('shortcut.add_modifier')
    return
  }

  emit('change', [...modifiers, key].join('+'))
  recording.value = false
  message.value = t('shortcut.applying')
  window.setTimeout(() => { if (!recording.value) message.value = '' }, 1400)
}

function normalizeKey(event: KeyboardEvent): string | null {
  if (['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight'].includes(event.code)) return null
  if (/^Key[A-Z]$/.test(event.code)) return event.code
  if (/^Digit\d$/.test(event.code)) return event.code
  if (/^Numpad\d$/.test(event.code)) return event.code
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(event.code)) return event.code
  const supported = new Set([
    'Space', 'Enter', 'Tab', 'Backspace', 'Delete', 'Insert', 'Home', 'End', 'PageUp', 'PageDown',
    'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Comma', 'Period', 'Slash', 'Semicolon',
    'Quote', 'BracketLeft', 'BracketRight', 'Backslash', 'Minus', 'Equal', 'Backquote',
  ])
  return supported.has(event.code) ? event.code : null
}

function displayTokens(shortcut: string): string[] {
  return shortcut.split('+').filter(Boolean).map((token) => {
    const normalized = token.toLocaleLowerCase('en')
    if (normalized === 'control' || normalized === 'ctrl') return 'Ctrl'
    if (normalized === 'meta' || normalized === 'super' || normalized === 'command' || normalized === 'cmd') return 'Cmd'
    if (normalized === 'alt') return 'Alt'
    if (normalized === 'shift') return 'Shift'
    if (/^key[a-z]$/i.test(token)) return token.slice(3).toUpperCase()
    if (/^digit\d$/i.test(token)) return token.slice(5)
    if (/^numpad\d$/i.test(token)) return `Num ${token.slice(6)}`
    if (normalized === 'space') return 'Espace'
    if (normalized.startsWith('arrow')) return `Flèche ${token.slice(5)}`
    return token
  })
}
</script>

<template>
  <div class="shortcut-view">
    <div class="shortcut-copy">
      <span class="shortcut-icon"><Keyboard :size="17" /></span>
      <div>
        <strong>{{ label }}</strong>
        <p v-if="description">{{ description }}</p>
      </div>
    </div>

    <div
      ref="field"
      class="shortcut-field"
      :class="{ 'shortcut-field--recording': recording, 'shortcut-field--disabled': disabled }"
      role="button"
      :tabindex="disabled ? -1 : 0"
      :aria-label="`${label || t('shortcut.press_combo')}: ${tokens.join(' + ')}`"
      @click="startRecording"
      @focus="recording = true"
      @blur="stopRecording"
      @keydown="capture"
    >
      <template v-for="(token, index) in tokens" :key="`${token}-${index}`">
        <span class="shortcut-key">{{ token }}</span>
        <span v-if="index < tokens.length - 1" class="shortcut-plus">+</span>
      </template>
    </div>

    <span v-if="message" class="shortcut-message">{{ message }}</span>
  </div>
</template>

<style scoped>
.shortcut-view {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px 18px;
  padding: 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--surface) 92%, transparent);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.14), 0 8px 22px -19px rgb(15 23 42 / 0.34);
}
.shortcut-copy { display: flex; align-items: center; gap: 11px; min-width: 0; }
.shortcut-icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-hover);
  color: var(--primary-text);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.18);
}
.shortcut-copy strong { display: block; color: var(--text); font-size: 12px; }
.shortcut-copy p { margin: 5px 0 0; color: var(--text-muted); font-size: 10px; line-height: 1.45; }
.shortcut-field {
  display: flex;
  align-items: center;
  min-height: 38px;
  padding: 5px 7px;
  border: 1px solid var(--border-strong);
  border-radius: 11px;
  background: color-mix(in srgb, var(--background) 82%, var(--surface));
  box-shadow: inset 0 2px 5px rgb(15 23 42 / 0.08), 0 1px 0 rgb(255 255 255 / 0.18);
  cursor: text;
  outline: none;
  transition: border-color 150ms ease, box-shadow 180ms ease, transform 180ms ease;
}
.shortcut-field:hover,
.shortcut-field:focus-visible,
.shortcut-field--recording {
  border-color: var(--primary);
  box-shadow: inset 0 2px 5px rgb(15 23 42 / 0.07), 0 0 0 4px color-mix(in srgb, var(--primary) 12%, transparent);
  transform: translateY(-1px);
}
.shortcut-field--disabled { cursor: not-allowed; opacity: 0.55; }
.shortcut-key {
  min-width: 27px;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-bottom-color: var(--border-strong);
  border-radius: 7px;
  background: var(--surface-elevated);
  color: var(--text-secondary);
  font-size: 10px;
  font-weight: 680;
  text-align: center;
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.22), 0 2px 0 var(--border);
}
.shortcut-plus { padding: 0 5px; color: var(--text-subtle); font-size: 10px; }
.shortcut-message { grid-column: 1 / -1; color: var(--text-muted); font-size: 10px; text-align: right; }
:global(:root[data-theme='dark']) .shortcut-field { background: color-mix(in srgb, var(--background) 86%, black); }
</style>
