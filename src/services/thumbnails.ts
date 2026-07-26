import type { ImageAsset } from '../types'
import { imagyxApi } from '../api/tauri'

const MAX_CONCURRENT_REQUESTS = 4
const MEMORY_CACHE_CAPACITY = 512

interface QueueTask {
  run: () => Promise<void>
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

  const request = new Promise<string>((resolve, reject) => {
    queue.push({
      run: async () => {
        try {
          const path = await imagyxApi.thumbnail(image)
          const url = imagyxApi.fileUrl(path)
          remember(key, url)
          resolve(url)
        } catch (error) {
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
