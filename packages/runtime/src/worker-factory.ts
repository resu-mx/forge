/**
 * The real database Worker. Lives in its own module, imported lazily, so that code which
 * never starts the runtime (an ordinary API-mode build) does not pull in the Worker
 * or need the generated wasm. A bundler (Vite) turns `new URL(..., import.meta.url)`
 * into a separate worker chunk. Build the wasm first: `just wasm-bundle`.
 */
export function createForgeWorker(): Worker {
  return new Worker(new URL('./worker.ts', import.meta.url), { type: 'module', name: 'forge-database' })
}
