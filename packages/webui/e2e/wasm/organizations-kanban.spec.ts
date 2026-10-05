import { expect, test, type Page } from '@playwright/test'

/**
 * Organizations and the pipeline board in the browser, with no server (resu-mx/forge#32):
 * location CRUD with its address, the HQ label, and every Kanban status action.
 *
 * The agent seeds through `window.forge`; the person's steps are done in the UI.
 */

type Forge = Record<string, any>

/** Run `body` in the page with the app's own client (`window.forge`) and return its result. */
async function inPage<T>(page: Page, body: string): Promise<T> {
  return page.evaluate(
    async (src) => {
      const forge = (window as unknown as { forge: Forge }).forge
      const ok = <R,>(r: { ok: boolean; data?: R; error?: unknown }): R => {
        if (!r.ok) throw new Error(JSON.stringify(r.error))
        return r.data as R
      }
      return await new Function('forge', 'ok', `return (async () => { ${src} })()`)(forge, ok)
    },
    body,
  ) as Promise<T>
}

async function openApp(page: Page, path = '/') {
  await page.goto(path)
  await page.waitForFunction(() => !!(window as unknown as { forge?: unknown }).forge)
  await inPage(page, 'await window.forgeRuntime.ready')
}

const statusOf = (page: Page, id: string) =>
  inPage<{ status: string | null }>(page, `return ok(await forge.organizations.get('${id}'))`).then((o) => o.status)

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

test('organizations: add, edit and delete a location; the HQ shows on the list and the board', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))
  await openApp(page)
  const org = await inPage<{ id: string }>(
    page,
    `return ok(await forge.organizations.create({ name: 'Babbage Works', status: 'backlog' }))`,
  )

  const locations = page.locator('.campuses-section', {
    has: page.getByRole('heading', { name: 'Campuses / Locations' }),
  })
  const openOrg = async () => {
    await page.getByText('Babbage Works').first().click()
    await expect(locations).toBeVisible()
  }

  // ── Add an HQ location with an address ──────────────────────────────────────────────────
  await page.goto('/data/organizations')
  await openOrg()
  await locations.getByRole('button', { name: '+ Add' }).click()
  await page.locator('#campus-name').fill('Head Office')
  await page.locator('#campus-city').fill('Arlington')
  await page.locator('#campus-state').fill('VA')
  await locations.getByLabel('Headquarters').check()
  await locations.getByRole('button', { name: 'Add Campus' }).click()
  await expect(locations.getByText('Head Office')).toBeVisible()

  const [loc] = await inPage<{ id: string; address_id: string | null; is_headquarters: unknown }[]>(
    page,
    `return ok(await forge.organizations.listLocations('${org.id}'))`,
  )
  expect(!!loc.is_headquarters).toBe(true)
  expect(loc.address_id).toBeTruthy()
  expect(await inPage(page, `return ok(await forge.addresses.get('${loc.address_id}'))`)).toMatchObject({
    city: 'Arlington',
    state: 'VA',
  })

  // ── The HQ label: org list (after a reload) and the board card ──────────────────────────
  await openApp(page, '/data/organizations')
  await expect(page.locator('.meta-item', { hasText: 'Arlington, VA' })).toBeVisible()
  await page.goto('/opportunities/organizations')
  await expect(page.locator('.kanban-card', { hasText: 'Babbage Works' }).getByText('Arlington, VA')).toBeVisible()

  // ── Edit ────────────────────────────────────────────────────────────────────────────────
  await page.goto('/data/organizations')
  await openOrg()
  await locations.getByText('Head Office').click()
  await page.locator('#edit-campus-name').fill('Main Office')
  await page.locator('#edit-campus-city').fill('Alexandria')
  await locations.locator('.campus-edit-actions').getByRole('button', { name: 'Save' }).click()
  // Saving is two calls (the address, then the location): wait for it to finish before reloading.
  await expect(page.locator('#edit-campus-name')).toBeHidden()
  await openApp(page, '/data/organizations')
  await openOrg()
  await expect(locations.getByText('Main Office')).toBeVisible()
  await expect(locations.getByText('Alexandria, VA')).toBeVisible()

  // ── Delete ──────────────────────────────────────────────────────────────────────────────
  await locations.getByRole('button', { name: 'Delete Main Office' }).click()
  await expect(locations.getByText(/No campuses defined/)).toBeVisible()
  await openApp(page, '/data/organizations')
  await openOrg()
  await expect(locations.getByText(/No campuses defined/)).toBeVisible()
  expect(await inPage<unknown[]>(page, `return ok(await forge.organizations.listLocations('${org.id}'))`)).toEqual([])

  expect(pageErrors).toEqual([])
})

test('kanban: status moves persist; interest, remove and add from the picker', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))
  await openApp(page)
  const ids = await inPage<{ lovelace: string; difference: string }>(
    page,
    `const a = ok(await forge.organizations.create({ name: 'Lovelace Labs', status: 'backlog' }))
     const b = ok(await forge.organizations.create({ name: 'Difference Engines' }))
     return { lovelace: a.id, difference: b.id }`,
  )

  // ── Drag: Backlog → Researching, persisted ──────────────────────────────────────────────
  await page.goto('/opportunities/organizations')
  await dragCardTo(page, 'Lovelace Labs', 'Researching')
  await expect.poll(() => statusOf(page, ids.lovelace)).toBe('researching')
  await openApp(page, '/opportunities/organizations')
  await expect(column(page, 'Researching').getByText('Lovelace Labs')).toBeVisible()

  // ── Drag to Targeting: the default interest level ───────────────────────────────────────
  await dragCardTo(page, 'Lovelace Labs', 'Targeting')
  await expect.poll(() => statusOf(page, ids.lovelace)).toBe('interested')

  // ── Change interest from the card's details ─────────────────────────────────────────────
  await page.locator('.kanban-card', { hasText: 'Lovelace Labs' }).click()
  const detail = page.getByRole('dialog', { name: 'Organization Details' })
  await detail.getByRole('button', { name: 'Exciting' }).click()
  await expect.poll(() => statusOf(page, ids.lovelace)).toBe('exciting')
  await expect(column(page, 'Targeting').getByText('Lovelace Labs')).toBeVisible()

  // ── Remove from pipeline: status null, off the board ────────────────────────────────────
  await detail.getByRole('button', { name: 'Remove from Pipeline' }).click()
  await expect.poll(() => statusOf(page, ids.lovelace)).toBeNull()
  await expect(page.locator('.kanban-card', { hasText: 'Lovelace Labs' })).toHaveCount(0)

  // ── The picker lists orgs off the board; add one to Backlog ─────────────────────────────
  await page.getByRole('button', { name: '+ Add Organization' }).click()
  const picker = page.getByRole('dialog', { name: 'Add Organization to Pipeline' })
  await expect(picker.getByText('Lovelace Labs')).toBeVisible()
  await picker.getByRole('button', { name: /Difference Engines/ }).click()
  await expect.poll(() => statusOf(page, ids.difference)).toBe('backlog')
  await expect(column(page, 'Backlog').getByText('Difference Engines')).toBeVisible()

  await expect(page.getByRole('alert').filter({ hasText: 'missing field' })).toHaveCount(0)
  expect(pageErrors).toEqual([])
})
