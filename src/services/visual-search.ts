import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { ImageAsset } from '../types'
import { VisionWorkerClient } from './vision-worker-client'
import { visualSearchSession, type VisualSearchSourceKind } from './visual-search-session'

const MODEL_ID = 'Xenova/mobileclip_s0'
const IMAGE_EDGE = 224
const IMAGE_CHANNELS = 3
const IMAGE_BYTES = IMAGE_EDGE * IMAGE_EDGE * IMAGE_CHANNELS
const visionWorker = new VisionWorkerClient()
let visionReady: Promise<void> | null = null
let querySequence = 0

function filename(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path
}

async function ensureVisionReady() {
  if (!visionReady) {
    visionReady = (async () => {
      const modelRoot = await invoke<string>('prepare_local_model', { modelKey: 'mobileclip-s0' })
      const modelsDir = modelRoot.replace(/[\\/]+Xenova[\\/]mobileclip_s0$/, '')
      const localModelPath = `${convertFileSrc(modelsDir).replace(/\/$/, '')}/`
      await visionWorker.initialize(MODEL_ID, localModelPath)
    })().catch((error) => {
      visionReady = null
      throw error
    })
  }
  await visionReady
}

async function inferPixels(pixels: ArrayBuffer): Promise<number[]> {
  if (pixels.byteLength !== IMAGE_BYTES) {
    throw new Error(`Invalid visual query pixels: ${pixels.byteLength} bytes`)
  }
  await ensureVisionReady()
  querySequence += 1
  const result = await visionWorker.infer(pixels, 1, `visual-query-${Date.now().toString(36)}-${querySequence}`)
  const vector = result.vectors[0]
  if (!vector?.length) throw new Error('MobileCLIP returned an empty visual query')
  return vector
}

async function blobPixels(blob: Blob): Promise<ArrayBuffer> {
  const bitmap = await createImageBitmap(blob)
  try {
    const canvas = document.createElement('canvas')
    canvas.width = IMAGE_EDGE
    canvas.height = IMAGE_EDGE
    const context = canvas.getContext('2d', { alpha: false, willReadFrequently: true })
    if (!context) throw new Error('Canvas 2D is unavailable')

    const sourceRatio = bitmap.width / Math.max(1, bitmap.height)
    const targetRatio = 1
    let sourceX = 0
    let sourceY = 0
    let sourceWidth = bitmap.width
    let sourceHeight = bitmap.height
    if (sourceRatio > targetRatio) {
      sourceWidth = bitmap.height
      sourceX = (bitmap.width - sourceWidth) / 2
    } else if (sourceRatio < targetRatio) {
      sourceHeight = bitmap.width
      sourceY = (bitmap.height - sourceHeight) / 2
    }
    context.drawImage(
      bitmap,
      sourceX,
      sourceY,
      sourceWidth,
      sourceHeight,
      0,
      0,
      IMAGE_EDGE,
      IMAGE_EDGE,
    )
    const rgba = context.getImageData(0, 0, IMAGE_EDGE, IMAGE_EDGE).data
    const rgb = new Uint8Array(IMAGE_BYTES)
    for (let source = 0, target = 0; source < rgba.length; source += 4) {
      rgb[target] = rgba[source] ?? 0
      rgb[target + 1] = rgba[source + 1] ?? 0
      rgb[target + 2] = rgba[source + 2] ?? 0
      target += 3
    }
    return rgb.buffer
  } finally {
    bitmap.close()
  }
}

async function findSimilar(image: ImageAsset): Promise<string> {
  visualSearchSession.begin('indexed', image.name, { sourceImage: image })
  try {
    const vector = await invoke<number[]>('get_image_embedding', { imageId: image.id })
    return visualSearchSession.activate(vector, {
      sourceKind: 'indexed',
      label: image.name,
      sourceImage: image,
      excludeImageId: image.id,
    })
  } catch (error) {
    visualSearchSession.fail(error)
    throw error
  }
}

async function searchPath(path: string, sourceKind: VisualSearchSourceKind = 'file'): Promise<string> {
  const label = filename(path)
  const previewUrl = convertFileSrc(path)
  visualSearchSession.begin(sourceKind, label, { previewUrl })
  try {
    const pixels = await invoke<ArrayBuffer>('prepare_visual_query_image', { path })
    const vector = await inferPixels(pixels)
    return visualSearchSession.activate(vector, {
      sourceKind,
      label,
      previewUrl,
    })
  } catch (error) {
    visualSearchSession.fail(error)
    throw error
  }
}

async function searchBlob(
  blob: Blob,
  label: string,
  sourceKind: Extract<VisualSearchSourceKind, 'clipboard' | 'drop'>,
): Promise<string> {
  if (blob.type && !blob.type.startsWith('image/')) {
    throw new Error('The dropped or pasted file is not an image')
  }
  const previewUrl = URL.createObjectURL(blob)
  visualSearchSession.begin(sourceKind, label, { previewUrl })
  try {
    const vector = await inferPixels(await blobPixels(blob))
    return visualSearchSession.activate(vector, {
      sourceKind,
      label,
      previewUrl,
      ownsPreviewUrl: true,
    })
  } catch (error) {
    URL.revokeObjectURL(previewUrl)
    visualSearchSession.fail(error)
    throw error
  }
}

export const visualSearch = {
  findSimilar,
  searchPath,
  searchFile: (file: File, sourceKind: 'clipboard' | 'drop' = 'drop') =>
    searchBlob(file, file.name || 'image', sourceKind),
  searchBlob,
}
