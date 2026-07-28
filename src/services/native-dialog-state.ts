let activeDialogs = 0
let installed = false
let releaseUploadDialog: (() => void) | null = null

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

export function installNativeDialogFocusGuard(): void {
  if (installed || typeof document === 'undefined') return
  installed = true

  document.addEventListener('click', (event) => {
    const target = event.target
    if (!(target instanceof Element) || !target.closest('.unified-search-input__upload')) return

    releaseUploadDialog?.()
    releaseUploadDialog = beginNativeDialog()
    window.addEventListener('focus', () => {
      window.setTimeout(() => {
        releaseUploadDialog?.()
        releaseUploadDialog = null
      }, 0)
    }, { once: true })
  }, { capture: true })
}
