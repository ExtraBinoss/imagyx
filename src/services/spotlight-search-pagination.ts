import { readonly, ref, type Ref } from 'vue'
import type { ImageAsset, SearchPage, SearchRequest } from '../types'

const PAGE_CACHE_TTL_MS = 2_000
const DEFAULT_PAGE_SIZE = 60

type InitialSearchContext = {
  token: number
  rawQuery: string
  requestKey: string
}

type CachedPage = {
  request: SearchRequest
  items: ImageAsset[]
  total: number
  storedAt: number
}

type SearchPageLoader = (request: SearchRequest) => Promise<SearchPage>

const totalResults = ref(0)
const loadingMore = ref(false)
const hasKnownTotal = ref(false)
const cachedPages = new Map<string, CachedPage>()

let currentRawQuery = ''
let requestSequence = 0
let activeToken = 0
let activeRequestKey = ''
let activeRequest: SearchRequest | null = null
let activeItems: ImageAsset[] | null = null

function isSpotlightSurface(): boolean {
  return typeof document !== 'undefined'
    && document.documentElement.dataset.window === 'spotlight'
}

function requestKey(request: SearchRequest): string {
  return [
    request.mode ?? 'text',
    request.folderId ?? 'all',
    request.excludeImageId ?? '-',
    request.query.trim().toLocaleLowerCase('en'),
  ].join(':')
}

function trimCache(now = performance.now()): void {
  for (const [key, page] of cachedPages) {
    if (now - page.storedAt > PAGE_CACHE_TTL_MS) cachedPages.delete(key)
  }
}

function activate(page: CachedPage, token = activeToken): void {
  activeToken = token
  activeRequestKey = requestKey(page.request)
  activeRequest = page.request
  activeItems = page.items
  totalResults.value = page.total
  hasKnownTotal.value = true
}

function setRawQuery(rawQuery: string): void {
  if (!isSpotlightSurface()) return
  currentRawQuery = rawQuery
  trimCache()
  const cached = cachedPages.get(rawQuery)
  if (cached) {
    activate(cached)
    return
  }
  activeRequestKey = ''
  activeRequest = null
  activeItems = null
  totalResults.value = 0
  hasKnownTotal.value = false
}

function beginInitialSearch(request: SearchRequest): InitialSearchContext | null {
  if (!isSpotlightSurface()) return null
  requestSequence += 1
  const context = {
    token: requestSequence,
    rawQuery: currentRawQuery,
    requestKey: requestKey(request),
  }
  if (activeRequestKey && activeRequestKey !== context.requestKey) {
    totalResults.value = 0
    hasKnownTotal.value = false
  }
  return context
}

function completeInitialSearch(
  context: InitialSearchContext | null,
  request: SearchRequest,
  page: SearchPage,
  reactiveItems: ImageAsset[],
): void {
  if (!context) return
  const cached = {
    request: { ...request, offset: 0 },
    items: reactiveItems,
    total: page.total,
    storedAt: performance.now(),
  }
  cachedPages.set(context.rawQuery, cached)
  trimCache(cached.storedAt)

  if (context.rawQuery !== currentRawQuery || context.token < requestSequence) return
  activate(cached, context.token)

  if (import.meta.env.DEV) {
    console.info('[Imagyx][SpotlightPagination] initial page ready', {
      rawQuery: context.rawQuery,
      mode: request.mode ?? 'text',
      query: request.query,
      folderId: request.folderId ?? null,
      loaded: reactiveItems.length,
      total: page.total,
      hasMore: reactiveItems.length < page.total,
    })
  }
}

async function loadMore(loader: SearchPageLoader): Promise<void> {
  const request = activeRequest
  const items = activeItems
  if (!request || !items || loadingMore.value || items.length >= totalResults.value) return

  const token = activeToken
  const key = activeRequestKey
  const rawQuery = currentRawQuery
  const offset = items.length
  const limit = Math.max(1, request.limit ?? DEFAULT_PAGE_SIZE)
  const diagnosticId = import.meta.env.DEV
    ? `${request.diagnosticId ?? 'spotlight-page'}-offset-${offset}`
    : undefined

  loadingMore.value = true
  const startedAt = import.meta.env.DEV ? performance.now() : 0
  try {
    const page = await loader({
      ...request,
      limit,
      offset,
      diagnosticId,
    })
    if (
      token !== activeToken
      || key !== activeRequestKey
      || rawQuery !== currentRawQuery
      || items !== activeItems
    ) return

    const knownIds = new Set(items.map((image) => image.id))
    const appended = page.items.filter((image) => !knownIds.has(image.id))
    items.push(...appended)
    totalResults.value = page.total
    hasKnownTotal.value = true

    const cached = cachedPages.get(rawQuery)
    if (cached && cached.items === items) {
      cached.total = page.total
      cached.storedAt = performance.now()
    }

    if (import.meta.env.DEV) {
      console.info('[Imagyx][SpotlightPagination] page appended', {
        rawQuery,
        mode: request.mode ?? 'text',
        query: request.query,
        folderId: request.folderId ?? null,
        offset,
        requested: limit,
        received: page.items.length,
        appended: appended.length,
        loaded: items.length,
        total: page.total,
        hasMore: items.length < page.total,
        durationMs: Number((performance.now() - startedAt).toFixed(2)),
      })
    }
  } catch (reason) {
    if (import.meta.env.DEV) {
      console.error('[Imagyx][SpotlightPagination] page failed', {
        rawQuery,
        mode: request.mode ?? 'text',
        query: request.query,
        folderId: request.folderId ?? null,
        offset,
        reason,
      })
    }
  } finally {
    loadingMore.value = false
  }
}

export const spotlightSearchPagination: {
  totalResults: Readonly<Ref<number>>
  loadingMore: Readonly<Ref<boolean>>
  hasKnownTotal: Readonly<Ref<boolean>>
  setRawQuery: typeof setRawQuery
  beginInitialSearch: typeof beginInitialSearch
  completeInitialSearch: typeof completeInitialSearch
  loadMore: typeof loadMore
} = {
  totalResults: readonly(totalResults),
  loadingMore: readonly(loadingMore),
  hasKnownTotal: readonly(hasKnownTotal),
  setRawQuery,
  beginInitialSearch,
  completeInitialSearch,
  loadMore,
}
