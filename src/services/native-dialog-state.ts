let activeDialogs = 0

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
