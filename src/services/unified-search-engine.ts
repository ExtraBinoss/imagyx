import { spotlightSearchTiming } from '../config/spotlight-search'
import { semanticRuntime } from './semantic'
import { visualSearchSession } from './visual-search-session'

let installed = false
let mainEmbeddingSequence = 0

function waitForDelay(delayMs: number): Promise<void> {
  if (delayMs <= 0) return Promise.resolve()
  return new Promise((resolve) => window.setTimeout(resolve, delayMs))
}

function installMainVisualEscape() {
  if ((document.documentElement.dataset.window ?? 'main') === 'spotlight') return
  window.addEventListener('keydown', (event) => {
    if (event.isComposing || event.key !== 'Escape' || visualSearchSession.state.status === 'idle') return
    const backButton = document.querySelector<HTMLButtonElement>(
      '.unified-search-input--visual .unified-search-input__nav',
    )
    if (!backButton || backButton.disabled) return
    event.preventDefault()
    event.stopImmediatePropagation()
    backButton.click()
  }, { capture: true })
}

export function installUnifiedSearchEngine(): void {
  if (installed || typeof window === 'undefined') return
  installed = true
  installMainVisualEscape()

  const embedTextQuery = semanticRuntime.embedQuery.bind(semanticRuntime)
  semanticRuntime.embedQuery = async (query: string) => {
    const trimmed = query.trim()
    // imagyxApi already turns this token into the pure visual request. The
    // semantic stage must stop here so no text model or duplicate search runs.
    if (visualSearchSession.vectorForQuery(trimmed)) {
      mainEmbeddingSequence += 1
      return undefined
    }

    const surface = document.documentElement.dataset.window ?? 'main'
    if (surface !== 'spotlight') {
      mainEmbeddingSequence += 1
      const sequence = mainEmbeddingSequence
      const remainingDelay = Math.max(
        0,
        spotlightSearchTiming.semanticDebounceMs - spotlightSearchTiming.lexicalDebounceMs,
      )
      await waitForDelay(remainingDelay)
      if (sequence !== mainEmbeddingSequence) return undefined
    }

    return embedTextQuery(trimmed)
  }
}
