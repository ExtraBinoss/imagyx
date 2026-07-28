import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { perfLog } from '../utils'
import { semanticRuntime } from './semantic'
import {
  SEMANTIC_QUERY_REQUEST_EVENT,
  type SemanticQueryRequest,
} from './semantic-channel'

async function handleSemanticQueryRequest(request: SemanticQueryRequest): Promise<void> {
  const startedAt = performance.now()
  const target = request.replyTo === 'spotlight' ? 'spotlight' : 'main'

  if (import.meta.env.DEV) {
    console.info(`[Imagyx][SemanticProvider][${request.requestId}] received`, {
      query: request.query,
      target,
    })
  }

  try {
    const embeddingStartedAt = performance.now()
    const result = await semanticRuntime.embedQuery(request.query)
    const embeddingMs = performance.now() - embeddingStartedAt
    perfLog('SemanticProvider', 'text embedding', embeddingMs, {
      requestId: request.requestId,
      query: request.query,
      vectorDimensions: result?.queryVector.length ?? 0,
      concepts: result?.concepts.length ?? 0,
    })

    const replyStartedAt = performance.now()
    await emitTo(target, request.replyEvent, {
      requestId: request.requestId,
      result,
    })
    const replyMs = performance.now() - replyStartedAt
    perfLog('SemanticProvider', 'main to Spotlight reply', replyMs, {
      requestId: request.requestId,
      query: request.query,
      target,
    })

    if (import.meta.env.DEV) {
      console.info(
        `[Imagyx][SemanticProvider][${request.requestId}] complete in ${(performance.now() - startedAt).toFixed(1)} ms`,
        {
          query: request.query,
          embeddingMs: Number(embeddingMs.toFixed(2)),
          replyMs: Number(replyMs.toFixed(2)),
          vectorDimensions: result?.queryVector.length ?? 0,
        },
      )
    }
  } catch (error) {
    const failedAt = performance.now()
    await emitTo(target, request.replyEvent, {
      requestId: request.requestId,
      error: String(error),
    })
    if (import.meta.env.DEV) {
      console.error(
        `[Imagyx][SemanticProvider][${request.requestId}] failed after ${(failedAt - startedAt).toFixed(1)} ms`,
        { query: request.query, error },
      )
    }
  }
}

export function registerSemanticQueryProvider(): Promise<UnlistenFn> {
  return listen<SemanticQueryRequest>(SEMANTIC_QUERY_REQUEST_EVENT, (event) => {
    void handleSemanticQueryRequest(event.payload)
  })
}
