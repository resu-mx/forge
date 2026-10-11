import { expect, test, type Page } from '@playwright/test'
import { inPage, openApp } from './helpers'

/**
 * The Sources board in the browser, with no server (resu-mx/forge#206, #191): a source page can
 * show its sources as a board, a drop saves the new status, and the choice survives a reload.
 *
 * The agent seeds through `window.forge`; the person's steps are done in the UI.
 */

const statusOf = (page: Page, id: string) =>
  inPage<{ status: string }>(page, `return ok(await forge.sources.get('${id}'))`).then((s) => s.status)

const column = (page: Page, name: string) =>
  page.locator('.column', { has: page.getByRole('heading', { name, exact: true }) })

/** svelte-dnd-action follows pointer movement, not HTML5 drag events: move in steps, with pauses. */
async function dragCardTo(page: Page, cardText: string, columnName: string) {
  const card = page.locator('.kanban-card', { hasText: cardText })
  const from = (await card.boundingBox())!
  const to = (await column(page, columnName).locator('.column-body').boundingBox())!
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2)
  await page.mouse.down()
  await page.waitForTimeout(150)
  await page.mouse.move(from.x + from.width / 2 + 10, from.y + from.height / 2 + 10, { steps: 5 })
  await page.waitForTimeout(150)
  await page.mouse.move(to.x + to.width / 2, to.y + 40, { steps: 25 })
  await page.waitForTimeout(300)
  await page.mouse.up()
}

test('sources board: a drop on Approved is stored and survives a reload; each type keeps its own mode', async ({
  page,
}) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))
  await openApp(page)
  const ids = await inPage<{ babbage: string; lovelace: string }>(
    page,
    `const a = ok(await forge.sources.create({ title: 'Babbage Notes', description: 'Notes on the Difference Engine.', source_type: 'general' }))
     const b = ok(await forge.sources.create({ title: 'Lovelace Notes', description: 'Notes on the Analytical Engine.', source_type: 'general' }))
     return { babbage: a.id, lovelace: b.id }`,
  )
  expect(await statusOf(page, ids.babbage)).toBe('draft')

  // ── Switch the General page to the board ────────────────────────────────────────────────
  await page.goto('/experience/general')
  await page.getByRole('radio', { name: 'Board' }).click()
  await expect(column(page, 'Draft').getByText('Babbage Notes')).toBeVisible()
  await expect(column(page, 'Draft').getByText('Lovelace Notes')).toBeVisible()

  // ── Drop Babbage on Approved: stored, and the toast reports what was stored ─────────────
  await dragCardTo(page, 'Babbage Notes', 'Approved')
  await expect.poll(() => statusOf(page, ids.babbage)).toBe('approved')
  await expect(page.getByText('Source moved to approved')).toBeVisible()
  expect(await statusOf(page, ids.lovelace)).toBe('draft')

  // ── Reload: still approved, still a board ───────────────────────────────────────────────
  await openApp(page, '/experience/general')
  await expect(page.getByRole('radio', { name: 'Board' })).toBeChecked()
  await expect(column(page, 'Approved').getByText('Babbage Notes')).toBeVisible()
  await expect(column(page, 'Draft').getByText('Lovelace Notes')).toBeVisible()

  // ── Another source type keeps its own mode: Projects is still a list ────────────────────
  await page.goto('/experience/projects')
  await expect(page.getByRole('radio', { name: 'List' })).toBeChecked()

  // ── A card on the board opens in the editor ─────────────────────────────────────────────
  await page.goto('/experience/general')
  await page.locator('.kanban-card', { hasText: 'Lovelace Notes' }).click()
  await expect(page.getByRole('heading', { name: 'Edit Source' })).toBeVisible()
  await expect(page.getByRole('radio', { name: 'List' })).toBeChecked()

  expect(pageErrors).toEqual([])
})
