import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { semanticRuntime } from './semantic'
import {
  SEMANTIC_QUERY_REQUEST_EVENT,
  type SemanticQueryRequest,
} from './semantic-channel'

export function registerSemanticQueryProvider(): Promise<UnlistenFn> {
  return listen<SemanticQueryRequest>(SEMANTIC_QUERY_REQUEST_EVENT, (event) => {
    const request = event.payload
    const target = request.replyTo === 'spotlight' ? 'spotlight' : 'main'
    void semanticRuntime
      .embedQuery(request.query)
      .then((result) =>
        emitTo(target, request.replyEvent, {
          requestId: request.requestId,
          result,
        }),
      )
      .catch((error) =>
        emitTo(target, request.replyEvent, {
          requestId: request.requestId,
          error: String(error),
        }),
      )
  })
}
