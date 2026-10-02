/**
 * Stand-in for `@forge/runtime/worker-factory` outside `VITE_FORGE_MODE=wasm`, so an
 * ordinary build does not need the generated wasm. Never called: `sdk.ts` only
 * creates the runtime in wasm mode.
 */
export function createForgeWorker(): never {
  throw new Error('The in-browser runtime is only available with VITE_FORGE_MODE=wasm')
}
