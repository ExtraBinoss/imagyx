const PERF_SAMPLE_LIMIT = 256

interface PerfBucket {
  samples: number[]
  totalSamples: number
}

export interface PerfMetricSnapshot {
  metric: string
  samples: number
  p50Ms: number
  p95Ms: number
  maxMs: number
  lastMs: number
}

const perfBuckets = new Map<string, PerfBucket>()

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

function percentile(sorted: number[], value: number): number {
  if (!sorted.length) return 0
  const index = Math.round((sorted.length - 1) * Math.min(1, Math.max(0, value)))
  return sorted[index] ?? 0
}

function recordPerf(component: string, action: string, durationMs: number): PerfBucket {
  const key = `${component}.${action}`
  const bucket = perfBuckets.get(key) ?? { samples: [], totalSamples: 0 }
  bucket.totalSamples += 1
  bucket.samples.push(durationMs)
  if (bucket.samples.length > PERF_SAMPLE_LIMIT) bucket.samples.shift()
  perfBuckets.set(key, bucket)
  return bucket
}

export function perfSnapshot(): PerfMetricSnapshot[] {
  return [...perfBuckets.entries()]
    .map(([metric, bucket]) => {
      const sorted = [...bucket.samples].sort((left, right) => left - right)
      return {
        metric,
        samples: bucket.totalSamples,
        p50Ms: Number(percentile(sorted, 0.5).toFixed(1)),
        p95Ms: Number(percentile(sorted, 0.95).toFixed(1)),
        maxMs: Number((sorted.at(-1) ?? 0).toFixed(1)),
        lastMs: Number((bucket.samples.at(-1) ?? 0).toFixed(1)),
      }
    })
    .sort((left, right) => right.p95Ms - left.p95Ms)
}

export function perfSample(
  component: string,
  action: string,
  durationMs: number,
): void {
  if (!import.meta.env.DEV) return
  const bucket = recordPerf(component, action, durationMs)
  if (bucket.totalSamples !== 1 && bucket.totalSamples % 16 !== 0) return
  const metric = perfSnapshot().find((item) => item.metric === `${component}.${action}`)
  if (metric) console.log('[PerfSummary]', metric)
}

export function perfLog(component: string, action: string, durationMs: number, details?: unknown) {
  if (import.meta.env.DEV) {
    recordPerf(component, action, durationMs)
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

export function installPerformanceDiagnostics(): void {
  if (!import.meta.env.DEV) return
  const startedAt = performance.now()
  requestAnimationFrame(() => {
    perfLog('Startup', 'first animation frame', performance.now() - startedAt)
  })

  if (PerformanceObserver.supportedEntryTypes?.includes('longtask')) {
    const observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        perfSample('Browser', 'long task', entry.duration)
      }
    })
    observer.observe({ entryTypes: ['longtask'] })
  }

  const diagnostics = {
    snapshot: perfSnapshot,
    print: () => console.table(perfSnapshot()),
    json: () => JSON.stringify(perfSnapshot(), null, 2),
    reset: () => perfBuckets.clear(),
  }
  Object.defineProperty(window, '__IMAGYX_PERF__', {
    configurable: true,
    value: diagnostics,
  })
  window.setTimeout(() => {
    console.info(
      '[Perf] Rapport disponible avec window.__IMAGYX_PERF__.print() ou window.__IMAGYX_PERF__.json()',
    )
    diagnostics.print()
  }, 5_000)
}
