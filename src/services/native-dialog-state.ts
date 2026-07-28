let activeDialogs = 0
let installed = false
let releaseUploadDialog: (() => void) | null = null
let releaseFallback: number | undefined

export function beginNativeDialog(): () => void {
  activeDialogs += 1
  let closed = false
  return () => {
    if (closed) return
    closed = true
    activeDialogs = Math.max(0, activeDialogs - 1)
  }
}

export function nativeDialogIsOpen(): boolean {
  return activeDialogs > 0
}

function releaseActiveUploadDialog(): void {
  if (releaseFallback) window.clearTimeout(releaseFallback)
  releaseFallback = undefined
  releaseUploadDialog?.()
  releaseUploadDialog = null
}

export function installNativeDialogFocusGuard(): void {
  if (installed || typeof document === 'undefined') return
  installed = true

  document.addEventListener('click', (event) => {
    const target = event.target
    if (!(target instanceof Element) || !target.closest('.unified-search-input__upload')) return

    releaseActiveUploadDialog()
    releaseUploadDialog = beginNativeDialog()
    // Focus normally returns when the system picker closes. The fallback only
    // prevents a failed native dialog from keeping Spotlight pinned forever.
    releaseFallback = window.setTimeout(releaseActiveUploadDialog, 120_000)
    window.addEventListener('focus', () => {
      window.setTimeout(releaseActiveUploadDialog, 0)
    }, { once: true })
  }, { capture: true })
}
