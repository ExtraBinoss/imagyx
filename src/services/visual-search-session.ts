import { readonly, shallowReactive } from 'vue'
import type { ImageAsset, SearchRequest } from '../types'

const VISUAL_QUERY_PREFIX = '\u2063imagyx-visual:'
const SESSION_EVENT = 'imagyx-visual-search-session'

export type VisualSearchSourceKind = 'indexed' | 'file' | 'clipboard' | 'drop'
export type VisualSearchStatus = 'idle' | 'loading' | 'ready' | 'error'

interface VisualSearchState {
  status: VisualSearchStatus
  token: string
  sourceKind: VisualSearchSourceKind | null
  label: string
  previewUrl: string | null
  sourceImage: ImageAsset | null
  vector: number[] | null
  excludeImageId: string | null
  error: string | null
}

const state = shallowReactive<VisualSearchState>({
  status: 'idle',
  token: '',
  sourceKind: null,
  label: '',
  previewUrl: null,
  sourceImage: null,
  vector: null,
  excludeImageId: null,
  error: null,
})

let sessionSequence = 0
let operationSequence = 0
let ownedPreviewUrl: string | null = null

function dispatchSessionEvent() {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent(SESSION_EVENT, {
    detail: {
      status: state.status,
      token: state.token,
    },
  }))
}

function releaseOwnedPreview() {
  if (!ownedPreviewUrl) return
  URL.revokeObjectURL(ownedPreviewUrl)
  ownedPreviewUrl = null
}

function begin(
  sourceKind: VisualSearchSourceKind,
  label: string,
  options: { previewUrl?: string | null; sourceImage?: ImageAsset | null } = {},
): number {
  operationSequence += 1
  releaseOwnedPreview()
  state.status = 'loading'
  state.token = ''
  state.sourceKind = sourceKind
  state.label = label
  state.previewUrl = options.previewUrl ?? null
  state.sourceImage = options.sourceImage ?? null
  state.vector = null
  state.excludeImageId = null
  state.error = null
  dispatchSessionEvent()
  return operationSequence
}

function activate(
  vector: number[],
  options: {
    sourceKind: VisualSearchSourceKind
    label: string
    previewUrl?: string | null
    sourceImage?: ImageAsset | null
    excludeImageId?: string | null
    ownsPreviewUrl?: boolean
  },
  operationId = operationSequence,
): string {
  if (operationId !== operationSequence) return ''
  if (!vector.length) throw new Error('Visual search embedding is empty')
  if (options.ownsPreviewUrl && options.previewUrl) ownedPreviewUrl = options.previewUrl
  sessionSequence += 1
  state.status = 'ready'
  state.token = `${VISUAL_QUERY_PREFIX}${Date.now().toString(36)}-${sessionSequence}`
  state.sourceKind = options.sourceKind
  state.label = options.label
  state.previewUrl = options.previewUrl ?? null
  state.sourceImage = options.sourceImage ?? null
  state.vector = vector
  state.excludeImageId = options.excludeImageId ?? null
  state.error = null
  dispatchSessionEvent()
  return state.token
}

function fail(reason: unknown, operationId = operationSequence) {
  if (operationId !== operationSequence) return
  if (import.meta.env.DEV) console.error('[Imagyx][VisualSearch] query failed', reason)
  state.status = 'error'
  state.vector = null
  state.token = ''
  // The shared input renders the localized error description. Keep technical
  // backend details in development logs instead of leaking English strings.
  state.error = null
  dispatchSessionEvent()
}

function clear() {
  operationSequence += 1
  releaseOwnedPreview()
  state.status = 'idle'
  state.token = ''
  state.sourceKind = null
  state.label = ''
  state.previewUrl = null
  state.sourceImage = null
  state.vector = null
  state.excludeImageId = null
  state.error = null
  dispatchSessionEvent()
}

function isVisualQuery(query: string): boolean {
  return Boolean(state.token) && query === state.token
}

function requestForActiveSession(request: SearchRequest): SearchRequest {
  if (!isVisualQuery(request.query) || state.status !== 'ready' || !state.vector) return request
  return {
    ...request,
    query: '',
    queryVector: state.vector,
    mode: 'visual',
    excludeImageId: state.excludeImageId ?? undefined,
  }
}

function vectorForQuery(query: string): number[] | null {
  return isVisualQuery(query) && state.status === 'ready' ? state.vector : null
}

export const visualSearchSession = {
  state: readonly(state),
  eventName: SESSION_EVENT,
  begin,
  activate,
  fail,
  clear,
  isVisualQuery,
  requestForActiveSession,
  vectorForQuery,
}
