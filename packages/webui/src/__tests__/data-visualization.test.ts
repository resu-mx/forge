/**
 * Data Visualization Acceptance Tests (Phase 79)
 *
 * Verifies chart theming, token usage, dark mode fixes,
 * and component correctness per the Design System spec.
 */
import { describe, test, expect } from 'bun:test'
import { readFileSync, readdirSync, existsSync } from 'fs'
import { join } from 'path'
import type { PaginatedResult, PaginationParams } from '@forge/sdk'
import { listAll, LIST_PAGE_LIMIT } from '../lib/list-all'

const WEBUI_SRC = join(import.meta.dir, '..')
const CHARTS_DIR = join(WEBUI_SRC, 'lib', 'components', 'charts')
const COMPONENTS_DIR = join(WEBUI_SRC, 'lib', 'components')

function read(path: string): string {
  return readFileSync(path, 'utf-8')
}

describe('Data Visualization Acceptance Tests', () => {
  describe('79.1: echarts-theme.ts uses correct tokens', () => {
    const theme = read(join(CHARTS_DIR, 'echarts-theme.ts'))

    test('does NOT reference --border-primary (nonexistent token)', () => {
      expect(theme).not.toContain('--border-primary')
    })

    test('does NOT reference --border-subtle (nonexistent token)', () => {
      expect(theme).not.toContain('--border-subtle')
    })

    test('uses --color-border for axis/tooltip borders', () => {
      expect(theme).toContain('--color-border')
    })

    test('uses --color-surface-sunken for split/grid lines', () => {
      expect(theme).toContain('--color-surface-sunken')
    })
  })

  describe('79.2: Chart components use --text-* tokens (not --font-size-*)', () => {
    const chartFiles = [
      'JDSkillRadar.svelte',
      'CompensationBulletGraph.svelte',
      'SkillsSunburst.svelte',
      'ApplicationGantt.svelte',
      'RoleChoropleth.svelte',
    ]

    for (const file of chartFiles) {
      const filePath = join(CHARTS_DIR, file)
      if (!existsSync(filePath)) continue

      test(`${file}: no --font-size-* references`, () => {
        const content = read(filePath)
        expect(content).not.toMatch(/--font-size-(sm|xs|base|lg|xl)/)
      })
    }
  })

  describe('79.3: EChart loading overlay supports dark mode', () => {
    const echart = read(join(CHARTS_DIR, 'EChart.svelte'))

    test('maskColor does NOT hardcode rgba(255, 255, 255', () => {
      expect(echart).not.toContain("maskColor: 'rgba(255, 255, 255")
      expect(echart).not.toContain('maskColor: "rgba(255, 255, 255')
    })

    test('maskColor resolves --color-surface token', () => {
      expect(echart).toContain('--color-surface')
    })
  })

  describe('79.5: Dashboard pending card links', () => {
    const dashboard = read(join(WEBUI_SRC, 'routes', '+page.svelte'))

    test('has a link to tab=bullets for pending bullets', () => {
      expect(dashboard).toContain('tab=bullets')
    })

    test('has a link to tab=perspectives for pending perspectives', () => {
      expect(dashboard).toContain('tab=perspectives')
    })

    test('perspectives link is distinct from bullets link', () => {
      const bulletsLinks = (dashboard.match(/tab=bullets/g) || []).length
      const perspectivesLinks = (dashboard.match(/tab=perspectives/g) || []).length
      expect(bulletsLinks).toBe(1)
      expect(perspectivesLinks).toBe(1)
    })
  })

  // #193: both API runtimes clamp a list `limit` to 200, so a chart that asks for 2000 rows
  // counts only the first 200. Charts page through with `listAll` instead.
  describe('Charts count every row, past the 200-row list cap', () => {
    type Row = { id: string }
    type Params = PaginationParams & { status?: string }

    /** A list endpoint over `total` rows that clamps `limit` to `cap`, like both runtimes. */
    function fakeList(total: number, cap = LIST_PAGE_LIMIT) {
      const rows: Row[] = Array.from({ length: total }, (_, i) => ({ id: `row-${i}` }))
      const calls: Params[] = []
      const list = async (params: Params): Promise<PaginatedResult<Row>> => {
        calls.push(params)
        const offset = params.offset ?? 0
        const limit = Math.min(cap, Math.max(1, params.limit ?? 50))
        return { ok: true, data: rows.slice(offset, offset + limit), pagination: { total, offset, limit } }
      }
      return { list, calls }
    }

    test('listAll returns all 300 rows where one list call stops at 200', async () => {
      const { list, calls } = fakeList(300)

      const single = await list({ limit: 2000 })
      expect(single.ok && single.data.length).toBe(200)

      calls.length = 0
      const all = await listAll(list)
      if (!all.ok) throw new Error(all.error.message)
      expect(all.data.length).toBe(300)
      expect(new Set(all.data.map((r) => r.id)).size).toBe(300)
      expect(all.data[299].id).toBe('row-299')
      expect(all.pagination.total).toBe(300)
      expect(calls.map((c) => c.offset)).toEqual([0, 200])
    })

    test('listAll makes one request when the first page holds every row', async () => {
      for (const total of [0, 1, 200]) {
        const { list, calls } = fakeList(total)
        const all = await listAll(list)
        expect(all.ok && all.data.length).toBe(total)
        expect(calls.length).toBe(1)
      }
    })

    test('listAll steps by the rows returned, so a lower server cap still reaches the total', async () => {
      const { list, calls } = fakeList(120, 50)
      const all = await listAll(list)
      expect(all.ok && all.data.length).toBe(120)
      expect(calls.map((c) => c.offset)).toEqual([0, 50, 100])
    })

    test('listAll forwards the filter on every page', async () => {
      const { list, calls } = fakeList(450)
      await listAll(list, { status: 'approved' })
      expect(calls.length).toBe(3)
      for (const c of calls) expect(c).toMatchObject({ status: 'approved', limit: LIST_PAGE_LIMIT })
    })

    test('listAll returns the error from any page', async () => {
      const error = { code: 'NETWORK_ERROR', message: 'down' }
      const failFirst = async (): Promise<PaginatedResult<Row>> => ({ ok: false, error })
      expect(await listAll(failFirst)).toEqual({ ok: false, error })

      const { list } = fakeList(450)
      const failLater = async (p: Params): Promise<PaginatedResult<Row>> =>
        (p.offset ?? 0) > 0 ? { ok: false, error } : list(p)
      expect(await listAll(failLater)).toEqual({ ok: false, error })
    })

    // `skills.list` is not paginated (it returns every skill); every other `.list` is.
    test('chart components never ask a paginated list for more than one page', () => {
      const offenders: string[] = []
      for (const file of readdirSync(CHARTS_DIR).filter((f) => f.endsWith('.svelte'))) {
        for (const m of read(join(CHARTS_DIR, file)).matchAll(/forge\.(\w+)\.list\(\{[^}]*\blimit:\s*(\d+)/g)) {
          if (m[1] !== 'skills' && Number(m[2]) > LIST_PAGE_LIMIT) offenders.push(`${file}: ${m[0]}`)
        }
      }
      expect(offenders).toEqual([])
    })
  })
})
