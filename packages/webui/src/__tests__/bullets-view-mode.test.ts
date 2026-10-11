// @ts-nocheck -- bun:test types are not resolvable by svelte-check; keeps `bun run check` at baseline
/**
 * Structural test (project convention: read source, assert patterns) for #207:
 * the Bullets and Perspectives tabs each read and write their own view-mode key.
 */
import { describe, test, expect } from 'bun:test'
import { readFileSync } from 'fs'
import { join } from 'path'

const src = readFileSync(join(import.meta.dir, '..', 'routes', 'data', 'sources', 'BulletsView.svelte'), 'utf-8')

describe('BulletsView per-tab view mode', () => {
  test('derives the storage key from the content type', () => {
    expect(src).toContain("type === 'bullet' ? 'bullets' : 'perspectives'")
  })

  test('initial read and write both use the per-tab key', () => {
    expect(src).toContain('getViewMode(viewModeKey(contentType))')
    expect(src).toContain('setViewMode(viewModeKey(contentType), mode)')
  })

  test('switching tabs re-reads the target tab key', () => {
    expect(src).toContain('viewMode = getViewMode(viewModeKey(type))')
  })

  test('no hardcoded read of the bullets key remains', () => {
    expect(src).not.toContain("getViewMode('bullets')")
  })
})
