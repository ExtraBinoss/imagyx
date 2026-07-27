import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { EmbeddedQuery } from './semantic'

export const SEMANTIC_QUERY_REQUEST_EVENT = 'semantic-query-requested'

export interface SemanticQueryRequest {
  requestId: string
  query: string
  replyTo: 'main' | 'spotlight'
  replyEvent: string
}

interface SemanticQueryResponse {
  result?: EmbeddedQuery
  error?: string
}

export async function requestSemanticEmbedding(
  query: string,
  timeoutMs = 10_000,
): Promise<EmbeddedQuery | undefined> {
  const requestId = crypto.randomUUID()
  const replyEvent = `semantic-query-response:${requestId}`
  let unlisten: UnlistenFn | null = null
  let timeout: number | undefined
  let resolveResponse: (value: EmbeddedQuery | undefined) => void = () => undefined
  let rejectResponse: (reason?: unknown) => void = () => undefined
  const response = new Promise<EmbeddedQuery | undefined>((resolve, reject) => {
    resolveResponse = resolve
    rejectResponse = reject
  })

  try {
    unlisten = await listen<SemanticQueryResponse>(replyEvent, (event) => {
      if (event.payload.error) rejectResponse(new Error(event.payload.error))
      else resolveResponse(event.payload.result)
    })
    timeout = window.setTimeout(
      () => rejectResponse(new Error('Le moteur de recherche sémantique ne répond pas')),
      timeoutMs,
    )
    await emitTo('main', SEMANTIC_QUERY_REQUEST_EVENT, {
      requestId,
      query,
      replyTo: 'spotlight',
      replyEvent,
    } satisfies SemanticQueryRequest)
    return await response
  } finally {
    if (timeout) window.clearTimeout(timeout)
    unlisten?.()
  }
}
