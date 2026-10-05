import { expect, test, type Page } from '@playwright/test'

/**
 * resu-mx/forge#33: the JD editor's sub-resources in the browser-first runtime.
 * Seeding goes through `window.forge` (the agent). The checks go through the UI (the person).
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

/** The browser dispatcher's message for an unported route (crates/forge-wasm/src/dispatch.rs). */
const NOT_PORTED = 'not available in the browser runtime'

const JD_TEXT =
  'Run Kubernetes and Terraform for a payments platform. Own SLOs, on-call and incident review.'

async function seedJd(page: Page, title: string): Promise<{ id: string }> {
  return inPage(
    page,
    `return ok(await forge.jobDescriptions.create(${JSON.stringify({ title, raw_text: JD_TEXT })}))`,
  )
}

/** Load the JD page and select a JD by its card (the page ignores ?selected=). */
async function openJd(page: Page, title: string) {
  await openApp(page, '/opportunities/job-descriptions')
  await page.locator('.card-list .jd-card', { hasText: title }).first().click()
  await expect(page.locator('#jd-title')).toHaveValue(title)
}

const chips = (page: Page) => page.locator('.skill-picker .skill-tags .skill-pill')
const linked = (page: Page) => page.locator('.jd-linked-resumes')

function trackErrors(page: Page): string[] {
  const errors: string[] = []
  page.on('pageerror', (e) => errors.push(e.message))
  return errors
}

test('required skills: add an existing skill, create one, remove one; survives a reload', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  await inPage(page, `ok(await forge.skills.create({ name: 'Jacquard Looms', category: 'tool' }))`)
  const jd = await seedJd(page, 'Analytical Engine SRE')

  await openJd(page, 'Analytical Engine SRE')
  const search = page.getByPlaceholder('Search or add skill...')
  await search.click() // the dropdown opens on focus
  await search.fill('Jacquard')
  await page.locator('.skill-picker .dropdown-item', { hasText: 'Jacquard Looms' }).click()
  await expect(chips(page).filter({ hasText: 'Jacquard Looms' })).toHaveCount(1)

  await expect(search).toBeEnabled()
  await search.click()
  await search.fill('Babbage Mesh')
  await page.getByRole('button', { name: 'Create "Babbage Mesh"' }).click()
  await expect(chips(page)).toHaveCount(2)

  await page.getByRole('button', { name: 'Remove Jacquard Looms' }).click()
  await expect(chips(page)).toHaveCount(1)
  await expect(page.locator('.toast-container')).not.toContainText(NOT_PORTED)

  await openJd(page, 'Analytical Engine SRE')
  await expect(chips(page)).toHaveText([/Babbage Mesh/])
  const names = await inPage<string[]>(
    page,
    `return ok(await forge.jobDescriptions.listSkills('${jd.id}')).map(s => s.name)`,
  )
  expect(names).toEqual(['Babbage Mesh'])
  await expect(page.locator('.toast-container')).not.toContainText(NOT_PORTED)
  expect(pageErrors).toEqual([])
})

test('skill radar renders for a JD with skills', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  const jd = await seedJd(page, 'Radar JD')
  await inPage(page, `ok(await forge.jobDescriptions.addSkill('${jd.id}', { name: 'Babbage Mesh' }))`)

  await openJd(page, 'Radar JD')
  await expect(page.getByRole('heading', { name: 'Skill Alignment' })).toBeVisible()
  await expect(page.getByText('Failed to load skill data')).toHaveCount(0)
  await expect(page.locator('.toast-container')).not.toContainText(NOT_PORTED)
  expect(pageErrors).toEqual([])
})

test('linked resumes: link and unlink from the JD; survives a reload', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  await seedJd(page, 'Linkable JD')
  await inPage(
    page,
    `ok(await forge.resumes.create({
      name: 'Platform SRE', target_role: 'SRE', target_employer: 'Acme', archetype: 'infrastructure' }))`,
  )

  await openJd(page, 'Linkable JD')
  await linked(page).getByRole('button', { name: '+ Link Resume' }).click()
  await page.locator('.picker-item', { hasText: 'Platform SRE' }).click()
  await expect(linked(page).locator('.linked-card-name')).toHaveText(['Platform SRE'])
  await expect(page.locator('.toast-container')).not.toContainText(NOT_PORTED)

  await openJd(page, 'Linkable JD')
  await expect(linked(page).locator('.linked-card-name')).toHaveText(['Platform SRE'])

  await linked(page).getByRole('button', { name: 'Unlink Platform SRE' }).click()
  await expect(linked(page).getByText('No resumes linked to this job description.')).toBeVisible()
  await expect(page.locator('.toast-container')).not.toContainText(NOT_PORTED)
  await openJd(page, 'Linkable JD')
  await expect(linked(page).getByText('No resumes linked to this job description.')).toBeVisible()
  expect(pageErrors).toEqual([])
})

test("overlay lists the JD's skills when opened from a resume", async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  const jd = await seedJd(page, 'Overlay JD')
  const resume = await inPage<{ id: string }>(
    page,
    `return ok(await forge.resumes.create({
      name: 'Overlay resume', target_role: 'SRE', target_employer: 'Acme', archetype: 'infrastructure' }))`,
  )
  await inPage(
    page,
    `ok(await forge.jobDescriptions.addSkill('${jd.id}', { name: 'Babbage Mesh' }))
     ok(await forge.jobDescriptions.linkResume('${jd.id}', '${resume.id}'))`,
  )

  await openApp(page, '/resumes')
  await page.getByText('Overlay resume').first().click()
  await page.locator('.resume-linked-jds').getByRole('button', { name: 'Overlay JD', exact: true }).click()
  const overlay = page.getByRole('dialog').filter({ hasText: 'Overlay JD' })
  await expect(overlay.locator('.jd-overlay-skill-tag', { hasText: 'Babbage Mesh' })).toBeVisible()
  await expect(overlay.getByText('Failed to load skills')).toHaveCount(0)
  await expect(page.locator('.toast-container')).not.toContainText(NOT_PORTED)
  expect(pageErrors).toEqual([])
})

test('a failed skills load is reported, not shown as empty', async ({ page }) => {
  const pageErrors = trackErrors(page)
  await openApp(page)
  await seedJd(page, 'Broken skills JD')

  await openApp(page, '/opportunities/job-descriptions')
  // Same instance the UI imports (packages/webui/src/lib/sdk.ts), so this reaches JDEditor.
  await inPage(
    page,
    `forge.jobDescriptions.listSkills = async () =>
      ({ ok: false, error: { code: 'INTERNAL_ERROR', message: 'injected' } })`,
  )
  await page.locator('.card-list .jd-card', { hasText: 'Broken skills JD' }).first().click()
  await expect(page.getByRole('alert').filter({ hasText: 'Failed to load required skills' })).toBeVisible()
  await expect(page.locator('.toast-container')).not.toContainText(NOT_PORTED)
  expect(pageErrors).toEqual([])
})
