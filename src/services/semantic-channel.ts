import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { imagyxApi } from '../api/tauri'
import { perfLog } from '../utils'
import type { EmbeddedQuery } from './semantic'

export const SEMANTIC_QUERY_REQUEST_EVENT = 'semantic-query-requested'

export interface SemanticQueryRequest {
  requestId: string
  query: string
  replyTo: 'main' | 'spotlight'
  replyEvent: string
}

const PROVIDER_READY_POLL_MS = 25
const REQUEST_RETRY_MS = 250
const RESPONSE_TIMEOUT_MS = 10_000

async function waitForSemanticProvider(timeoutMs: number): Promise<void> {
  const startedAt = performance.now()
  while (performance.now() - startedAt < timeoutMs) {
    if (await imagyxApi.semanticProviderReady()) return
    await new Promise<void>((resolve) => window.setTimeout(resolve, PROVIDER_READY_POLL_MS))
  }
  throw new Error('Le moteur de recherche sémantique est encore en cours de démarrage')
}

interface SemanticQueryResponse {
  requestId: string
  status?: 'accepted'
  result?: EmbeddedQuery
  error?: string
}

export async function requestSemanticEmbedding(
  query: string,
  timeoutMs = RESPONSE_TIMEOUT_MS,
): Promise<EmbeddedQuery | undefined> {
  const requestId = crypto.randomUUID()
  const replyEvent = `semantic-query-response:${requestId}`
  const startedAt = import.meta.env.DEV ? performance.now() : 0
  let unlisten: UnlistenFn | null = null
  let timeout: number | undefined
  let retry: number | undefined
  let acknowledged = false
  let resolveResponse: (value: EmbeddedQuery | undefined) => void = () => undefined
  let rejectResponse: (reason?: unknown) => void = () => undefined
  const response = new Promise<EmbeddedQuery | undefined>((resolve, reject) => {
    resolveResponse = resolve
    rejectResponse = reject
  })

  if (import.meta.env.DEV) {
    console.info(`[Imagyx][SemanticQuery][${requestId}] start`, { query, timeoutMs })
  }

  try {
    // Spotlight and the main window start independently. Do not emit before the
    // main window has installed its reply listener: Tauri events are not queued
    // for listeners that do not yet exist.
    await waitForSemanticProvider(timeoutMs)
    const listenerStartedAt = import.meta.env.DEV ? performance.now() : 0
    unlisten = await listen<SemanticQueryResponse>(replyEvent, (event) => {
      if (event.payload.requestId !== requestId) return
      if (event.payload.status === 'accepted') {
        acknowledged = true
        if (retry) window.clearInterval(retry)
        return
      }
      if (import.meta.env.DEV) {
        const responseMs = performance.now() - startedAt
        perfLog('SemanticQuery', 'Spotlight response event', responseMs, {
          requestId,
          query,
          responseRequestId: event.payload.requestId,
          vectorDimensions: event.payload.result?.queryVector.length ?? 0,
          concepts: event.payload.result?.concepts.length ?? 0,
          error: event.payload.error,
        })
      }
      if (event.payload.error) rejectResponse(new Error(event.payload.error))
      else resolveResponse(event.payload.result)
    })
    if (import.meta.env.DEV) {
      perfLog('SemanticQuery', 'reply listener setup', performance.now() - listenerStartedAt, {
        requestId,
        query,
      })
    }

    timeout = window.setTimeout(
      () => rejectResponse(new Error('Le moteur de recherche sémantique ne répond pas')),
      timeoutMs,
    )

    const dispatch = async () => {
      if (acknowledged) return
      const dispatchStartedAt = import.meta.env.DEV ? performance.now() : 0
      await emitTo('main', SEMANTIC_QUERY_REQUEST_EVENT, {
        requestId,
        query,
        replyTo: 'spotlight',
        replyEvent,
      } satisfies SemanticQueryRequest)
      if (import.meta.env.DEV) {
        perfLog('SemanticQuery', 'Spotlight to main dispatch', performance.now() - dispatchStartedAt, {
          requestId,
          query,
        })
      }
    }
    await dispatch()
    // Tauri drops events sent before a listener exists. Keep sending this
    // idempotent request until the main window acknowledges receipt.
    retry = window.setInterval(() => { void dispatch() }, REQUEST_RETRY_MS)

    const result = await response
    if (import.meta.env.DEV) {
      const totalMs = performance.now() - startedAt
      perfLog('SemanticQuery', 'Spotlight embedding round trip', totalMs, {
        requestId,
        query,
        vectorDimensions: result?.queryVector.length ?? 0,
        concepts: result?.concepts.length ?? 0,
      })
      console.info(`[Imagyx][SemanticQuery][${requestId}] complete in ${totalMs.toFixed(1)} ms`, {
        query,
        vectorDimensions: result?.queryVector.length ?? 0,
        concepts: result?.concepts.length ?? 0,
      })
    }
    return result
  } catch (error) {
    if (import.meta.env.DEV) {
      console.error(
        `[Imagyx][SemanticQuery][${requestId}] failed after ${(performance.now() - startedAt).toFixed(1)} ms`,
        { query, error },
      )
    }
    throw error
  } finally {
    if (timeout) window.clearTimeout(timeout)
    if (retry) window.clearInterval(retry)
    unlisten?.()
  }
}
