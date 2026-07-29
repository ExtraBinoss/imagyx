<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ArrowLeft, ArrowRight, Check, EyeOff, FolderPlus, PartyPopper, X } from '@lucide/vue'
import type { ThemeMode } from '../../stores/theme'
import Button from '../ui/Button/Button.vue'
import OnboardingFeaturePreview from './OnboardingFeaturePreview.vue'
import { useTranslate } from '../../i18n'

const props = defineProps<{
  open: boolean
  neverAskAgain: boolean
  themeMode: ThemeMode
  shortcut: string
  hasFolders: boolean
}>()

const emit = defineEmits<{
  close: []
  finish: []
  neverAskAgain: [value: boolean]
  addFolder: []
  themeChange: [value: ThemeMode]
}>()

const { t } = useTranslate()

const steps = [
  {
    titleKey: 'onboarding.step_search_title',
    descKey: 'onboarding.step_search_desc',
    kind: 'search' as const,
  },
  {
    titleKey: 'onboarding.step_indexing_title',
    descKey: 'onboarding.step_indexing_desc',
    kind: 'indexing' as const,
  },
  {
    titleKey: 'onboarding.step_spotlight_title',
    descKey: 'onboarding.step_spotlight_desc',
    kind: 'spotlight' as const,
  },
]

const stepIndex = ref(0)
const direction = ref<'forward' | 'backward'>('forward')
const currentStep = computed(() => {
  const step = steps[stepIndex.value] ?? steps[0]
  return {
    ...step,
    title: t(step.titleKey),
    description: t(step.descKey),
  }
})
const isFirst = computed(() => stepIndex.value === 0)
const isLast = computed(() => stepIndex.value === steps.length - 1)

watch(() => props.open, (open) => {
  if (!open) return
  stepIndex.value = 0
  direction.value = 'forward'
})

function goTo(index: number) {
  const bounded = Math.max(0, Math.min(steps.length - 1, index))
  direction.value = bounded >= stepIndex.value ? 'forward' : 'backward'
  stepIndex.value = bounded
}

function next() {
  if (isLast.value) emit('finish')
  else goTo(stepIndex.value + 1)
}

function handleKeydown(event: KeyboardEvent) {
  if (!props.open) return
  const target = event.target as HTMLElement | null
  if (target?.matches('input, textarea, select, [contenteditable="true"]')) return
  if (event.key === 'Escape') {
    event.preventDefault()
    emit('close')
  } else if (event.key === 'ArrowRight') {
    event.preventDefault()
    next()
  } else if (event.key === 'ArrowLeft' && !isFirst.value) {
    event.preventDefault()
    goTo(stepIndex.value - 1)
  }
}

