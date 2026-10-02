import { ForgeClient, isDevMode } from '@forge/sdk'
import type { ForgeError } from '@forge/sdk'
import { createRuntime, installApiFetch } from '@forge/runtime'

/**
 * Where the API lives:
 * - `api` (default): an HTTP server (the TS server or `forge-server`), reached through
 *   the dev proxy or the same origin.
 * - `wasm`: the Rust API running in this browser, in a Worker, over OPFS storage. Set
 *   `VITE_FORGE_MODE=wasm` and build the runtime first (`just wasm-bundle`).
 */
export const forgeMode: 'api' | 'wasm' = import.meta.env.VITE_FORGE_MODE === 'wasm' ? 'wasm' : 'api'

/** The in-browser runtime; null in `api` mode (and when rendering without a window). */
export const runtime =
  forgeMode === 'wasm' && typeof window !== 'undefined'
    ? createRuntime({
        // Lazy, so an API-mode build never loads the Worker (or needs the generated wasm).
        createWorker: async () => (await import('@forge/runtime/worker-factory')).createForgeWorker(),
        // The PDF compiler is a separate 25 MB module: only fetched when a PDF is first asked for.
        createPdfWorker: async () => (await import('@forge/runtime/worker-factory')).createTypstWorker(),
      })
    : null

// The UI also makes raw `fetch('/api/...')` calls that bypass the SDK; send those to the
// runtime as well.
if (runtime) installApiFetch(runtime)

export const forge = new ForgeClient({ baseUrl: '', debug: true, fetch: runtime?.fetch })

// Expose forge on window in dev mode for console debugging
// Usage: forge.debug.getAll() in browser console
if (typeof window !== 'undefined' && isDevMode()) {
  ;(window as unknown as Record<string, unknown>).forge = forge
  ;(window as unknown as Record<string, unknown>).forgeRuntime = runtime
}

/**
 * Convert an API error to a user-friendly message.
 * Detects when the API server is unreachable and shows a helpful hint.
 */
export function friendlyError(error: ForgeError, fallback?: string): string {
  if (error.code === 'STORAGE_BUSY') {
    return 'Forge is open in another tab. Close that tab, or switch to it.'
  }
  if (
    error.code === 'NETWORK_ERROR' ||
    error.code === 'UNKNOWN_ERROR' &&
    (error.message.includes('non-JSON') || error.message.includes('502'))
  ) {
    return forgeMode === 'wasm'
      ? 'The in-browser Forge database is not available. See Settings → Storage.'
      : 'Cannot connect to the Forge API server. Start it with: just api'
  }
  return fallback ? `${fallback}: ${error.message}` : error.message
}
