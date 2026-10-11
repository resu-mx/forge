// @ts-nocheck -- bun:test types are not resolvable by svelte-check; keeps `bun run check` at baseline
/**
 * Structural test (project convention: read source, assert patterns) for #206 and #191:
 * the live SourcesView offers a board, each source type keeps its own view mode, and a drop
 * saves the status the server stored. The unrouted copy of the view is gone.
 */
import { describe, test, expect } from 'bun:test'
import { existsSync, readFileSync } from 'fs'
import { join } from 'path'

const SRC = join(import.meta.dir, '..')
const view = readFileSync(join(SRC, 'lib', 'components', 'SourcesView.svelte'), 'utf-8')
const dashboard = readFileSync(join(SRC, 'routes', '+page.svelte'), 'utf-8')

describe('SourcesView board', () => {
  test('offers the shared view toggle and kanban', () => {
    expect(view).toContain("import ViewToggle from '$lib/components/ViewToggle.svelte'")
    expect(view).toContain("import GenericKanban from '$lib/components/kanban/GenericKanban.svelte'")
    expect(view).toContain('<GenericKanban')
    expect(view).toContain('<ViewToggle')
  })

  test('the toggle is reachable in both modes', () => {
    // Once in the board header, once in the list header.
    expect(view.match(/<ViewToggle/g)?.length).toBe(2)
  })

  test('each source type reads and writes its own view-mode key', () => {
    expect(view).toContain("`sources:${sourceTypeFilter ?? 'all'}`")
    expect(view).toContain('getViewMode(viewModeKey)')
    expect(view).toContain('setViewMode(viewModeKey, mode)')
    expect(view).not.toContain("getViewMode('sources')")
  })

  test('a board column drop saves the status through the typed update, with no cast', () => {
    expect(view).toContain('forge.sources.update(itemId, { status: newStatus as UnifiedKanbanStatus })')
    expect(view).not.toContain('as any })')
  })

  test('a failed drop throws so the card returns, and the shown status is the stored one', () => {
    expect(view).toContain("throw new Error('Status update failed')")
    expect(view).toContain('status: result.data.status')
  })

  test('the Approved column also holds the legacy deriving status but drops as approved', () => {
    expect(view).toContain("statuses: ['approved', 'deriving'], dropStatus: 'approved'")
  })
})

describe('the unrouted copy is gone', () => {
  test('routes/data/sources/SourcesView.svelte and its .bak no longer exist', () => {
    expect(existsSync(join(SRC, 'routes', 'data', 'sources', 'SourcesView.svelte'))).toBe(false)
    expect(existsSync(join(SRC, 'routes', 'data', 'sources', '_old_page.svelte.bak'))).toBe(false)
  })

  test('the Bullets and Skills views that share the directory are untouched', () => {
    expect(existsSync(join(SRC, 'routes', 'data', 'sources', 'BulletsView.svelte'))).toBe(true)
    expect(existsSync(join(SRC, 'routes', 'data', 'sources', 'SkillsView.svelte'))).toBe(true)
  })
})

describe('dashboard links', () => {
  const bulletsView = readFileSync(join(SRC, 'routes', 'data', 'sources', 'BulletsView.svelte'), 'utf-8')

  test('no card or button points at the /data/sources redirect', () => {
    expect(dashboard).not.toContain('/data/sources')
  })

  test('the pending cards deep-link to their own tab, and "View Sources" opens a sources page', () => {
    expect(dashboard).toContain('href="/data/bullets?tab=bullets"')
    expect(dashboard).toContain('href="/data/bullets?tab=perspectives"')
    expect(dashboard).toContain("goto('/experience/roles')")
  })

  test('BulletsView opens on the tab the link names', () => {
    expect(bulletsView).toContain("import { page } from '$app/state'")
    expect(bulletsView).toContain("page.url.searchParams.get('tab') === 'perspectives'")
  })
})
