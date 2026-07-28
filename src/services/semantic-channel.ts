import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { perfLog } from '../utils'
import type { EmbeddedQuery } from './semantic'

export const SEMANTIC_QUERY_REQUEST_EVENT = 'semantic-query-requested'

export interface SemanticQueryRequest {
  requestId: string
  query: string
  replyTo: 'main' | 'spotlight'
  replyEvent: string
}

interface SemanticQueryResponse {
  requestId: string
  result?: EmbeddedQuery
  error?: string
}

export async function requestSemanticEmbedding(
  query: string,
  timeoutMs = 10_000,
): Promise<EmbeddedQuery | undefined> {
  const requestId = crypto.randomUUID()
  const replyEvent = `semantic-query-response:${requestId}`
  const startedAt = import.meta.env.DEV ? performance.now() : 0
  let unlisten: UnlistenFn | null = null
  let timeout: number | undefined
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
    const listenerStartedAt = import.meta.env.DEV ? performance.now() : 0
    unlisten = await listen<SemanticQueryResponse>(replyEvent, (event) => {
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
    unlisten?.()
  }
}
