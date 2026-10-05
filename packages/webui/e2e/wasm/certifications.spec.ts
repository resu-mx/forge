import { expect, test, type Page } from '@playwright/test'

/**
 * Qualifications → Certifications in the browser-first app (resu-mx/forge#38): linked skills
 * show, linking and creating skills keeps the page alive, unlinking sticks, and every
 * certification loads in short-name order.
 */

type Forge = Record<string, any>

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

const card = (page: Page, shortName: string) =>
  page.locator('.cert-card', { has: page.locator('.card-name', { hasText: new RegExp(`^${shortName}$`) }) })

test('certifications: skills show, link, create-and-link, unlink, and survive reload', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))

  await openApp(page)
  await inPage(
    page,
    `
    const cert = ok(await forge.certifications.create({ short_name: 'CKA', long_name: 'Certified Kubernetes Administrator' }))
    const helm = ok(await forge.skills.create({ name: 'E2E Helm' }))
    ok(await forge.skills.create({ name: 'E2E Kubernetes' }))
    ok(await forge.certifications.addSkill(cert.id, helm.id))`,
  )

  await openApp(page, '/qualifications/certifications')
  await card(page, 'CKA').click()
  const pills = page.locator('.skill-pill')
  await expect(pills.filter({ hasText: 'E2E Helm' })).toBeVisible()
  await expect(page.getByText('No skills linked yet.')).toHaveCount(0)

  const picker = page.getByPlaceholder('Search or create skill...')
  await picker.fill('E2E Kube')
  await page.locator('.skill-dropdown .dropdown-item', { hasText: 'E2E Kubernetes' }).click()
  await expect(pills.filter({ hasText: 'E2E Kubernetes' })).toBeVisible()

  await picker.fill('E2E Brand New Skill')
  await page.locator('.skill-dropdown .dropdown-item.create-item').click()
  await expect(pills.filter({ hasText: 'E2E Brand New Skill' })).toBeVisible()

  // Still usable: the list filter runs over every row without throwing.
  await page.getByPlaceholder('Search certifications...').fill('CKA')
  await expect(card(page, 'CKA')).toBeVisible()

  await openApp(page, '/qualifications/certifications')
  await card(page, 'CKA').click()
  await expect(pills).toHaveCount(3)

  await page.getByRole('button', { name: 'Remove E2E Helm' }).click()
  await expect(pills.filter({ hasText: 'E2E Helm' })).toHaveCount(0)
  await openApp(page, '/qualifications/certifications')
  await card(page, 'CKA').click()
  await expect(pills).toHaveCount(2)
  await expect(pills.filter({ hasText: 'E2E Helm' })).toHaveCount(0)

  expect(pageErrors).toEqual([])
})

test('certifications: more than 50 load, sorted by short name', async ({ page }) => {
  const pageErrors: string[] = []
  page.on('pageerror', (e) => pageErrors.push(e.message))

  await openApp(page)
  // Scrambled creation order: neither created_at ASC nor DESC is the expected order.
  await inPage(
    page,
    `
    for (let k = 0; k < 55; k++) {
      const n = String((k * 23) % 55 + 1).padStart(2, '0')
      ok(await forge.certifications.create({ short_name: 'E2E-' + n, long_name: 'E2E certification ' + n }))
    }`,
  )

  await openApp(page, '/qualifications/certifications')
  const names = page.locator('.cert-card .card-name')
  await expect(names).toHaveCount(55)
  const expected = Array.from({ length: 55 }, (_, i) => `E2E-${String(i + 1).padStart(2, '0')}`)
  expect(await names.allTextContents()).toEqual(expected)

  // The contract: every row as {data}, no pagination envelope (raw fetch goes to the runtime).
  const body = await page.evaluate(async () => (await fetch('/api/certifications')).json())
  expect(body.data).toHaveLength(55)
  expect(body).not.toHaveProperty('pagination')

  expect(pageErrors).toEqual([])
})
