import type { IndexProgress, RuntimeStats } from '../types'
import { formatBytes } from '../utils'

function formatMs(value?: number) {
  if (!value) return '0 ms'
  if (value < 1_000) return `${Math.round(value)} ms`
  return `${(value / 1_000).toFixed(1)} s`
}

function formatLastIndexed(iso?: string | null) {
  if (!iso) return 'Never'
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

export interface DebugInfoOptions {
  appVersion: string
  platform: string
  databasePath?: string
  shortcut?: string
  themeMode?: string
  controlsPosition?: string
  runtimeStats?: RuntimeStats | null
  progress?: IndexProgress | null
  lastIndexedAt?: string | null
  foldersCount?: number
  totalImagesCount?: number
  phaseLabel?: string
  currentProgress?: number
  totalProgress?: number
}

export function buildDebugInfoText(options: DebugInfoOptions): string {
  const stats = options.runtimeStats
  const dbPath = options.databasePath ?? 'Unavailable'
  const phase = options.phaseLabel ?? (stats?.stage ?? 'Ready')
  const current = options.currentProgress ?? stats?.current ?? 0
  const total = options.totalProgress ?? stats?.total ?? 0
  const speedStr =
    stats?.imagesPerSecond != null && stats.imagesPerSecond > 0
      ? `${stats.imagesPerSecond.toFixed(1)} img/s (${stats.averageMsPerImage ? `${Math.round(stats.averageMsPerImage)} ms/img` : '—'})`
      : '0.0 img/s'

  return `=== System Information ===
App Version: v${options.appVersion}
OS Platform: ${options.platform}
Window Controls: ${options.controlsPosition ?? 'left'}
Database Location: ${dbPath}${options.shortcut ? `\nShortcut: ${options.shortcut}` : ''}${options.themeMode ? `\nTheme Mode: ${options.themeMode}` : ''}

=== Indexing Information ===
Last Indexing: ${formatLastIndexed(options.lastIndexedAt)}
AI Model: ${stats?.modelName ?? 'MobileCLIP-S0'}
Phase: ${phase}
Hardware Accelerated: ${stats?.accelerationActive ? 'OK' : 'KO'}
Backend: ${stats?.backendEffective ?? 'Automatic'} (Requested: ${stats?.backendRequested ?? 'WebGPU'})
${stats?.batchTotal ? `Batch: ${stats.batchCurrent ?? 0} / ${stats.batchTotal} (${stats.batchSize ?? 0} img)\n` : ''}Progress: ${current} / ${total}
Throughput: ${speedStr}
Timings: Decode: ${formatMs(stats?.decodeMs)} | Inference: ${formatMs(stats?.inferenceMs)} | SQLite: ${formatMs(stats?.saveMs)}
Resources: App CPU: ${stats?.processCpuPercent?.toFixed(0) ?? 0}% | System CPU: ${stats?.systemCpuPercent?.toFixed(0) ?? 0}% | RAM: ${formatBytes(stats?.processMemoryBytes ?? 0)}${options.foldersCount != null ? `\nLibrary: ${options.foldersCount} watched folder(s) (${options.totalImagesCount ?? 0} images)` : ''}`
}

export async function copyDebugInfoToClipboard(options: DebugInfoOptions): Promise<void> {
  const text = buildDebugInfoText(options)
  await navigator.clipboard.writeText(text)
}
