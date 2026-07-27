import { onBeforeUnmount, ref } from 'vue'
import { imagyxApi } from '../../api/tauri'
import type { ImageAsset } from '../../types'

const MINIMUM_FEEDBACK_MS = 320
const OPEN_FEEDBACK_MS = 240
const SUCCESS_DURATION_MS = 1800

export function useSpotlightResultActions(reportError: (error: unknown) => void) {
  const copiedImageId = ref<string | null>(null)
  const copyingImageId = ref<string | null>(null)
  const revealingImageId = ref<string | null>(null)
  const openingImageId = ref<string | null>(null)
  let copiedTimer: number | undefined

  async function copyImage(image: ImageAsset) {
    if (copyingImageId.value === image.id) return
    copyingImageId.value = image.id
    try {
      await withMinimumFeedback(imagyxApi.copyImage(image.path))
      copiedImageId.value = image.id
      if (copiedTimer) window.clearTimeout(copiedTimer)
      copiedTimer = window.setTimeout(() => {
        if (copiedImageId.value === image.id) copiedImageId.value = null
      }, SUCCESS_DURATION_MS)
    } catch (error) {
      reportError(error)
    } finally {
      if (copyingImageId.value === image.id) copyingImageId.value = null
    }
  }

  async function revealImage(image: ImageAsset) {
    if (revealingImageId.value === image.id) return
    revealingImageId.value = image.id
    try {
      await withMinimumFeedback(imagyxApi.openInFileManager(image.path, true))
    } catch (error) {
      reportError(error)
    } finally {
      if (revealingImageId.value === image.id) revealingImageId.value = null
    }
  }

  async function openImage(image: ImageAsset) {
    if (openingImageId.value === image.id) return
    openingImageId.value = image.id
    try {
      await delay(OPEN_FEEDBACK_MS)
      await imagyxApi.openInImagyx(image.id)
    } catch (error) {
      reportError(error)
      if (openingImageId.value === image.id) openingImageId.value = null
    }
  }

  function resetActionFeedback() {
    copiedImageId.value = null
    copyingImageId.value = null
    revealingImageId.value = null
    openingImageId.value = null
    if (copiedTimer) window.clearTimeout(copiedTimer)
    copiedTimer = undefined
  }

  onBeforeUnmount(resetActionFeedback)

  return {
    copiedImageId,
    copyingImageId,
    revealingImageId,
    openingImageId,
    copyImage,
    revealImage,
    openImage,
    resetActionFeedback,
  }
}

async function withMinimumFeedback(operation: Promise<void>) {
  const startedAt = performance.now()
  await operation
  const remaining = MINIMUM_FEEDBACK_MS - (performance.now() - startedAt)
  if (remaining > 0) await delay(remaining)
}

function delay(duration: number) {
  return new Promise<void>((resolve) => window.setTimeout(resolve, duration))
}
