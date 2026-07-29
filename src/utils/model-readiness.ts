import type { ModelDownloadProgress, RuntimeStats } from '../types'

const MODEL_PREPARING_STAGES = new Set(['checking', 'downloading', 'loading'])
const RUNTIME_PREPARING_STAGES = new Set(['checking', 'loading', 'loading-text'])

export function isModelDownloading(progress: ModelDownloadProgress | null): boolean {
  return progress?.stage === 'downloading'
}

/** True only while the model cannot yet answer a semantic query. */
export function isModelPreparing(
  progress: ModelDownloadProgress | null,
  runtimeStats: RuntimeStats | null,
): boolean {
  return MODEL_PREPARING_STAGES.has(progress?.stage ?? '')
    || RUNTIME_PREPARING_STAGES.has(runtimeStats?.stage ?? '')
}
