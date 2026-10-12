// @ts-nocheck -- bun:test types are not resolvable by svelte-check; keeps `bun run check` at baseline
/**
 * Structural test (project convention: read source, assert patterns) for #127: the UI reaches
 * the API only through `@forge/sdk`. A raw `fetch('/api/…')` works in `api` mode but needs a
 * patched global `fetch` in `wasm` mode, and it skips the SDK's error handling.
 */
import { describe, test, expect } from 'bun:test'
import { readdirSync, readFileSync, statSync } from 'fs'
import { join } from 'path'

const SRC = join(import.meta.dir, '..')

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) return name === '__tests__' ? [] : sources(p)
    return /\.(svelte|ts)$/.test(name) && !/\.test\.ts$/.test(name) ? [p] : []
  })
}

describe('API access', () => {
  test('no source file calls fetch() on an /api URL', () => {
    const offenders = sources(SRC).filter((f) => /\bfetch\(\s*[`'"]\/api\b/.test(readFileSync(f, 'utf-8')))
    expect(offenders).toEqual([])
  })

  test('the global fetch is not patched', () => {
    expect(readFileSync(join(SRC, 'lib', 'sdk.ts'), 'utf-8')).not.toContain('installApiFetch')
  })
})
