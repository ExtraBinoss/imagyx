const DEFAULT_LEXICAL_DEBOUNCE_MS = 25
const DEFAULT_SEMANTIC_DEBOUNCE_MS = 80
const MAX_CONFIGURED_DEBOUNCE_MS = 1_000

function configuredDelay(rawValue: unknown, fallbackMs: number): number {
  if (typeof rawValue !== 'string' || rawValue.trim() === '') return fallbackMs
  const parsed = Number(rawValue)
  if (!Number.isFinite(parsed)) return fallbackMs
  return Math.min(MAX_CONFIGURED_DEBOUNCE_MS, Math.max(0, Math.round(parsed)))
}

const lexicalDebounceMs = configuredDelay(
  import.meta.env.VITE_SPOTLIGHT_LEXICAL_DEBOUNCE_MS,
  DEFAULT_LEXICAL_DEBOUNCE_MS,
)
const semanticDebounceMs = Math.max(
  lexicalDebounceMs,
  configuredDelay(
    import.meta.env.VITE_SPOTLIGHT_SEMANTIC_DEBOUNCE_MS,
    DEFAULT_SEMANTIC_DEBOUNCE_MS,
  ),
)

export const spotlightSearchTiming = Object.freeze({
  lexicalDebounceMs,
  semanticDebounceMs,
})
