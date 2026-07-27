import { onBeforeUnmount, ref } from 'vue'
import { imagyxApi } from '../../api/tauri'
import type { ImageAsset } from '../../types'
import { perfLog } from '../../utils'

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
    const startedAt = performance.now()
    try {
      await imagyxApi.copyImage(image.path)
      perfLog('Spotlight', 'copy image', performance.now() - startedAt, {
        sizeBytes: image.sizeBytes,
      })
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

function delay(duration: number) {
  return new Promise<void>((resolve) => window.setTimeout(resolve, duration))
}
