import { expect, test, type Page } from '@playwright/test'

/**
 * Organization aliases in the browser, with no server (resu-mx/forge#127): add, remove, and a
 * rejected duplicate that must show its error. These calls used to be raw `fetch('/api/…')`
 * and now go through the SDK.
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

test('organizations: add and remove an alias; a duplicate shows its error', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))
  await openApp(page)
  const org = await inPage<{ id: string }>(
    page,
    `const org = ok(await forge.organizations.create({ name: 'Western Governors University', org_type: 'education' }))
     ok(await forge.organizations.createAlias(org.id, 'WGU'))
     return org`,
  )

  await openApp(page, '/data/organizations')
  await page.getByText('Western Governors University').first().click()
  const pills = page.locator('.alias-pills .alias-pill')
  await expect(pills).toHaveText([/WGU/])

  // Add one in the UI; the API agrees.
  const input = page.getByPlaceholder('e.g. WGU, USAF')
  await input.fill('Western Governors')
  await input.press('Enter')
  await expect(pills).toHaveCount(2)
  const stored = await inPage<{ alias: string }[]>(page, `return ok(await forge.organizations.listAliases('${org.id}'))`)
  expect(stored.map((a) => a.alias).sort()).toEqual(['WGU', 'Western Governors'])

  // A duplicate is refused, and the person is told.
  await input.fill('WGU')
  await input.press('Enter')
  await expect(page.getByRole('alert').filter({ hasText: /already exists/i })).toBeVisible()
  await expect(pills).toHaveCount(2)

  // Remove one; it stays removed after a reload.
  await pills.filter({ hasText: 'WGU' }).getByRole('button', { name: '×' }).click()
  await expect(pills).toHaveCount(1)
  await openApp(page, '/data/organizations')
  await page.getByText('Western Governors University').first().click()
  await expect(pills).toHaveText([/Western Governors/])

  expect(pageErrors).toEqual([])
})