onMounted(() => window.addEventListener('keydown', handleKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', handleKeydown))
</script>

<template>
  <Teleport to="body">
    <Transition name="onboarding-overlay">
      <div v-if="open" class="onboarding-overlay" role="presentation" @pointerdown.self="emit('close')">
        <section
          class="onboarding-dialog"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="`onboarding-title-${stepIndex}`"
          :aria-describedby="`onboarding-description-${stepIndex}`"
        >
          <Button class="onboarding-close" variant="ghost" size="icon" :aria-label="t('onboarding.close')" @click="emit('close')">
            <X :size="18" />
          </Button>

          <header class="onboarding-header">
            <Transition :name="direction === 'forward' ? 'step-forward' : 'step-backward'" mode="out-in">
              <div :key="`copy-${stepIndex}`" class="onboarding-copy">
                <h2 :id="`onboarding-title-${stepIndex}`">{{ currentStep.title }}</h2>
                <p :id="`onboarding-description-${stepIndex}`">{{ currentStep.description }}</p>
              </div>
            </Transition>
          </header>

          <div class="onboarding-preview">
            <Transition :name="direction === 'forward' ? 'step-forward' : 'step-backward'" mode="out-in">
              <OnboardingFeaturePreview
                :key="currentStep.kind"
                class="onboarding-preview__content"
                :kind="currentStep.kind"
                :theme-mode="themeMode"
                :shortcut="shortcut"
                :has-folders="hasFolders"
                @add-folder="emit('addFolder')"
                @theme-change="emit('themeChange', $event)"
              />
            </Transition>
          </div>

          <footer class="onboarding-footer">
            <Button
              class="onboarding-never"
              variant="ghost"
              size="sm"
              :pressed="neverAskAgain"
              @click="emit('neverAskAgain', !neverAskAgain)"
            >
              <template #leading>
                <Check v-if="neverAskAgain" :size="14" />
                <EyeOff v-else :size="14" />
              </template>
              {{ t('onboarding.dont_show_again') }}
            </Button>

            <div class="onboarding-progress" :aria-label="t('onboarding.step_of', { current: stepIndex + 1, total: steps.length })">
              <Button
                v-for="(_, index) in steps"
                :key="index"
                class="onboarding-dot"
                :class="{ 'onboarding-dot--active': index === stepIndex, 'onboarding-dot--done': index < stepIndex }"
                variant="ghost"
                size="icon"
                :depth="false"
                :aria-label="t('onboarding.go_to_step', { number: index + 1 })"
                @click="goTo(index)"
              />
            </div>

            <div class="onboarding-actions">
              <Button v-if="!isFirst" variant="secondary" size="md" @click="goTo(stepIndex - 1)">
                <template #leading><ArrowLeft :size="15" /></template>{{ t('onboarding.back') }}
              </Button>
              <Button v-if="currentStep.kind === 'indexing' && !hasFolders" variant="secondary" size="md" @click="emit('addFolder')">
                <template #leading><FolderPlus :size="15" /></template>{{ t('onboarding.add_folder') }}
              </Button>
              <Button variant="primary" size="md" @click="next">
                <template #leading><PartyPopper v-if="isLast" :size="15" /></template>
                {{ isLast ? t('onboarding.get_started') : t('onboarding.next') }}
                <template #trailing><ArrowRight v-if="!isLast" :size="15" /></template>
              </Button>
            </div>
          </footer>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.onboarding-overlay {
  position: fixed;
  z-index: 120;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 28px;
  background: color-mix(in srgb, var(--background) 34%, rgb(2 6 23 / 0.58));
  backdrop-filter: blur(14px) saturate(0.92);
}
.onboarding-dialog {
  position: relative;
  display: grid;
  grid-template-rows: 102px minmax(0, 1fr) 44px;
  gap: 18px;
  width: min(920px, calc(100vw - 56px));
  height: min(760px, calc(100vh - 56px));
  min-height: min(620px, calc(100vh - 56px));
  padding: 26px;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--border-strong) 78%, transparent);
  border-radius: 26px;
  background:
    radial-gradient(circle at 12% 0%, color-mix(in srgb, var(--primary) 7%, transparent), transparent 30%),
    color-mix(in srgb, var(--surface-elevated) 97%, transparent);
  box-shadow:
    inset 0 1px rgb(255 255 255 / 0.14),
    0 38px 110px -48px rgb(2 6 23 / 0.72),
    0 12px 34px -24px rgb(2 6 23 / 0.38);
  animation: onboarding-pop 380ms cubic-bezier(0.16, 1, 0.3, 1) both;
}
.onboarding-close { position: absolute; z-index: 3; top: 17px; right: 17px; }
.onboarding-header { min-height: 0; padding-right: 48px; overflow: hidden; }
.onboarding-copy h2 { margin: 3px 0 0; color: var(--text); font-size: clamp(24px, 3vw, 34px); line-height: 1.08; letter-spacing: -0.04em; }
.onboarding-copy p { max-width: 790px; margin: 11px 0 0; color: var(--text-muted); font-size: 13px; line-height: 1.55; }
.onboarding-preview { min-height: 0; overflow: hidden; }
.onboarding-preview__content { height: 100%; }
.onboarding-footer { display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr); align-items: center; gap: 16px; min-width: 0; }
.onboarding-never { justify-self: start; }
.onboarding-progress { display: flex; align-items: center; justify-content: center; gap: 6px; }
.onboarding-dot {
  width: 7px;
  min-width: 7px;
  height: 7px;
  min-height: 7px;
  padding: 0;
  border: 0;
  border-radius: 99px;
  background: var(--border-strong);
  color: transparent;
  overflow: visible;
  transition: width 220ms cubic-bezier(0.16, 1, 0.3, 1), min-width 220ms cubic-bezier(0.16, 1, 0.3, 1), background-color 160ms ease, transform 160ms ease;
}
.onboarding-dot:hover { transform: scale(1.25); background: color-mix(in srgb, var(--primary) 42%, var(--border)); }
.onboarding-dot--done { background: color-mix(in srgb, var(--primary) 56%, var(--border)); }
.onboarding-dot--active { width: 22px; min-width: 22px; background: var(--primary); }
.onboarding-actions { display: flex; align-items: center; justify-content: flex-end; gap: 8px; }
.onboarding-overlay-enter-active,
.onboarding-overlay-leave-active { transition: opacity 220ms ease, backdrop-filter 220ms ease; }
.onboarding-overlay-enter-from,
.onboarding-overlay-leave-to { opacity: 0; backdrop-filter: blur(0); }
.onboarding-overlay-leave-active .onboarding-dialog { animation: onboarding-out 190ms cubic-bezier(0.4, 0, 1, 1) both; }
.step-forward-enter-active,
.step-forward-leave-active,
.step-backward-enter-active,
.step-backward-leave-active { transition: opacity 170ms ease, transform 240ms cubic-bezier(0.16, 1, 0.3, 1), filter 170ms ease; }
.step-forward-enter-from { opacity: 0; transform: translateX(18px) scale(0.99); filter: blur(5px); }
.step-forward-leave-to { opacity: 0; transform: translateX(-14px) scale(0.992); filter: blur(4px); }
.step-backward-enter-from { opacity: 0; transform: translateX(-18px) scale(0.99); filter: blur(5px); }
.step-backward-leave-to { opacity: 0; transform: translateX(14px) scale(0.992); filter: blur(4px); }
@keyframes onboarding-pop {
  0% { opacity: 0; transform: translateY(18px) scale(0.94); filter: blur(10px); }
  70% { opacity: 1; transform: translateY(-1px) scale(1.004); filter: blur(0); }
  100% { opacity: 1; transform: none; filter: none; }
}
@keyframes onboarding-out { to { opacity: 0; transform: translateY(10px) scale(0.975); filter: blur(6px); } }
:global(:root[data-theme='dark']) .onboarding-overlay { background: rgb(0 0 0 / 0.62); }
:global(:root[data-theme='dark']) .onboarding-dialog { box-shadow: inset 0 1px rgb(255 255 255 / 0.07), 0 42px 120px -48px rgb(0 0 0 / 0.92); }
@media (max-width: 820px) {
  .onboarding-overlay { padding: 14px; }
  .onboarding-dialog {
    grid-template-rows: 112px minmax(0, 1fr) auto;
    width: calc(100vw - 28px);
    height: calc(100vh - 28px);
    min-height: 0;
    padding: 20px;
    border-radius: 22px;
  }
  .onboarding-footer { grid-template-columns: 1fr auto; }
  .onboarding-progress { grid-column: 1 / -1; grid-row: 1; }
  .onboarding-never { grid-row: 2; }
  .onboarding-actions { grid-row: 2; }
}
@media (prefers-reduced-motion: reduce) {
  .onboarding-dialog,
  .onboarding-overlay-leave-active .onboarding-dialog { animation-duration: 0.01ms; }
  .step-forward-enter-active,
  .step-forward-leave-active,
  .step-backward-enter-active,
  .step-backward-leave-active,
  .onboarding-overlay-enter-active,
  .onboarding-overlay-leave-active { transition-duration: 0.01ms; }
}
</style>
