<script setup lang="ts">
import { AlertCircle, CheckCircle2, Info, X } from '@lucide/vue'
import { useToastStore } from '../../../stores/toasts'
import { useTranslate } from '../../../i18n'
import Button from '../Button/Button.vue'
import ProgressBar from '../ProgressBar/ProgressBar.vue'

const { t } = useTranslate()
const toasts = useToastStore()
</script>

<template>
  <div class="toast-viewport" aria-live="polite" aria-relevant="additions removals">
    <TransitionGroup name="toast" tag="div" class="toast-stack">
      <article v-for="toast in toasts.items" :key="toast.id" class="toast" :data-kind="toast.kind">
        <span class="toast__icon" aria-hidden="true">
          <CheckCircle2 v-if="toast.kind === 'success'" :size="18" />
          <AlertCircle v-else-if="toast.kind === 'error'" :size="18" />
          <Info v-else :size="18" />
        </span>

        <div class="toast__body">
          <div class="toast__heading">
            <strong>{{ toast.title }}</strong>
            <Button
              variant="ghost"
              size="icon"
              :aria-label="t('toast.dismiss')"
              @click="toasts.dismiss(toast.id)"
            >
              <X :size="15" />
            </Button>
          </div>
          <p v-if="toast.description">{{ toast.description }}</p>
          <div v-if="toast.progress !== undefined" class="toast__progress">
            <ProgressBar :value="toast.progress" size="sm" :label="toast.title" />
            <span v-if="toast.progressLabel">{{ toast.progressLabel }}</span>
          </div>
        </div>
      </article>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-viewport {
  position: fixed;
  right: var(--space-5);
  bottom: var(--space-5);
  z-index: var(--z-toast);
  width: min(390px, calc(100vw - 32px));
  pointer-events: none;
}

.toast-stack {
  display: grid;
  gap: var(--space-3);
}

.toast {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: var(--space-3);
  padding: var(--space-4);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  background: var(--surface-elevated);
  color: var(--text);
  box-shadow: var(--shadow-toast);
  pointer-events: auto;
}

.toast__icon {
  display: inline-flex;
  margin-top: 2px;
  color: var(--primary-text);
}

.toast[data-kind='success'] .toast__icon {
  color: var(--success-text);
}

.toast[data-kind='error'] .toast__icon {
  color: var(--danger-text);
}

.toast__body,
.toast__heading {
  min-width: 0;
}

.toast__heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}

.toast__heading strong {
  font-size: var(--text-sm);
  font-weight: 650;
}

.toast__heading :deep(.ui-button) {
  width: 26px;
  height: 26px;
}

.toast p {
  margin: var(--space-1) 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.toast__progress {
  display: grid;
  gap: var(--space-2);
  margin-top: var(--space-3);
}

.toast__progress span {
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-variant-numeric: tabular-nums;
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 180ms ease,
    transform 180ms ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
