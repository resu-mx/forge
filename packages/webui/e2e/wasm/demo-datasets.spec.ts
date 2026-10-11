import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { expect, test, type Page } from '@playwright/test'
import { importDataset } from './helpers'

/**
 * Every generated demo dataset (`packages/demo-data`) loads into the browser app and renders.
 *
 * Skipped unless FORGE_DEMO_DATA_DIR points at the generator's output (`data/demo`), so the
 * regular e2e-wasm job is unaffected. Run it with `just demo-data e2e`.
 *
 * One test per `user/<uuid>/data.sqlite` found there. The first dataset is imported the way a
 * person does it (Settings → Storage), the rest through `window.forgeRuntime.importDatabase`.
 * Each test has its own browser context, so its own empty OPFS database to import into.
 */

const DIR = process.env.FORGE_DEMO_DATA_DIR

interface Dataset {
  slug: string
  uuid: string
  file: string
}

function discoverDatasets(dir: string): Dataset[] {
  const root = resolve(dir)
  // index.json only names them; the files on disk are what gets tested.
  const slugs = new Map<string, string>()
  const index = join(root, 'index.json')
  if (existsSync(index)) {
    const personas = (JSON.parse(readFileSync(index, 'utf8')).personas ?? {}) as Record<string, { uuid: string }>
    for (const [slug, p] of Object.entries(personas)) slugs.set(p.uuid, slug)
  }
  const users = join(root, 'user')
  if (!existsSync(users)) return []
  return readdirSync(users)
    .filter((uuid) => existsSync(join(users, uuid, 'data.sqlite')))
    .sort()
    .map((uuid) => ({ uuid, slug: slugs.get(uuid) ?? uuid, file: join(users, uuid, 'data.sqlite') }))
}

/**
 * Boards open in list mode unless `localStorage['forge:viewMode:<key>']` is `board`. Today the
 * bullets page reads only `bullets` (for both of its tabs) and only the unrouted sources board
 * reads `sources`; all five are set so the spec keeps working if that changes.
 */
const BOARD_VIEW_KEYS = ['bullets', 'perspectives', 'sources', 'resumes', 'jobDescriptions']

/** A board column by its label: open (heading) or collapsed (label + count). */
function boardColumn(page: Page, label: string) {
  return {
    open: page.locator('.column', { has: page.getByRole('heading', { name: label, exact: true }) }),
    collapsed: page.locator('.column-collapsed', {
      has: page.locator('.collapsed-label', { hasText: new RegExp(`^${label}$`) }),
    }),
  }
}

/**
 * Every column of the board on screen has at least one card. A collapsed column (Archived,
 * Closed, Excluded) shows only a count: check it, then expand it and check for a card too.
 */
async function expectEveryColumnHasACard(page: Page, labels: string[]) {
  for (const label of labels) {
    const { open, collapsed } = boardColumn(page, label)
    await expect(open.or(collapsed), `column "${label}"`).toHaveCount(1)
    if (await collapsed.count()) {
      await expect(collapsed.locator('.collapsed-count'), `"${label}" (collapsed) count`).toHaveText(/^[1-9]\d*$/)
      await collapsed.click()
    }
    await expect(open.locator('.column-count'), `"${label}" count`).toHaveText(/^[1-9]\d*$/)
    await expect(open.locator('.column-body > *').first(), `a card in "${label}"`).toBeVisible()
  }
}

const STATUS_COLUMNS = ['Draft', 'In Review', 'Approved', 'Rejected', 'Archived']

/** Dashboard "Quick Stats" label → the table it counts. */
const DASHBOARD_TOTALS: Record<string, string> = {
  Sources: 'sources',
  Bullets: 'bullets',
  Perspectives: 'perspectives',
  Organizations: 'organizations',
  Resumes: 'resumes',
}

/** Per-table row counts from the dataset's manifest.json, when it has one. */
function readManifestCounts(dataset: Dataset): Record<string, number> | null {
  const manifest = join(dataset.file, '..', 'manifest.json')
  if (!existsSync(manifest)) return null
  return (JSON.parse(readFileSync(manifest, 'utf8')).counts ?? null) as Record<string, number> | null
}

