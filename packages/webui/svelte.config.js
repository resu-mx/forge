import adapter from '@sveltejs/adapter-static'
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'

// Production builds only: the Vite dev server needs inline scripts and websockets that this
// policy would block. FORGE_CSP=off skips it for a build too.
const enforceCsp = process.argv.includes('build') && process.env.FORGE_CSP !== 'off'

/**
 * `window.forge` exposes a person's data to any script on this origin, so scripts are limited
 * to our own files. Hash mode lets SvelteKit allow exactly its own inline bootstrap script
 * (the SPA fallback page) and nothing else.
 *
 * Each directive that goes beyond `'self'` is here because the app needs it:
 * - `'wasm-unsafe-eval'`: WebAssembly compilation (the database and Typst modules).
 * - `style-src 'unsafe-inline'`: some pages set inline `style` attributes.
 * - `frame-src blob:` + `object-src 'self'`: the PDF preview is an iframe on a blob: URL, and
 *   Chrome's PDF viewer renders blank under `object-src 'none'` (checked).
 * - `img-src data: blob:`: generated images.
 *
 * @type {import('@sveltejs/kit').CspDirectives}
 */
const cspDirectives = {
  'default-src': ['self'],
  'script-src': ['self', 'wasm-unsafe-eval'],
  'worker-src': ['self', 'blob:'],
  'style-src': ['self', 'unsafe-inline'],
  'img-src': ['self', 'data:', 'blob:'],
  'connect-src': ['self'],
  'frame-src': ['self', 'blob:'],
  'object-src': ['self'],
  'base-uri': ['self'],
  'form-action': ['self'],
}

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: 'index.html',
    }),
    prerender: {
      entries: [],
    },
    ...(enforceCsp ? { csp: { mode: 'hash', directives: cspDirectives } } : {}),
  },
}

export default config
