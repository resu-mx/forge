import { sveltekit } from '@sveltejs/kit/vite'
import { defineConfig } from 'vite'
import { fileURLToPath } from 'node:url'

// API proxy target. Defaults to host-local dev server; override with
// FORGE_API_URL when running in Docker (where core is at http://core:3000).
const apiTarget = process.env.FORGE_API_URL ?? 'http://localhost:3000'

// By default the API is the Rust runtime inside the browser (build it first:
// `just wasm-bundle typst-bundle`). With VITE_FORGE_MODE=api the Worker factory is replaced by
// a stub, so a build against an API server neither needs nor bundles the generated wasm.
// Keep this in step with the default in src/lib/sdk.ts.
const wasmMode = process.env.VITE_FORGE_MODE !== 'api'

export default defineConfig({
  plugins: [sveltekit()],
  resolve: {
    alias: wasmMode
      ? {}
      : { '@forge/runtime/worker-factory': fileURLToPath(new URL('./src/lib/runtime-stub.ts', import.meta.url)) },
  },
  // The database Worker is an ES module that imports the wasm-bindgen glue.
  worker: { format: 'es' },
  server: {
    // The database Worker and the generated wasm live in packages/runtime, outside this
    // package; Vite serves only allow-listed paths.
    fs: { allow: [fileURLToPath(new URL('../..', import.meta.url))] },
    proxy: {
      '/api': {
        target: apiTarget,
        changeOrigin: true,
        timeout: 120000,  // 2 minutes — PDF generation can take 60s+ on first run (tectonic downloads packages)
      },
    },
  },
})
