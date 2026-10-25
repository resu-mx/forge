// @ts-nocheck -- bun:test types are not resolvable by svelte-check; keeps `bun run check` at baseline
import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { DEFAULT_DATA_URL, DEFAULT_ID_URL, resolveOrigin } from './config-defaults.js'
import { dataUrl, hostCanCarrySession, idUrl, signInAvailable, syncAvailable } from './config'

describe('service origins', () => {
  test('default to the production hosts', () => {
    expect(DEFAULT_ID_URL).toBe('https://id.resu.mx')
    expect(DEFAULT_DATA_URL).toBe('https://data.resu.mx')
  })

  test('the typed values are origins taken from the build variables, else the defaults', () => {
    // Bun exposes the process environment as `import.meta.env`, as Vite does for VITE_* at build.
    expect(idUrl).toBe(resolveOrigin(process.env.VITE_ID_URL, DEFAULT_ID_URL))
    expect(dataUrl).toBe(resolveOrigin(process.env.VITE_DATA_URL, DEFAULT_DATA_URL))
    expect(new URL(idUrl).origin).toBe(idUrl)
    expect(new URL(dataUrl).origin).toBe(dataUrl)
  })

  test('resolveOrigin keeps the origin and drops path and trailing slash', () => {
    expect(resolveOrigin('http://localhost:8787/', 'x')).toBe('http://localhost:8787')
    expect(resolveOrigin('https://id.example.test/auth/', 'x')).toBe('https://id.example.test')
  })

  test('resolveOrigin falls back on empty, malformed or non-web values', () => {
    for (const bad of [undefined, '', '   ', 'not a url', 'javascript:alert(1)', 'ftp://id.resu.mx']) {
      expect(resolveOrigin(bad, 'https://fallback.test')).toBe('https://fallback.test')
    }
  })
})

describe('where sign-in and sync exist', () => {
  test('app.resu.mx, localhost and 127.0.0.1 can carry the session cookie', () => {
    for (const host of ['app.resu.mx', 'localhost', '127.0.0.1', 'APP.resu.mx']) {
      expect(signInAvailable(host)).toBe(true)
      expect(syncAvailable(host)).toBe(true)
    }
  })

  test('previews and every other host cannot', () => {
    for (const host of [
      'resumx-webapp.pages.dev',
      'pr-12.resumx-webapp.pages.dev',
      'id.resu.mx',
      'resu.mx',
      'evil.app.resu.mx',
      'app.resu.mx.evil.test',
      'localhost.evil.test',
      '',
      undefined,
    ]) {
      expect(signInAvailable(host)).toBe(false)
      expect(syncAvailable(host)).toBe(false)
      expect(hostCanCarrySession(host)).toBe(false)
    }
  })

  test('without a window there is no hostname, so nothing is available', () => {
    expect(typeof location).toBe('undefined')
    expect(signInAvailable()).toBe(false)
  })
})

describe('Content-Security-Policy', () => {
  const svelteConfig = readFileSync(join(import.meta.dir, '..', '..', 'svelte.config.js'), 'utf-8')

  test('connect-src allows the id and data origins, from the same config module', () => {
    expect(svelteConfig).toContain("from './src/lib/config-defaults.js'")
    expect(svelteConfig).toMatch(/'connect-src':\s*\['self',\s*idOrigin,\s*dataOrigin\]/)
  })

  test('form-action stays self-only: sign-in is fetch plus a top-level navigation', () => {
    expect(svelteConfig).toMatch(/'form-action':\s*\['self'\]/)
  })
})
