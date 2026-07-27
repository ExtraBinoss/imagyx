import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'
import { perfSample } from '../utils'

const MAX_CONCURRENT_REQUESTS = 6
const MAX_QUEUED_REQUESTS = 96
const MEMORY_CACHE_CAPACITY = 512

interface QueueTask {
  key: string
  run: () => Promise<void>
  cancel: () => void
}

const urls = new Map<string, string>()
const pending = new Map<string, Promise<string>>()
const queue: QueueTask[] = []
let activeRequests = 0

function keyFor(image: Pick<ImageAsset, 'id' | 'modifiedAt'>): string {
  return `${image.id}:${image.modifiedAt}`
}

function remember(key: string, url: string): void {
  urls.delete(key)
  urls.set(key, url)
  while (urls.size > MEMORY_CACHE_CAPACITY) {
    const oldest = urls.keys().next().value as string | undefined
    if (!oldest) break
    urls.delete(oldest)
  }
}

function pump(): void {
  while (activeRequests < MAX_CONCURRENT_REQUESTS && queue.length > 0) {
    const task = queue.shift()
    if (!task) return
    activeRequests += 1
    void task.run().finally(() => {
      activeRequests -= 1
      pump()
    })
  }
}

function makeRoom(): void {
  while (queue.length >= MAX_QUEUED_REQUESTS) {
    const dropped = queue.shift()
    if (!dropped) return
    pending.delete(dropped.key)
    dropped.cancel()
  }
}

export function requestThumbnail(
  image: Pick<ImageAsset, 'id' | 'path' | 'modifiedAt'>,
): Promise<string> {
  const key = keyFor(image)
  const cached = urls.get(key)
  if (cached) {
    remember(key, cached)
    return Promise.resolve(cached)
  }

  const existing = pending.get(key)
  if (existing) return existing

  const queuedAt = performance.now()
  const request = new Promise<string>((resolve, reject) => {
    makeRoom()
    queue.push({
      key,
      cancel: () => reject(new Error('Thumbnail request superseded by the visible viewport')),
      run: async () => {
        const startedAt = performance.now()
        perfSample('Thumbnail', 'queue wait', startedAt - queuedAt)
        try {
          const path = await imagyxApi.thumbnail(image)
          const url = imagyxApi.fileUrl(path)
          remember(key, url)
          perfSample('Thumbnail', 'IPC and generation', performance.now() - startedAt)
          resolve(url)
        } catch (error) {
          perfSample('Thumbnail', 'failed request', performance.now() - startedAt)
          reject(error)
        } finally {
          pending.delete(key)
        }
      },
    })
    pump()
  })

  pending.set(key, request)
  return request
}
