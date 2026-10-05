import { expect, test, type Page } from '@playwright/test'

/**
 * resu-mx/forge#35: summary keyword skills in the browser-first runtime.
 * Needs resu-mx/forge#30 (keyword routes) and `include=relations`.
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

/** The browser dispatcher's message for an unported route. */
const NOT_PORTED = 'not available in the browser runtime'

function trackErrors(page: Page): string[] {
  const errors: string[] = []
  page.on('pageerror', (e) => errors.push(e.message))
  return errors
}

async function seedSkill(page: Page, name: string): Promise<{ id: string }> {
  return inPage(page, `return ok(await forge.skills.create(${JSON.stringify({ name, category: 'tool' })}))`)
}

async function seedSummary(page: Page, title: string, skillIds: string[] = []): Promise<{ id: string }> {
  return inPage(
    page,
    `const s = ok(await forge.summaries.create(${JSON.stringify({ title })}))
     for (const id of ${JSON.stringify(skillIds)}) ok(await forge.summaries.addSkill(s.id, id))
     return s`,
  )
}

const modal = (page: Page) => page.getByRole('dialog')
const pills = (page: Page) => modal(page).locator('.keyword-pills .pill')
const toasts = (page: Page) => page.locator('.toast-container')

async function pickKeyword(page: Page, name: string) {
  const input = modal(page).getByPlaceholder('Search or create skill...')
  await input.click()
  await input.fill(name.split(' ')[0])
  await modal(page).locator('.skill-dropdown .dropdown-item', { hasText: name }).click()
  await expect(pills(page).filter({ hasText: name })).toHaveCount(1)
}

async function openSummary(page: Page, title: string) {
  await page.locator('.summary-row', { hasText: title }).first().click()
  await expect(modal(page).locator('#summary-title')).toHaveValue(title)
}

test('a new summary keeps the keywords picked before Create', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  await seedSkill(page, 'Jacquard Looms')
  await seedSkill(page, 'Babbage Mesh')

  await openApp(page, '/resumes/summaries')
  await page.getByRole('button', { name: '+ New Summary' }).click()
  await modal(page).locator('#summary-title').fill('Platform SRE summary')
  await pickKeyword(page, 'Jacquard Looms')
  await pickKeyword(page, 'Babbage Mesh')
  await modal(page).getByRole('button', { name: 'Create', exact: true }).click()
  await expect(toasts(page)).toContainText('Summary created')
  await expect(toasts(page)).not.toContainText('not saved')

  await openSummary(page, 'Platform SRE summary')
  await expect(pills(page)).toHaveCount(2)
  const names = await inPage<string[]>(
    page,
    `const s = ok(await forge.summaries.list({ limit: 50 })).find(x => x.title === 'Platform SRE summary')
     return ok(await forge.summaries.listSkills(s.id)).map(k => k.name)`,
  )
  expect([...names].sort()).toEqual(['Babbage Mesh', 'Jacquard Looms'])
  await expect(toasts(page)).not.toContainText(NOT_PORTED)
  expect(pageErrors).toEqual([])
})

test('an existing summary: add and remove a keyword; survives a reload', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  const jacquard = await seedSkill(page, 'Jacquard Looms')
  await seedSkill(page, 'Babbage Mesh')
  await seedSummary(page, 'Editable summary', [jacquard.id])

  await openApp(page, '/resumes/summaries')
  await openSummary(page, 'Editable summary')
  await expect(pills(page)).toHaveText([/Jacquard Looms/])
  await pickKeyword(page, 'Babbage Mesh')
  await modal(page).getByRole('button', { name: 'Remove Jacquard Looms' }).click()
  await expect(pills(page)).toHaveText([/Babbage Mesh/])
  await expect(toasts(page)).not.toContainText(NOT_PORTED)

  await openApp(page, '/resumes/summaries')
  await openSummary(page, 'Editable summary')
  await expect(pills(page)).toHaveText([/Babbage Mesh/])
  expect(pageErrors).toEqual([])
})

test('the page shows keyword pills, groups by keyword and filters by keyword', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  const jacquard = await seedSkill(page, 'Jacquard Looms')
  const babbage = await seedSkill(page, 'Babbage Mesh')
  await seedSummary(page, 'Keyworded summary', [jacquard.id, babbage.id])
  await seedSummary(page, 'Plain summary')

  await openApp(page, '/resumes/summaries')
  const row = page.locator('.summary-row', { hasText: 'Keyworded summary' })
  await expect(row.locator('.kw-pill')).toHaveText(['Babbage Mesh', 'Jacquard Looms'])

  await page.getByLabel('Group By').selectOption('keyword')
  await expect(page.locator('.group-header .group-label')).toHaveText(['(No keywords)', 'Babbage Mesh', 'Jacquard Looms'])
  await expect(page.locator('.group-header .group-count')).toHaveText(['1', '1', '1'])

  await page.getByLabel('Group By').selectOption('none')
  // The label's accessible name includes its options, so match the control by its caption.
  await page
    .locator('label.control', { has: page.locator('.control-label', { hasText: /^Keyword$/ }) })
    .locator('select')
    .selectOption({ label: 'Jacquard Looms' })
  await expect(page.locator('.summary-row')).toHaveCount(1)
  await expect(page.locator('.summary-row')).toContainText('Keyworded summary')
  await expect(toasts(page)).not.toContainText("Couldn't load keywords")
  expect(pageErrors).toEqual([])
})

test('a keyword that fails to save during create is reported by name', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  await seedSkill(page, 'Jacquard Looms')
  const babbage = await seedSkill(page, 'Babbage Mesh')

  await openApp(page, '/resumes/summaries')
  await inPage(
    page,
    `const add = forge.summaries.addSkill.bind(forge.summaries)
     forge.summaries.addSkill = (id, skillId) => skillId === '${babbage.id}'
       ? Promise.resolve({ ok: false, error: { code: 'INTERNAL_ERROR', message: 'injected' } })
       : add(id, skillId)`,
  )
  await page.getByRole('button', { name: '+ New Summary' }).click()
  await modal(page).locator('#summary-title').fill('Half-saved summary')
  await pickKeyword(page, 'Jacquard Looms')
  await pickKeyword(page, 'Babbage Mesh')
  await modal(page).getByRole('button', { name: 'Create', exact: true }).click()
  await expect(toasts(page)).toContainText('Summary created, but 1 keyword was not saved: Babbage Mesh')

  await openSummary(page, 'Half-saved summary')
  await expect(pills(page)).toHaveText([/Jacquard Looms/])
  expect(pageErrors).toEqual([])
})
