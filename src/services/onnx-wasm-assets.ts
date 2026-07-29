import ortWasmModuleUrl from 'onnxruntime-web/dist/ort-wasm-simd-threaded.jsep.mjs?url'
import ortWasmBinaryUrl from 'onnxruntime-web/dist/ort-wasm-simd-threaded.jsep.wasm?url'
import type { env as transformersEnv } from '@huggingface/transformers'

/**
 * Transformers.js defaults to a jsDelivr URL for its ONNX WASM runtime. Keep
 * the runtime in the Vite bundle so installed desktop builds work offline.
 */
export function configureLocalOnnxWasm(env: typeof transformersEnv): void {
  const wasm = env.backends.onnx.wasm
  if (!wasm) return

  wasm.wasmPaths = {
    mjs: ortWasmModuleUrl,
    wasm: ortWasmBinaryUrl,
  }
  // Tauri webviews are not cross-origin isolated. The inference worker already
  // keeps WASM work off the UI thread, so a single ONNX worker is the reliable
  // local fallback.
  wasm.numThreads = 1
  wasm.proxy = false
}
