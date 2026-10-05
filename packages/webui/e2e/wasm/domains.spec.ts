import { expect, test, type Page } from '@playwright/test'

/**
 * Data → Domains in the browser-first app (resu-mx/forge#37): usage counts, the in-use delete
 * guard (in the UI and in the API), deleting an unused domain, and editing.
 * Setup goes through `window.forge`; the person's steps go through the UI.
 */

type Forge = Record<string, any>

// Same helpers as core-loop.spec.ts.
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

const SECURITY_ENGINEER = 'a0000001-0000-4000-8000-000000000003'

function row(page: Page, name: string) {
  return page.locator('.domains-table tbody tr', {
    has: page.locator('td.domain-name', { hasText: new RegExp(`^${name}$`) }),
  })
}

test('domains: usage counts and the in-use delete guard', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))

  await openApp(page)
  await inPage(
    page,
    `
    ok(await forge.domains.create({ name: 'e2e_in_use', description: 'Named by one perspective' }))
    ok(await forge.domains.create({ name: 'e2e_unused' }))
    const b = ok(await forge.bullets.create({ content: 'Ran the e2e suite' }))
    ok(await forge.perspectives.create({ bullet_id: b.id, content: 'Ran the e2e suite in CI', domain: 'e2e_in_use' }))`,
  )

  await openApp(page, '/data/domains')

  // Seeded `security` is linked to two archetypes (migration 003).
  await expect(row(page, 'security').locator('td.count').nth(1)).toHaveText('2')
  await expect(row(page, 'security').getByRole('button', { name: 'Delete' })).toBeDisabled()
  await expect(row(page, 'e2e_in_use').locator('td.count').nth(0)).toHaveText('1')
  await expect(row(page, 'e2e_in_use').getByRole('button', { name: 'Delete' })).toBeDisabled()

  // The API refuses too, and nothing is torn down.
  const refused = await inPage<{ ok: boolean; error?: { code: string } }>(
    page,
    `
    const d = ok(await forge.domains.list({ limit: 200 })).find((x) => x.name === 'security')
    return await forge.domains.delete(d.id)`,
  )
  expect(refused.ok).toBe(false)
  expect(refused.error?.code).toBe('CONFLICT')
  const links = await inPage<{ name: string }[]>(
    page,
    `return ok(await forge.archetypes.listDomains('${SECURITY_ENGINEER}'))`,
  )
  expect(links.map((d) => d.name)).toContain('security')
  const still = await inPage<{ name: string }[]>(page, `return ok(await forge.domains.list({ limit: 200 }))`)
  expect(still.map((d) => d.name)).toContain('security')

  // An unused domain deletes through the UI.
  await row(page, 'e2e_unused').getByRole('button', { name: 'Delete' }).click()
  await page.getByRole('alertdialog', { name: 'Delete Domain' }).getByRole('button', { name: 'Delete' }).click()
  await expect(page.locator('.toast', { hasText: "Domain 'e2e_unused' deleted" })).toBeVisible()
  await expect(row(page, 'e2e_unused')).toHaveCount(0)

  expect(pageErrors).toEqual([])
})

test('domains: edit saves', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))

  await openApp(page)
  await inPage(page, `ok(await forge.domains.create({ name: 'e2e_edit', description: 'Before' }))`)
  await openApp(page, '/data/domains')

  await row(page, 'e2e_edit').getByRole('button', { name: 'Edit' }).click()
  const editing = page.locator('tr.editing-row')
  await editing.locator('input.field-input').nth(1).fill('Edited by e2e')
  await editing.getByRole('button', { name: 'Save' }).click()
  await expect(page.locator('.toast', { hasText: 'Domain updated' })).toBeVisible()
  await expect(page.locator('.toast', { hasText: 'Method Not Allowed' })).toHaveCount(0)

  await openApp(page, '/data/domains')
  await expect(row(page, 'e2e_edit').locator('td.domain-desc')).toHaveText('Edited by e2e')

  expect(pageErrors).toEqual([])
  // TODO: also clear the description and expect '--' after reload, once PATCH /domains/:id supports null.
})
