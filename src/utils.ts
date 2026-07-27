export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`
  const units = ['Ko', 'Mo', 'Go', 'To']
  let value = bytes / 1024
  let unitIndex = 0
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`
}

export function basename(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path
}

export function debounce<Args extends unknown[]>(
  fn: (...args: Args) => void,
  delay: number,
): (...args: Args) => void {
  let timer: ReturnType<typeof setTimeout> | undefined
  return (...args: Args) => {
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => fn(...args), delay)
  }
}

export function perfLog(component: string, action: string, durationMs: number, details?: unknown) {
  if (import.meta.env.DEV) {
    const color = durationMs > 100 ? '#ef4444' : durationMs > 30 ? '#f59e0b' : '#10b981'
    console.log(
      `%c[Perf][${component}] %c${action}: %c${durationMs.toFixed(1)}ms`,
      'color: #8b5cf6; font-weight: bold;',
      'color: #94a3b8;',
      `color: ${color}; font-weight: bold;`,
      details !== undefined ? details : '',
    )
  }
}