if (!DIR) {
  test('demo datasets', () => {
    test.skip(true, 'FORGE_DEMO_DATA_DIR is not set; run `just demo-data e2e`')
  })
} else {
  const datasets = discoverDatasets(DIR)

  test('demo datasets: at least one under FORGE_DEMO_DATA_DIR', () => {
    expect(datasets.map((d) => d.slug), `user/*/data.sqlite under ${DIR}`).not.toEqual([])
  })

  datasets.forEach((dataset, i) => {
    const via = i === 0 ? 'ui' : 'runtime'

    test(`demo dataset ${dataset.slug}: imports (${via}) and renders`, async ({ page }) => {
      test.setTimeout(300_000)
      const pageErrors: string[] = []
      page.on('pageerror', (e) => pageErrors.push(e.message))
      await page.addInitScript((keys) => {
        // Init scripts run in every frame, and the PDF preview's frame has no localStorage
        // (a board that stays in list mode fails its own assertions below).
        if (window.top !== window || !window.localStorage) return
        for (const k of keys) window.localStorage.setItem(`forge:viewMode:${k}`, 'board')
      }, BOARD_VIEW_KEYS)

      await importDataset(page, dataset.file, via)

      // ── Dashboard: totals and integrity ───────────────────────────────────────────────────
      await expect(page.getByText('No drift detected.', { exact: false })).toBeVisible()
      // The totals are this dataset's row counts (from its manifest), which also shows the
      // import replaced the empty database the context started with.
      const counts = readManifestCounts(dataset)
      for (const [label, table] of Object.entries(DASHBOARD_TOTALS)) {
        const total = page.locator('.stat-card', { has: page.locator('.stat-label', { hasText: label }) }).locator('.stat-number')
        await expect(total, `dashboard total: ${label}`).toHaveText(/^[1-9]\d*$/)
        if (counts) await expect(total, `dashboard total: ${label} = manifest ${table}`).toHaveText(String(counts[table]))
      }

      // ── Boards: every column has a card ───────────────────────────────────────────────────
      await page.goto('/data/bullets')
      await expectEveryColumnHasACard(page, STATUS_COLUMNS)

      await page.getByRole('button', { name: 'Perspectives', exact: true }).click()
      await expectEveryColumnHasACard(page, STATUS_COLUMNS)

      await page.goto('/resumes')
      await expectEveryColumnHasACard(page, STATUS_COLUMNS)

      await page.goto('/opportunities/job-descriptions')
      await expectEveryColumnHasACard(page, [
        'Discovered',
        'Analyzing',
        'Applying',
        'Applied',
        'Interviewing',
        'Offered',
        'Closed',
      ])

      await page.goto('/opportunities/organizations')
      await expectEveryColumnHasACard(page, ['Backlog', 'Researching', 'Targeting', 'Excluded'])

      // Sources have a board component (routes/data/sources/SourcesView.svelte), but no page
      // shows it: /data/sources redirects to /experience/roles, whose SourcesView is a list.
      // So, weaker than a board: every source page lists something, and between them every
      // board column's status appears on some source card.
      const sourceStatuses = new Set<string>()
      for (const type of ['roles', 'projects', 'presentations', 'education', 'general']) {
        await page.goto(`/experience/${type}`)
        const cards = page.locator('.source-card')
        await expect(cards.first(), `a source on /experience/${type}`).toBeVisible()
        for (const s of await cards.locator('.badge').allTextContents()) sourceStatuses.add(s.trim())
      }
      expect([...sourceStatuses].sort()).toEqual([...STATUS_COLUMNS].sort())

      // ── An approved resume's PDF preview ──────────────────────────────────────────────────
      await page.goto('/resumes')
      await boardColumn(page, 'Approved').open.locator('.column-body > *').first().click()
      await page.getByRole('tab', { name: 'Preview', exact: true }).click()
      await expect(page.locator('iframe[title="Resume PDF Preview"]')).toBeVisible({ timeout: 60_000 })

      // ── Lists that are not boards ─────────────────────────────────────────────────────────
      for (const [path, card] of [
        ['/qualifications/credentials', '.credential-card'],
        ['/qualifications/certifications', '.cert-card'],
        ['/data/contacts', '.contact-card'],
        ['/data/notes', '.note-card'],
        ['/resumes/summaries', '.summary-row'],
      ] as const) {
        await page.goto(path)
        await expect(page.locator(card).first(), `an item on ${path}`).toBeVisible()
      }
      // The generator's own note marks every dataset as generated (dataset_meta has no page).
      await page.goto('/data/notes')
      await expect(page.locator('.note-card', { hasText: 'About this demo dataset' })).toBeVisible()

      // ── The answer bank: EEO selects and work-authorization radios are filled in ──────────
      await page.goto('/settings/eeo')
      for (const id of ['eeo-gender', 'eeo-race', 'eeo-veteran', 'eeo-disability']) {
        await expect(page.locator(`#${id}`), id).not.toHaveValue('')
      }
      await page.goto('/settings/work-auth')
      for (const name of ['us_authorized', 'sponsorship']) {
        await expect(page.locator(`input[type="radio"][name="${name}"]:checked`), name).toHaveCount(1)
      }

      expect(pageErrors).toEqual([])
    })
  })
}
