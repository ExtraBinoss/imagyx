import { defineStore } from 'pinia'

export type ToastKind = 'info' | 'success' | 'error'

export interface ToastItem {
  id: string
  title: string
  description?: string
  kind?: ToastKind
  progress?: number
  progressLabel?: string
  persistent?: boolean
  duration?: number
}

const timers = new Map<string, ReturnType<typeof setTimeout>>()

export const useToastStore = defineStore('toasts', {
  state: () => ({
    items: [] as ToastItem[],
  }),

  actions: {
    upsert(toast: ToastItem) {
      const normalized: ToastItem = {
        kind: 'info',
        duration: 4200,
        ...toast,
      }
      const index = this.items.findIndex((item) => item.id === toast.id)
      if (index === -1) this.items.push(normalized)
      else this.items[index] = normalized

      const existingTimer = timers.get(toast.id)
      if (existingTimer) clearTimeout(existingTimer)
      timers.delete(toast.id)

      if (!normalized.persistent) {
        const timer = setTimeout(() => this.dismiss(toast.id), normalized.duration)
        timers.set(toast.id, timer)
      }
    },

    dismiss(id: string) {
      this.items = this.items.filter((item) => item.id !== id)
      const timer = timers.get(id)
      if (timer) clearTimeout(timer)
      timers.delete(id)
    },
  },
})
